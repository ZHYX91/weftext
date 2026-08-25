"use client";

import { createElement, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import BackupSurface, { type BackupDirectoryKind } from "./backup-surface";
import ExportSurface from "./export-surface";
import IntakeSurface from "./intake-surface";
import {
  WORKSPACE_ACTION_REGISTRY,
  conflictingDirtyNodeIds,
  freezeCoreReviewedDraftScope,
  resolveWorkspaceActionTarget,
  type FrozenReviewedDraftScope,
  type FrozenWorkspaceActionTarget,
  type WorkspaceActionInvocation,
} from "./node-actions";
import QuerySurface from "./query-surface";
import SourceEditor from "./source-editor";
import TaskSurface from "./task-surface";
import WriteEditor from "./write-editor";
import { isDocumentModel, isDocumentProfile, isDocumentViewModel, isNodeMetadataProjection, type DocumentBlock, type DocumentListItem, type DocumentListModel, type DocumentModel, type DocumentProfile, type DocumentProfileId, type DocumentTableModel, type DocumentTableRow, type DocumentViewModel, type NodeMetadataProjection, type WorkspaceDocumentFormat } from "./document-contract";
import { DEMO_WORKSPACE, type DemoAnnotation, type DemoDocument, type DemoWorkspace } from "./demo-workspace";
import {
  INITIAL_NAVIGATION_WINDOW,
  directContents,
  incrementalWindow,
  interactionMeasurement,
  locationBreadcrumbs,
  navigationProjection,
  readExplorerState,
  validatedBrowseLocator,
  visibleHierarchy,
  writeExplorerState,
  type ExplorerActivity,
  type ExplorerMode,
  type NavigationPerformanceSample,
  type SharedNavigationProjection,
} from "./shared-navigation";

type ViewMode = "write" | "source" | "read";
type FrozenNodeActionTarget = Extract<FrozenWorkspaceActionTarget, { kind: "node" }>;
type WorkspaceTargetResolution = "focused_pane" | "explicit_row" | "caller_explicit";
const REFERENCE_RECORD_READ_ONLY_NOTICE = "历史参考文献记录仅作为只读转换证据；创建、字段编辑和引用键重命名没有写入口。cite、nocite 与 bibliography occurrence 仍可编辑。";
const ANNOTATION_ACTOR_STORAGE_KEY = "weftext.annotation.actor.v1";
type ThemeMode = "system" | "light" | "dark" | "contrast";
type IconPlacement = "before" | "after" | "hidden";
type Dialog = "search" | "query" | "intake" | "export" | "backup" | "node_actions" | "new" | "rename" | "move" | "copy" | "node_trash" | "chrono" | "property" | "icon" | "resource" | "resource_trash" | "annotation" | "citation" | "conflict" | "recovery" | "trash_item" | "trash_restore" | "trash_permanent" | "core" | "save" | "structure" | null;

type ResolvedNodeIcon = { kind: "emoji" | "built_in"; value: string; glyph: string };
type BuiltInNodeIcon = { id: string; label: string; glyph: string };
type DocumentProperty = {
  name: string;
  value: string;
  kind: "descriptive" | "custom";
  range: { start: number; end: number };
  nameRange: { start: number; end: number };
  valueRange: { start: number; end: number };
};
type DocumentPropertyDiagnostic = {
  code: string;
  message: string;
  range: { start: number; end: number };
  name: string | null;
};
type DocumentPropertyAnalysis = {
  properties: DocumentProperty[];
  diagnostics: DocumentPropertyDiagnostic[];
  headerRange: { start: number; end: number };
};

export type { DocumentBlock } from "./document-contract";
export { extendTableAtCoreBlock, formattedBlockReplacement, headingBlockReplacement } from "./demo-workspace";

type LiveDocument = {
  nodeId: string;
  name: string;
  revision: string;
  length: number;
  source: string;
  profile?: DocumentProfile;
  model: DocumentModel;
  view?: DocumentViewModel;
  metadata?: NodeMetadataProjection;
  properties?: DocumentPropertyAnalysis;
  recoveryDraft?: RecoveryDraft | null;
  recoveryIssue?: string;
};

type RecoveryDraft = {
  nodeId: string;
  name: string;
  baseRevision: string;
  currentRevision: string;
  profile?: "ascii_doc_v1";
  stale: boolean;
  length: number;
  updatedAtUnixMs: number;
  source?: string;
};

type DraftRecovery = {
  drafts: RecoveryDraft[];
  issues: string[];
};

type DesktopDiagnostics = {
  safeMode: boolean;
  workspaceValid: boolean;
  nodeCount: number;
  inventoryIssueCodes: string[];
  recoveryDraftCount: number;
  recoveryIssueCount: number;
  index: string;
  pathsRedacted: boolean;
  documentBodiesIncluded: boolean;
};

type SavePlan = {
  baseRevision: string;
  nextRevision: string;
  oldLength: number;
  newLength: number;
  changed: boolean;
};

type DocumentFormatCommand =
  | { kind: "bold" | "emphasis" | "inline_code" | "link" | "paragraph" | "list" | "quote_increase" | "quote_decrease" | "code_block" | "table_insert" | "table_add_row" | "table_add_column" }
  | { kind: "heading"; level: number }
  | { kind: "image"; target: string; alt: string };

type DocumentFormatPlan = {
  profile: DocumentProfileId;
  source: string;
  selectionStart: number;
  selectionEnd: number;
  changed: boolean;
};

type ResourcePlan = {
  planId: string;
  nodeId: string;
  name: string;
  byteLength: number;
  baseRevision: string;
};

type AnnotationPlan = { planId: string; baseRevision: string; action: "annotation" };
type AnnotationActionName =
  | "create"
  | "reply"
  | "edit_message"
  | "set_appearance"
  | "set_labels"
  | "resolve"
  | "reopen"
  | "reanchor"
  | "accept_suggestion"
  | "reject_suggestion";
type AnnotationKind = "comment" | "mark" | "suggestion_insert" | "suggestion_delete";
type LiveAnnotationStore = {
  version: number;
  document_id: string;
  annotations: Array<{
    id: string;
    kind: AnnotationKind;
    target: { kind: "text_range" | "insertion_point" | "block" | "document" | "resource_region"; exact?: string; heading_path?: string[] };
    appearance: { mark: string; theme: string } | null;
    suggested_source?: string;
    labels: string[];
    thread: Array<{
      id: string;
      author_id: string;
      author_name: string;
      body: { format: "weftext.asciidoc.inline.v1"; source: string };
      created_at: string;
      updated_at: string;
    }>;
    state: "open" | "resolved" | "orphaned";
    resolution?: "resolved" | "accepted" | "rejected";
    created_at: string;
    updated_at: string;
  }>;
};

type TreeNode = {
  id: string;
  name: string;
  depth: number;
  open?: boolean;
  kind?: "chrono" | "note" | "folder";
  parentId?: string | null;
  path?: string;
  icon?: ResolvedNodeIcon | null;
  displayIcon?: WorkspaceItemIcon | null;
};

type TrashOriginResolution = "active" | "in_trash" | "missing" | "unknown" | "reconciliation_required";

type TrashManifestCommon = {
  schema: "weftext.trash-item/v1";
  trashItemId: string;
  operationId: string;
  trashedAt: string;
  originStatus: "known" | "unknown";
  originalName: string;
};

type TrashItemManifest = TrashManifestCommon & ({
  kind: "node";
  nodeId: string;
  originalParentNodeId: string | null;
  ancestorNodeIds: string[];
  payloadSha256: string;
  payloadByteLength: number;
  payloadEntryCount: number;
} | {
  kind: "resource";
  originalOwnerNodeId: string | null;
  sha256: string;
  byteLength: number;
});

type TrashItemSummary = {
  manifest: TrashItemManifest;
  containedNodeIds: string[];
  restore: {
    originResolution: TrashOriginResolution;
    originalAvailable: boolean;
    withAncestorsAvailable: boolean;
    requiredAncestorItemIds: string[];
    blockedReason?: string | null;
  };
};

type WorkspaceItemIcon = {
  kind: "explicit_node" | "default_node" | "folder" | "markdown_file" | "file" | "workspace_root" | "trash";
  explicit?: ResolvedNodeIcon;
};

type WorkspaceContentItem = {
  kind: "managed_node" | "unmanaged_directory" | "unmanaged_markdown" | "resource";
  name: string;
  path: string;
  parentPath: string | null;
  nodeId: string | null;
  ownerNodeId: string | null;
  displayIcon: WorkspaceItemIcon;
};

type LiveWorkspace = {
  rootNodeId: string;
  revision: string;
  documentFormat?: WorkspaceDocumentFormat;
  presentation: { adjacentHeadingBody: "separate" | "run_in" };
  nodes: Array<{ id: string; name: string; parentId: string | null; path: string; icon: ResolvedNodeIcon | null; displayIcon?: WorkspaceItemIcon }>;
  trashItems?: TrashItemSummary[];
  trashReconciliation?: { required: boolean; issueCount: number };
  trashLegacyMigrationRequired?: boolean;
  content?: WorkspaceContentItem[];
  navigation?: SharedNavigationProjection;
  links: {
    outgoing: Array<{ sourceNodeId: string; targetNodeIds: string[]; authoredLocator: string }>;
    backlinks: Array<{ sourceNodeId: string; targetNodeId: string }>;
    potentialMentions: Array<{ sourceNodeId: string; matchedText: string; matchedScalarLength: number; targetNodeIds: string[]; primary: boolean }>;
  };
  iconCatalog: BuiltInNodeIcon[];
};

type StructuralPlan = {
  planId: string;
  action: string;
  baseRevision: string;
  pathChanges: Array<{ sourceNodeId?: string | null; oldPath: string | null; newPath: string; nodeId: string }>;
  documentChanges: Array<{ path: string; editCount: number }>;
  generatedNodeIds: string[];
  scopeSummary: {
    rootNode: { nodeId: string; displayName: string };
    descendantNodeCount: number;
    resourceCount: number;
    annotationSidecarCount: number;
    byteTotal: number;
    affectedDocumentNodeIds: string[];
    rewrittenDocumentNodeIds: string[];
    identityPolicy: "preserve" | "rekey";
    trashItemCount: number;
    operationId: string | null;
  } | null;
  identityMap: Array<{ sourceNodeId: string; destinationNodeId: string }>;
  capturedTarget: null | { kind: "node"; nodeId: string; resolvedBy: string }
    | { kind: "trash_item"; trashItemId: string; resolvedBy: string }
    | { kind: "owned_resource"; ownerNodeId: string; name: string; resolvedBy: string };
  targetNodeIds: string[];
  draftSensitiveNodeIds: string[];
  trashItemChanges?: Array<{
    disposition: "stored" | "restored" | "permanently_deleted" | "migrated";
    manifest: TrashItemManifest;
    destinationNodeId: string | null;
    destinationName: string | null;
  }>;
};

function targetResolution(target: FrozenWorkspaceActionTarget): WorkspaceTargetResolution {
  if (target.kind === "node") return target.source === "editor_command" ? "focused_pane" : "explicit_row";
  return "explicit_row";
}

type StructuralContext = {
  kind: "node_metadata";
  nodeId: string;
  revision: string;
  summary: string;
} | {
  kind: "trash";
  purpose: "node_trash" | "resource_trash" | "restore" | "permanent_delete" | "migration";
  itemIds: string[];
  target?: FrozenWorkspaceActionTarget;
} | {
  kind: "node_action";
  target: FrozenNodeActionTarget;
  label: string;
} | null;

function trashPayloadSha256(manifest: TrashItemManifest) {
  return manifest.kind === "node" ? manifest.payloadSha256 : manifest.sha256;
}

function trashPayloadByteLength(manifest: TrashItemManifest) {
  return manifest.kind === "node" ? manifest.payloadByteLength : manifest.byteLength;
}

function trashDispositionLabel(disposition: NonNullable<StructuralPlan["trashItemChanges"]>[number]["disposition"]) {
  return {
    stored: "创建废纸篓条目",
    restored: "恢复废纸篓条目",
    permanently_deleted: "永久删除废纸篓条目",
    migrated: "迁移旧废纸篓条目",
  }[disposition];
}

function structuralPreviewLabel(context: StructuralContext) {
  if (context?.kind === "node_action") return context.label;
  if (context?.kind === "node_metadata") return "修改节点元数据";
  if (context?.kind === "trash") {
    return {
      node_trash: "将整个节点分支移入废纸篓",
      resource_trash: "将节点资源移入废纸篓",
      restore: "恢复废纸篓条目",
      permanent_delete: "永久删除废纸篓条目",
      migration: "迁移旧废纸篓条目",
    }[context.purpose];
  }
  return "修改工作区设置";
}

type SearchResult = {
  id: string;
  name: string;
  path: string;
  snippet: string;
  icon?: ResolvedNodeIcon | null;
};

type DocumentHeading = {
  level: number;
  text: string;
  start: number;
  line: number;
};

type DocumentMatch = {
  start: number;
  end: number;
};

type DeviceEditorState = {
  selectionStart: number;
  selectionEnd: number;
  scrollTop: number;
  view: ViewMode;
};

type NavigationTab = { id: string; nodeId: string };
type SplitSession = DeviceEditorState & { nodeId: string };
type WorkspaceNavigation = {
  version: 1;
  tabs: NavigationTab[];
  activeTabId: string;
  back: string[];
  forward: string[];
  recent: string[];
  bookmarks: string[];
  split: SplitSession | null;
};

type DesktopOpenPayload = {
  opened: boolean;
  workspace?: LiveWorkspace;
  document?: LiveDocument;
  draftRecovery?: DraftRecovery;
  safeMode?: boolean;
  restoreError?: string;
  searchIndexWarning?: DerivedIndexWarning | null;
};

type DerivedIndexWarning = {
  code: string;
  message: string;
  rebuildRequired: boolean;
  workspaceOpenSucceeded?: boolean;
  authoritativeCommitSucceeded?: boolean;
};

type CitationRange = { start: number; end: number };
type CitationReferenceHit = {
  nodeId: string;
  key: string;
  itemType: string;
  title: string;
  contributors: string[];
  identifiers: Record<string, string>;
  selectable: boolean;
  matchedFields: string[];
};

type CitationRichRun = {
  text: string;
  style: { italic: boolean; smallCaps: boolean; weight: "normal" | "bold" | "light"; underline: boolean; verticalAlign: "none" | "baseline" | "superscript" | "subscript" };
  link: string | null;
  referenceNodeId: string | null;
};

type CitationRichText = { runs: CitationRichRun[] };
type CitationReferenceValue =
  | { kind: "text"; value: string }
  | { kind: "names"; value: Array<Record<string, string | null>> }
  | { kind: "date"; value: { dateParts?: number[][] | null; literal?: string | null; season?: string | null; circa?: boolean | null } };
type CitationIntentItem = { referenceNodeId: string; key: string; title: string; label: string | null; locator: string | null; prefix: string | null; suffix: string | null };
type CitationResolvedReference = { nodeId: string; citationData: { key: string; itemType: string; title: string; fields: Record<string, CitationReferenceValue> } };
type CitationResolvedCluster = { form: "parenthetical" | "narrative"; range: CitationRange; items: Array<{ label: string; locator: string | null; prefix: string | null; suffix: string | null; reference: CitationResolvedReference }> };
type CitationComponentPresentation = {
  componentNodeId: string;
  revision: string;
  citations: Array<{ sourceRange: CitationRange; form: "parenthetical" | "narrative"; noteNumber: number | null; referenceNodeIds: string[]; content: CitationRichText }>;
  bibliography: null | {
    sourceRange: CitationRange;
    hangingIndent: boolean;
    secondFieldAlign: "margin" | "flush" | null;
    lineSpacing: number;
    entrySpacing: number;
    entries: Array<{ referenceNodeId: string; firstField: CitationRichText | null; content: CitationRichText }>;
  };
};

type CitationDraftPayload = {
  authoring: {
    reference: {
      citationData: null | { key: string; itemType: string; title: string; fields: Record<string, CitationReferenceValue> };
      diagnostics: Array<{ code: string; message: string; path: string | null; range: CitationRange | null }>;
    };
    citations: { clusters?: Array<{ form: "parenthetical" | "narrative"; range: CitationRange }>; diagnostics: Array<{ code: string; message: string; range: CitationRange }> };
  };
  analysis: { clusters?: CitationResolvedCluster[]; nocites?: Array<{ range: CitationRange; references: CitationResolvedReference[] }>; bibliography?: { range: CitationRange; inclusion: "cited" | "all" } | null; diagnostics: Array<{ code: string; message: string; range: CitationRange | null }> };
  presentation: null | { providerId: string; providerVersion: string; profile: { styleId: string; locale: string }; components: CitationComponentPresentation[] };
  presentationFailure: null | { diagnostics: Array<{ code: string; message: string; sourceRange: CitationRange | null }> };
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isLiveAnnotationStore(value: unknown): value is LiveAnnotationStore {
  if (!isRecord(value) || value.version !== 3 || typeof value.document_id !== "string" || !Array.isArray(value.annotations)) return false;
  return value.annotations.every((annotation) => {
    if (!isRecord(annotation) || typeof annotation.id !== "string" || !isRecord(annotation.target) || !Array.isArray(annotation.labels) || !Array.isArray(annotation.thread)) return false;
    if (!["comment", "mark", "suggestion_insert", "suggestion_delete"].includes(String(annotation.kind))) return false;
    if (!["open", "resolved", "orphaned"].includes(String(annotation.state))) return false;
    if (annotation.appearance !== null && annotation.appearance !== undefined && (!isRecord(annotation.appearance) || typeof annotation.appearance.mark !== "string" || typeof annotation.appearance.theme !== "string")) return false;
    return annotation.thread.every((message) => isRecord(message)
      && typeof message.id === "string"
      && typeof message.author_id === "string"
      && typeof message.author_name === "string"
      && isRecord(message.body)
      && message.body.format === "weftext.asciidoc.inline.v1"
      && typeof message.body.source === "string");
  });
}

function isCitationDraftPayload(value: unknown): value is CitationDraftPayload {
  if (!isRecord(value) || !isRecord(value.authoring) || !isRecord(value.analysis)) return false;
  const authoring = value.authoring;
  if (!isRecord(authoring.reference) || !isRecord(authoring.citations)) return false;
  if (!Array.isArray(authoring.reference.diagnostics) || !Array.isArray(authoring.citations.diagnostics) || !Array.isArray(value.analysis.diagnostics)) return false;
  if (value.presentation !== null && !isRecord(value.presentation)) return false;
  if (value.presentationFailure !== null && !isRecord(value.presentationFailure)) return false;
  return true;
}

type CitationDiagnostic = { code: string; message: string; range: CitationRange | null };

type DisplayAnnotation = DemoAnnotation & {
  kind?: AnnotationKind;
  state?: "open" | "resolved" | "orphaned";
  resolution?: "resolved" | "accepted" | "rejected";
  targetKind?: LiveAnnotationStore["annotations"][number]["target"]["kind"];
  suggestedSource?: string;
  mark?: string;
  color?: string;
  labels?: string[];
  messages?: Array<{ id: string; authorId: string; authorName: string; body: string; time: string }>;
};

type DesktopBridge = {
  request(path: string, body?: unknown): Promise<unknown>;
  restoreWorkspace(): Promise<unknown>;
  chooseWorkspace(): Promise<unknown | null>;
  chooseMarkdownExportDestination(suggestedName: string): Promise<{ capability: string; displayPath: string } | null>;
  chooseTaskImportReceiptDestination(suggestedName: string): Promise<{ capability: string; displayPath: string } | null>;
  chooseBackupDirectory?(kind: BackupDirectoryKind): Promise<{ capability: string; kind: BackupDirectoryKind; displayPath: string } | null>;
};

const emptyDraftRecovery: DraftRecovery = { drafts: [], issues: [] };
const editorStateStorageKey = "weftext.editor-state.v1";
const iconPreferenceStorageKey = "weftext.icon-preferences.v1";
const navigationStorageKey = "weftext.navigation.v1";
const emojiIconCatalog = ["😀", "😺", "📘", "📌", "⭐", "💡", "🗓️", "✅", "🧭", "🧪", "📝", "🔖", "🌱", "🚀", "❤️", "⚠️"];

declare global {
  interface Window {
    weftextDesktop?: DesktopBridge;
  }
}

function readDeviceEditorStates() {
  if (typeof window === "undefined") return {} as Record<string, DeviceEditorState>;
  try {
    const parsed = JSON.parse(window.localStorage.getItem(editorStateStorageKey) ?? "{}") as Record<string, DeviceEditorState>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function readAnnotationActor() {
  const fallback = { id: globalThis.crypto?.randomUUID?.() ?? "00000000-0000-4000-8000-000000000002", name: "Local author" };
  if (typeof window === "undefined") return fallback;
  try {
    const value = JSON.parse(window.localStorage.getItem(ANNOTATION_ACTOR_STORAGE_KEY) ?? "null") as { id?: unknown; name?: unknown } | null;
    if (value && typeof value.id === "string" && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value.id) && typeof value.name === "string" && value.name.trim()) {
      return { id: value.id.toLowerCase(), name: value.name.trim() };
    }
    window.localStorage.setItem(ANNOTATION_ACTOR_STORAGE_KEY, JSON.stringify(fallback));
  } catch {
    // Device-local actor identity is unavailable in private storage modes.
  }
  return fallback;
}

function writeDeviceEditorStates(states: Record<string, DeviceEditorState>) {
  try {
    window.localStorage.setItem(editorStateStorageKey, JSON.stringify(states));
  } catch {
    // Editor continuity is best-effort device state; document drafts remain authoritative.
  }
}

function readIconPreferences() {
  if (typeof window === "undefined") return { placement: "before" as IconPlacement, showInTitle: true };
  try {
    const parsed = JSON.parse(window.localStorage.getItem(iconPreferenceStorageKey) ?? "{}") as { placement?: string; showInTitle?: boolean };
    const placement = parsed.placement === "after" || parsed.placement === "hidden" ? parsed.placement : "before";
    return { placement: placement as IconPlacement, showInTitle: parsed.showInTitle !== false };
  } catch {
    return { placement: "before" as IconPlacement, showInTitle: true };
  }
}

function defaultNavigation(nodeId: string): WorkspaceNavigation {
  return {
    version: 1,
    tabs: [{ id: `tab-${nodeId}`, nodeId }],
    activeTabId: `tab-${nodeId}`,
    back: [],
    forward: [],
    recent: [nodeId],
    bookmarks: [],
    split: null,
  };
}

function readWorkspaceNavigation(workspaceId: string, fallbackNodeId: string, validNodeIds: Set<string>) {
  if (typeof window === "undefined") return defaultNavigation(fallbackNodeId);
  try {
    const all = JSON.parse(window.localStorage.getItem(navigationStorageKey) ?? "{}") as Record<string, WorkspaceNavigation>;
    const stored = all[workspaceId];
    if (!stored || stored.version !== 1) return defaultNavigation(fallbackNodeId);
    const tabs = stored.tabs.filter((tab) => validNodeIds.has(tab.nodeId));
    if (!tabs.length) tabs.push({ id: `tab-${fallbackNodeId}`, nodeId: fallbackNodeId });
    const activeTabId = tabs.some((tab) => tab.id === stored.activeTabId) ? stored.activeTabId : tabs[0].id;
    const filter = (values: string[]) => [...new Set(values.filter((id) => validNodeIds.has(id)))];
    return {
      version: 1,
      tabs,
      activeTabId,
      back: filter(stored.back),
      forward: filter(stored.forward),
      recent: filter(stored.recent),
      bookmarks: filter(stored.bookmarks),
      split: stored.split && validNodeIds.has(stored.split.nodeId) ? stored.split : null,
    } satisfies WorkspaceNavigation;
  } catch {
    return defaultNavigation(fallbackNodeId);
  }
}

function writeWorkspaceNavigation(workspaceId: string, navigation: WorkspaceNavigation) {
  try {
    const all = JSON.parse(window.localStorage.getItem(navigationStorageKey) ?? "{}") as Record<string, WorkspaceNavigation>;
    window.localStorage.setItem(navigationStorageKey, JSON.stringify({ ...all, [workspaceId]: navigation }));
  } catch {
    // Navigation session is device-local and best effort; document drafts are stored separately.
  }
}

function readThemeMode(): ThemeMode {
  if (typeof window === "undefined") return "system";
  const value = window.localStorage.getItem("weftext.theme.v1");
  return value === "light" || value === "dark" || value === "contrast" ? value : "system";
}

function deviceEditorStateKey(workspaceId: string, nodeId: string) {
  return `${workspaceId}/${nodeId}`;
}

function localDateValue(date = new Date()) {
  return `${date.getFullYear().toString().padStart(4, "0")}-${(date.getMonth() + 1).toString().padStart(2, "0")}-${date.getDate().toString().padStart(2, "0")}`;
}

function chronoNodeName(period: "year" | "quarter" | "month" | "week" | "day", dateValue: string) {
  const [yearText, monthText, dayText] = dateValue.split("-");
  if (period === "year") return yearText;
  if (period === "quarter") return `${yearText}-Q${Math.floor((Number(monthText) - 1) / 3) + 1}`;
  if (period === "month") return `${yearText}-${monthText}`;
  if (period === "day") return dateValue;
  const date = new Date(Date.UTC(Number(yearText), Number(monthText) - 1, Number(dayText)));
  const weekday = date.getUTCDay() || 7;
  date.setUTCDate(date.getUTCDate() + 4 - weekday);
  const weekYear = date.getUTCFullYear();
  const yearStart = new Date(Date.UTC(weekYear, 0, 1));
  const week = Math.ceil(((date.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
  return `${weekYear.toString().padStart(4, "0")}-W${week.toString().padStart(2, "0")}`;
}

async function requestCore(endpoint: string, token: string, path: string, body?: unknown) {
  if (window.weftextDesktop) {
    return window.weftextDesktop.request(path, body) as Promise<Record<string, unknown>>;
  }
  const response = await fetch(`${endpoint}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    cache: "no-store",
  });
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error ?? "Core 拒绝了请求");
  return payload as Record<string, unknown>;
}

function renderHeading(level: number, text: ReactNode, key: number) {
  const normalized = Math.max(1, Math.min(level, 9));
  if (normalized <= 6) return createElement(`h${normalized}`, { key }, text);
  return <div className={`extended-heading heading-level-${normalized}`} role="heading" aria-level={normalized} key={key}>{text}</div>;
}

function renderQuote(depth: number, text: ReactNode, key: number) {
  const actualDepth = Math.max(1, Math.trunc(depth));
  const renderedDepth = Math.min(actualDepth, 32);
  let content: ReactNode = <span>{text}</span>;
  for (let level = renderedDepth; level >= 1; level -= 1) {
    content = <blockquote className={`quote-level-${Math.min(level, 9)}`} data-quote-level={level}>{content}</blockquote>;
  }
  return <div className="nested-quote" data-quote-depth={actualDepth} aria-label={`引用层级 ${actualDepth}`} key={key}>{content}</div>;
}

function renderCitationRichText(content: CitationRichText, key: string) {
  return content.runs.map((run, index) => {
    const style = {
      fontStyle: run.style.italic ? "italic" : undefined,
      fontVariant: run.style.smallCaps ? "small-caps" : undefined,
      fontWeight: run.style.weight === "normal" ? undefined : run.style.weight,
      textDecoration: run.style.underline ? "underline" : undefined,
      verticalAlign: run.style.verticalAlign === "none" ? undefined : run.style.verticalAlign,
      fontSize: run.style.verticalAlign === "superscript" || run.style.verticalAlign === "subscript" ? "0.78em" : undefined,
    } as const;
    const common = { style, dir: "auto" as const, "data-reference-node-id": run.referenceNodeId ?? undefined };
    return run.link ? <a key={`${key}-${index}`} {...common} href={run.link} rel="noreferrer">{run.text}</a> : <span key={`${key}-${index}`} {...common}>{run.text}</span>;
  });
}

function citationProjectedBlock(
  source: string,
  block: DocumentBlock,
  component: CitationComponentPresentation | null,
): ReactNode {
  if (!component) return block.text;
  const replacements: Array<{ range: CitationRange; content: ReactNode }> = component.citations
    .filter((citation) => citation.sourceRange.start >= block.start && citation.sourceRange.end <= block.end)
    .map((citation, index) => ({
      range: citation.sourceRange,
      content: <cite className={`citation citation-${citation.form}`} data-source-start={citation.sourceRange.start} key={`citation-${index}`}>{renderCitationRichText(citation.content, `citation-${index}`)}</cite>,
    }));
  const bibliography = component.bibliography;
  if (bibliography && bibliography.sourceRange.start >= block.start && bibliography.sourceRange.end <= block.end) {
    replacements.push({
      range: bibliography.sourceRange,
      content: <section className={`citation-bibliography ${bibliography.hangingIndent ? "hanging" : ""}`} aria-label="参考文献" key="bibliography">{bibliography.entries.map((entry, index) => <div className="citation-bibliography-entry" data-reference-node-id={entry.referenceNodeId} key={entry.referenceNodeId}>{entry.firstField && <span className="citation-first-field">{renderCitationRichText(entry.firstField, `first-${index}`)}</span>}<span>{renderCitationRichText(entry.content, `entry-${index}`)}</span></div>)}</section>,
    });
  }
  if (!replacements.length) return block.text;
  replacements.sort((left, right) => left.range.start - right.range.start);
  const startByte = replacements.every((replacement) => replacement.range.start >= block.textStart && replacement.range.end <= block.textEnd) ? block.textStart : block.start;
  const endByte = startByte === block.textStart ? block.textEnd : block.end;
  const start = stringOffsetAtUtf8Byte(source, startByte);
  const end = stringOffsetAtUtf8Byte(source, endByte);
  const rendered: ReactNode[] = [];
  let cursor = start;
  replacements.forEach((replacement, index) => {
    const replacementStart = stringOffsetAtUtf8Byte(source, replacement.range.start);
    const replacementEnd = stringOffsetAtUtf8Byte(source, replacement.range.end);
    if (replacementStart > cursor) rendered.push(<span key={`plain-${index}`}>{source.slice(cursor, replacementStart)}</span>);
    rendered.push(replacement.content);
    cursor = replacementEnd;
  });
  if (cursor < end) rendered.push(<span key="plain-tail">{source.slice(cursor, end)}</span>);
  return rendered;
}

function blockContainsBibliography(block: DocumentBlock, component: CitationComponentPresentation | null) {
  const bibliography = component?.bibliography;
  return Boolean(bibliography && bibliography.sourceRange.start >= block.start && bibliography.sourceRange.end <= block.end);
}

function renderListItems(items: DocumentListItem[], kind: DocumentListModel["kind"], key: string): ReactNode {
  const ordered = kind === "ordered" || kind === "callout";
  const children = items.map((item, index) => <li key={`${key}-${index}`}>
    {item.checked !== null && <input type="checkbox" checked={item.checked} readOnly aria-label={item.checked ? "已完成" : "未完成"} />}
    <span>{item.text}</span>
    {item.children.length > 0 && renderListItems(item.children, kind, `${key}-${index}-children`)}
  </li>);
  return ordered ? <ol>{children}</ol> : <ul>{children}</ul>;
}

function renderTableRow(row: DocumentTableRow, header: boolean, key: string) {
  return <tr key={key}>{row.cells.map((cell, index) => {
    const props = { colSpan: cell.columnSpan, rowSpan: cell.rowSpan, key: `${key}-${index}` };
    return header ? <th {...props} scope="col">{cell.text}</th> : <td {...props}>{cell.text}</td>;
  })}</tr>;
}

function renderTable(model: DocumentTableModel, key: number) {
  return <table key={key} className="projected-table">
    {model.header && <thead>{renderTableRow(model.header, true, `header-${key}`)}</thead>}
    <tbody>{model.body.map((row, index) => renderTableRow(row, false, `body-${key}-${index}`))}</tbody>
    {model.footer && <tfoot>{renderTableRow(model.footer, false, `footer-${key}`)}</tfoot>}
  </table>;
}

function renderPreservedSource(block: DocumentBlock, index: number, label: string) {
  return <figure className={`document-source-block block-${block.kind}`} key={index}>
    <figcaption>{label}（仅保留源码，未执行）</figcaption>
    <pre><code>{block.text}</code></pre>
  </figure>;
}

function renderModel(model: DocumentModel, source = "", citationComponent: CitationComponentPresentation | null = null) {
  const groupedBodies = new Set(model.runInGroups.map((group) => group.bodyBlock));
  const groupedHeadings = new Map(model.runInGroups.map((group) => [group.headingBlock, group.bodyBlock]));
  const rendered: ReactNode[] = [];
  if (model.status !== "complete") {
    rendered.push(<aside className={`document-analysis-status status-${model.status}`} role="status" key="analysis-status">
      <strong>{model.status === "failed" ? "Core 无法完整解析；以下内容以安全源码形式显示" : "部分 AsciiDoc 语义采用受限显示"}</strong>
      {model.degradations.length > 0 && <ul>{model.degradations.map((item, index) => <li key={`${item.kind}-${index}`}>{item.message}</li>)}</ul>}
    </aside>);
  }
  model.blocks.forEach((block, index) => {
    if (block.kind === "frontmatter" || groupedBodies.has(index)) return;
    const bodyIndex = groupedHeadings.get(index);
    if (bodyIndex !== undefined) {
      const body = model.blocks[bodyIndex];
      rendered.push(<div className="run-in-paragraph" key={index}>{renderHeading(block.headingLevel ?? 1, citationProjectedBlock(source, block, citationComponent), index)}<span>{body ? citationProjectedBlock(source, body, citationComponent) : ""}</span></div>);
      return;
    }
    if (block.kind === "document_title") {
      rendered.push(<h1 className="document-title" key={index}>{block.text}</h1>);
    } else if (block.kind === "document_subtitle") {
      rendered.push(<p className="document-subtitle" key={index}>{block.text}</p>);
    } else if (block.kind === "heading") {
      rendered.push(renderHeading(block.headingLevel ?? 1, citationProjectedBlock(source, block, citationComponent), index));
    } else if (block.kind === "paragraph") {
      rendered.push(blockContainsBibliography(block, citationComponent)
        ? <div className="citation-bibliography-block" key={index}>{citationProjectedBlock(source, block, citationComponent)}</div>
        : <p key={index}>{citationProjectedBlock(source, block, citationComponent)}</p>);
    } else if (block.kind === "quote") {
      const quote = renderQuote(block.quoteDepth ?? 1, citationProjectedBlock(source, block, citationComponent), index);
      rendered.push(block.semantic.kind === "quote" && (block.semantic.attribution || block.semantic.citation)
        ? <figure className="projected-quote" key={index}>{quote}<figcaption>{[block.semantic.attribution, block.semantic.citation].filter(Boolean).join(", ")}</figcaption></figure>
        : quote);
    } else if (block.kind === "list") {
      rendered.push(block.semantic.kind === "list"
        ? <div className="projected-list" key={index}>{renderListItems(block.semantic.model.items, block.semantic.model.kind, `list-${index}`)}</div>
        : renderPreservedSource(block, index, "列表"));
    } else if (block.kind === "table") {
      rendered.push(block.semantic.kind === "table" ? renderTable(block.semantic.model, index) : renderPreservedSource(block, index, "表格"));
    } else if (block.kind === "image") {
      const label = block.semantic.kind === "image" ? block.semantic.alt ?? block.semantic.target : block.text;
      rendered.push(<figure className="projected-image-placeholder" key={index} role="img" aria-label={label}><figcaption>图片：{label}</figcaption></figure>);
    } else if (block.kind === "block_title") {
      rendered.push(<div className="document-block-title" key={index}>{block.text}</div>);
    } else if (block.kind === "thematic_break") {
      rendered.push(<hr key={index} />);
    } else if (block.kind === "comment") {
      return;
    } else if (block.kind === "passthrough" || block.kind === "html" || block.kind === "unsupported") {
      rendered.push(renderPreservedSource(block, index, block.kind === "passthrough" ? "Passthrough" : block.kind === "html" ? "HTML" : "不支持的 AsciiDoc 块"));
    } else if (block.kind === "math" || block.kind === "mermaid") {
      rendered.push(<pre className={`document-source-block block-${block.kind}`} key={index} aria-label={block.kind === "math" ? "数学源码" : "Mermaid 源码"}><code>{block.text}</code></pre>);
    } else {
      rendered.push(<pre className={`document-source-block block-${block.kind}`} key={index}><code>{block.text}</code></pre>);
    }
  });
  return rendered;
}

function workspaceTreeNodes(workspace: LiveWorkspace): TreeNode[] {
  if (workspace.navigation?.version === 1) {
    return workspace.navigation.hierarchy.map((node) => {
      return {
        id: node.nodeId,
        name: node.name,
        parentId: node.parentNodeId,
        path: node.locator,
        depth: node.depth,
        kind: node.childCount ? "folder" : "note",
        icon: node.displayIcon.kind === "explicit_node" ? node.displayIcon.explicit ?? null : null,
        displayIcon: node.displayIcon,
      };
    });
  }
  const parentIds = new Set(workspace.nodes.map((node) => node.parentId).filter((id): id is string => Boolean(id)));
  return workspace.nodes.map((node) => ({
    ...node,
    depth: node.path ? node.path.split("/").length : 0,
    kind: parentIds.has(node.id) ? "folder" : "note",
  }));
}

function workspaceIconGlyph(displayIcon?: WorkspaceItemIcon | null, explicit?: ResolvedNodeIcon | null) {
  if (displayIcon?.kind === "explicit_node") return displayIcon.explicit?.glyph ?? explicit?.glyph ?? null;
  return ({
    default_node: "文",
    folder: "夹",
    markdown_file: "M",
    file: "档",
    workspace_root: "根",
    trash: "废",
  } as Record<string, string>)[displayIcon?.kind ?? ""] ?? explicit?.glyph ?? null;
}

function sourcePosition(source: string, offset: number) {
  const before = source.slice(0, offset);
  const lines = before.split("\n");
  return { line: lines.length, column: (lines.at(-1)?.length ?? 0) + 1 };
}

function stringOffsetAtUtf8Byte(source: string, byteOffset: number) {
  if (byteOffset <= 0) return 0;
  let bytes = 0;
  let stringOffset = 0;
  for (const scalar of source) {
    const nextBytes = bytes + new TextEncoder().encode(scalar).length;
    if (nextBytes > byteOffset) break;
    bytes = nextBytes;
    stringOffset += scalar.length;
  }
  return stringOffset;
}

function utf8ByteOffsetAtString(source: string, stringOffset: number) {
  return new TextEncoder().encode(source.slice(0, Math.max(0, Math.min(stringOffset, source.length)))).length;
}

function coreDocumentHeadings(source: string, model: DocumentModel): DocumentHeading[] {
  return model.blocks.flatMap((block) => {
    if (block.kind !== "heading") return [];
    const start = stringOffsetAtUtf8Byte(source, block.start);
    return [{
      level: block.headingLevel ?? 1,
      text: block.text,
      start,
      line: sourcePosition(source, start).line,
    }];
  });
}

function editorDocumentModel(source: string, model: DocumentModel): DocumentModel {
  return {
    ...model,
    blocks: model.blocks.map((block) => ({
      ...block,
      start: stringOffsetAtUtf8Byte(source, block.start),
      end: stringOffsetAtUtf8Byte(source, block.end),
      textStart: stringOffsetAtUtf8Byte(source, block.textStart),
      textEnd: stringOffsetAtUtf8Byte(source, block.textEnd),
    })),
  };
}

type BlockFormatAction = "paragraph" | "list" | "quote_increase" | "quote_decrease" | "code";
type InlineFormatAction = "bold" | "emphasis" | "inline_code" | "link";

function findDocumentMatches(source: string, query: string, includeFrontmatter: boolean, bodyStart: number): DocumentMatch[] {
  if (!query) return [];
  const baseOffset = includeFrontmatter ? 0 : Math.max(0, Math.min(bodyStart, source.length));
  const searchable = source.slice(baseOffset);
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matches: DocumentMatch[] = [];
  for (const match of searchable.matchAll(new RegExp(escaped, "giu"))) {
    const relative = match.index ?? 0;
    matches.push({
      start: baseOffset + relative,
      end: baseOffset + relative + match[0].length,
    });
  }
  return matches;
}

type ModelLoadState = "idle" | "pending" | "ready" | "error";

const unavailableNode: TreeNode = { id: "__unavailable__", name: "未打开文档", depth: 0, kind: "note" };
const unavailableDocument: DemoDocument = { parent: "", title: "未打开文档", label: "等待 Core", lead: "", body: [] };

function coreModelFromPayload(payload: Record<string, unknown>) {
  if (!isDocumentModel(payload.model)) throw new Error("Core 返回了无效的文档模型契约");
  if (payload.profile !== undefined && !isDocumentProfile(payload.profile)) throw new Error("Core 返回了无效的 AsciiDoc profile 契约");
  return payload.model;
}

function requireLiveDocumentContract(document: LiveDocument, label = "文档") {
  if (!isDocumentModel(document.model)) throw new Error(`Core 返回了无效的${label}模型契约`);
  if (document.profile !== undefined && !isDocumentProfile(document.profile)) throw new Error(`Core 返回了无效的${label} profile 契约`);
  if (document.view !== undefined && !isDocumentViewModel(document.view)) throw new Error(`Core 返回了无效的${label}视图契约`);
  if (document.metadata !== undefined && !isNodeMetadataProjection(document.metadata)) throw new Error(`Core 返回了无效的${label}节点元数据契约`);
  if (document.metadata && document.metadata.id !== document.nodeId) throw new Error(`Core 返回的${label}节点身份与元数据不一致`);
  if (document.profile && document.view && document.profile.profile !== document.view.profile) throw new Error(`Core 返回的${label} profile 与视图不一致`);
}

function coreBodyStart(source: string, model: DocumentModel | null) {
  const frontmatter = model?.blocks.find((block) => block.kind === "frontmatter");
  return frontmatter ? stringOffsetAtUtf8Byte(source, frontmatter.end) : 0;
}

export function WeftextApp({ demo = null }: { demo?: DemoWorkspace | null }) {
  const initialNodes = (demo?.nodes ?? [unavailableNode]) as TreeNode[];
  const initialNodeId = demo?.initialNodeId ?? unavailableNode.id;
  const initialWorkspaceId = demo?.id ?? "unopened";
  const [view, setView] = useState<ViewMode>("write");
  const [theme, setTheme] = useState<ThemeMode>(readThemeMode);
  const [iconPreferences, setIconPreferences] = useState(readIconPreferences);
  const [iconQuery, setIconQuery] = useState("");
  const [nodes, setNodes] = useState(initialNodes);
  const [selected, setSelected] = useState(() => {
    const navigation = readWorkspaceNavigation(initialWorkspaceId, initialNodeId, new Set(initialNodes.map((node) => node.id)));
    return navigation.tabs.find((tab) => tab.id === navigation.activeTabId)?.nodeId ?? initialNodeId;
  });
  const [navigation, setNavigation] = useState<WorkspaceNavigation>(() => readWorkspaceNavigation(initialWorkspaceId, initialNodeId, new Set(initialNodes.map((node) => node.id))));
  const [searchCreatesTab, setSearchCreatesTab] = useState(false);
  const [splitDocument, setSplitDocument] = useState<LiveDocument | null>(null);
  const [splitModel, setSplitModel] = useState<DocumentModel | null>(null);
  const [splitModelState, setSplitModelState] = useState<ModelLoadState>("idle");
  const [splitModelError, setSplitModelError] = useState("");
  const [splitFindOpen, setSplitFindOpen] = useState(false);
  const [splitReplaceOpen, setSplitReplaceOpen] = useState(false);
  const [splitFindQuery, setSplitFindQuery] = useState("");
  const [splitReplaceText, setSplitReplaceText] = useState("");
  const [splitFindIndex, setSplitFindIndex] = useState(0);
  const [splitSelectionRestoreToken, setSplitSelectionRestoreToken] = useState(0);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [query, setQuery] = useState("");
  const [newName, setNewName] = useState("");
  const [commentsOpen, setCommentsOpen] = useState(true);
  const [toast, setToast] = useState<string | null>(null);
  const [liveDocument, setLiveDocument] = useState<LiveDocument | null>(null);
  const [editorSource, setEditorSource] = useState("");
  const [draftSources, setDraftSources] = useState<Record<string, string>>({});
  const draftSourcesRef = useRef<Record<string, string>>({});
  const [dirtyNodeIds, setDirtyNodeIds] = useState<Set<string>>(() => new Set());
  const [draftModel, setDraftModel] = useState<DocumentModel | null>(null);
  const [draftModelSource, setDraftModelSource] = useState("");
  const [draftModelState, setDraftModelState] = useState<ModelLoadState>("idle");
  const [draftModelError, setDraftModelError] = useState("");
  const [propertyAnalysis, setPropertyAnalysis] = useState<DocumentPropertyAnalysis | null>(null);
  const [propertyAnalysisSource, setPropertyAnalysisSource] = useState("");
  const [coreEndpoint, setCoreEndpoint] = useState("");
  const [coreToken, setCoreToken] = useState("");
  const [coreState, setCoreState] = useState<"idle" | "connecting" | "connected" | "error">(() => typeof window !== "undefined" && window.weftextDesktop ? "connecting" : "idle");
  const [coreError, setCoreError] = useState("");
  const [savePlan, setSavePlan] = useState<SavePlan | null>(null);
  const [saveTarget, setSaveTarget] = useState<"primary" | "split">("primary");
  const [liveWorkspace, setLiveWorkspace] = useState<LiveWorkspace | null>(null);
  const [structuralPlan, setStructuralPlan] = useState<StructuralPlan | null>(null);
  const [structuralContext, setStructuralContext] = useState<StructuralContext>(null);
  const [structuralDraftScope, setStructuralDraftScope] = useState<FrozenReviewedDraftScope | null>(null);
  const [showTrash, setShowTrash] = useState(false);
  const [selectedTrashItemId, setSelectedTrashItemId] = useState<string | null>(null);
  const [trashRestoreMode, setTrashRestoreMode] = useState<"original" | "with_ancestors" | "existing_target">("original");
  const [trashRestoreTarget, setTrashRestoreTarget] = useState("");
  const [trashRestoreName, setTrashRestoreName] = useState("");
  const [trashResourceNames, setTrashResourceNames] = useState("");
  const [permanentDeleteConfirmed, setPermanentDeleteConfirmed] = useState(false);
  const [nodeActionInvocation, setNodeActionInvocation] = useState<WorkspaceActionInvocation | null>(null);
  const [nodeActionTarget, setNodeActionTarget] = useState<FrozenWorkspaceActionTarget | null>(null);
  const [moveParent, setMoveParent] = useState("");
  const [moveName, setMoveName] = useState("");
  const [desktopMode, setDesktopMode] = useState(() => typeof window !== "undefined" && Boolean(window.weftextDesktop));
  const [restoreError, setRestoreError] = useState("");
  const [draftRecovery, setDraftRecovery] = useState<DraftRecovery>(emptyDraftRecovery);
  const [draftSaveState, setDraftSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [safeMode, setSafeMode] = useState(false);
  const [diagnostics, setDiagnostics] = useState<DesktopDiagnostics | null>(null);
  const [workspaceSearch, setWorkspaceSearch] = useState<SearchResult[]>([]);
  const [derivedIndexWarning, setDerivedIndexWarning] = useState<DerivedIndexWarning | null>(null);
  const [inspectorTab, setInspectorTab] = useState<"outline" | "properties" | "tasks" | "citations" | "annotations" | "backlinks">("annotations");
  const [propertyKey, setPropertyKey] = useState("");
  const [propertyValue, setPropertyValue] = useState("");
  const [propertyExisting, setPropertyExisting] = useState(false);
  const [aliasesInput, setAliasesInput] = useState("");
  const [childSortInput, setChildSortInput] = useState<"name" | "manual">("name");
  const [childSortDirectionInput, setChildSortDirectionInput] = useState<"ascending" | "descending">("ascending");
  const [siblingRankInput, setSiblingRankInput] = useState("");
  const [resourcePlan, setResourcePlan] = useState<ResourcePlan | null>(null);
  const [liveAnnotations, setLiveAnnotations] = useState<LiveAnnotationStore | null>(null);
  const [annotationPlan, setAnnotationPlan] = useState<AnnotationPlan | null>(null);
  const [annotationAction, setAnnotationAction] = useState<AnnotationActionName>("create");
  const [annotationTarget, setAnnotationTarget] = useState<string | null>(null);
  const [annotationMessageTarget, setAnnotationMessageTarget] = useState<string | null>(null);
  const [annotationCreateKind, setAnnotationCreateKind] = useState<AnnotationKind>("comment");
  const [annotationBody, setAnnotationBody] = useState("");
  const [annotationSuggestedSource, setAnnotationSuggestedSource] = useState("");
  const [annotationMark, setAnnotationMark] = useState("highlight");
  const [annotationColor, setAnnotationColor] = useState("yellow");
  const [annotationLabels, setAnnotationLabels] = useState("");
  const [annotationActor, setAnnotationActor] = useState(readAnnotationActor);
  const [citationDraft, setCitationDraft] = useState<CitationDraftPayload | null>(null);
  const [citationDraftSource, setCitationDraftSource] = useState("");
  const [citationAnalysisError, setCitationAnalysisError] = useState("");
  const [citationStyle, setCitationStyle] = useState("apa");
  const [citationLocale, setCitationLocale] = useState("en-US");
  const [citationAction, setCitationAction] = useState<"citation" | "nocite" | "bibliography">("citation");
  const [citationForm, setCitationForm] = useState<"parenthetical" | "narrative">("parenthetical");
  const [citationQuery, setCitationQuery] = useState("");
  const [citationHits, setCitationHits] = useState<CitationReferenceHit[]>([]);
  const [citationSearchIndex, setCitationSearchIndex] = useState(0);
  const [citationSelectedReference, setCitationSelectedReference] = useState<string | null>(null);
  const [citationItems, setCitationItems] = useState<CitationIntentItem[]>([]);
  const [citationEditRange, setCitationEditRange] = useState<CitationRange | null>(null);
  const [citationLocator, setCitationLocator] = useState("");
  const [citationLabel, setCitationLabel] = useState("page");
  const [citationPrefix, setCitationPrefix] = useState("");
  const [citationSuffix, setCitationSuffix] = useState("");
  const [bibliographyInclusion, setBibliographyInclusion] = useState<"cited" | "all">("cited");
  const [chronoDate, setChronoDate] = useState(localDateValue);
  const [chronoTargetName, setChronoTargetName] = useState("");
  const [findOpen, setFindOpen] = useState(false);
  const [replaceOpen, setReplaceOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [replaceText, setReplaceText] = useState("");
  const [findIndex, setFindIndex] = useState(0);
  const [findNavigated, setFindNavigated] = useState(false);
  const [collapsedNodes, setCollapsedNodes] = useState<Set<string>>(() => new Set());
  const initialExplorerState = useRef(readExplorerState(initialWorkspaceId));
  const [explorerActivity, setExplorerActivity] = useState<ExplorerActivity>(initialExplorerState.current.activity);
  const [explorerMode, setExplorerMode] = useState<ExplorerMode>(initialExplorerState.current.mode);
  const [explorerFilter, setExplorerFilter] = useState(initialExplorerState.current.filter);
  const [explorerWidth, setExplorerWidth] = useState(initialExplorerState.current.width);
  const [explorerScrollTop, setExplorerScrollTop] = useState(initialExplorerState.current.scrollTop);
  const [contentsBrowseLocator, setContentsBrowseLocator] = useState<string | null>(initialExplorerState.current.browseLocator);
  const [hierarchyLimit, setHierarchyLimit] = useState(INITIAL_NAVIGATION_WINDOW);
  const [contentsLimit, setContentsLimit] = useState(INITIAL_NAVIGATION_WINDOW);
  const [focusedPane, setFocusedPane] = useState<"primary" | "split">("primary");
  const [navigationPerformance, setNavigationPerformance] = useState<NavigationPerformanceSample[]>([]);
  const [searchIndex, setSearchIndex] = useState(0);
  const [cursor, setCursor] = useState({ line: 1, column: 1 });
  const [documentSelection, setDocumentSelection] = useState({ start: 0, end: 0 });
  const [documentScrollTop, setDocumentScrollTop] = useState(0);
  const [selectionRestoreToken, setSelectionRestoreToken] = useState(0);
  const editorStatesRef = useRef<Record<string, DeviceEditorState>>(readDeviceEditorStates());
  const nodeRequestRef = useRef(0);
  const searchRequestRef = useRef(0);
  const modelRequestRef = useRef(0);
  const splitModelRequestRef = useRef(0);
  const citationRequestRef = useRef(0);
  const citationSearchRequestRef = useRef(0);
  const metadataRequestRef = useRef(0);
  const primaryDocumentNodeRef = useRef(selected);
  const splitDocumentNodeRef = useRef(splitDocument?.nodeId ?? null);
  primaryDocumentNodeRef.current = selected;
  splitDocumentNodeRef.current = splitDocument?.nodeId ?? null;
  const liveDocumentRef = useRef<LiveDocument | null>(liveDocument);
  liveDocumentRef.current = liveDocument;
  const dirtyNodeIdsRef = useRef(dirtyNodeIds);
  dirtyNodeIdsRef.current = dirtyNodeIds;
  const draftRecoveryRef = useRef(draftRecovery);
  draftRecoveryRef.current = draftRecovery;
  const searchInputRef = useRef<HTMLInputElement>(null);
  const findInputRef = useRef<HTMLInputElement>(null);
  const explorerScrollRef = useRef<HTMLDivElement>(null);
  const explorerWorkspaceRef = useRef(initialWorkspaceId);
  const navigationRenderStartedRef = useRef(performance.now());
  const sharedCoreRequest = useCallback(
    (path: string, body?: unknown) => requestCore(coreEndpoint, coreToken, path, body),
    [coreEndpoint, coreToken],
  );
  const loadMetadataInputs = useCallback((metadata?: NodeMetadataProjection) => {
    setAliasesInput(metadata?.aliases.join("\n") ?? "");
    setChildSortInput(metadata?.childSort ?? "name");
    setChildSortDirectionInput(metadata?.childSortDirection ?? "ascending");
    setSiblingRankInput(metadata?.siblingRank?.toString() ?? "");
  }, []);

  useEffect(() => {
    try {
      window.localStorage.setItem("weftext.theme.v1", theme);
    } catch {
      // Theme is device-local preference and may be unavailable in private storage modes.
    }
  }, [theme]);
  useEffect(() => {
    try {
      window.localStorage.setItem(iconPreferenceStorageKey, JSON.stringify(iconPreferences));
    } catch {
      // Icon placement is best-effort device state; the selected icon remains portable YAML.
    }
  }, [iconPreferences]);
  useEffect(() => {
    if (!annotationActor.id || !annotationActor.name.trim()) return;
    try {
      window.localStorage.setItem(
        ANNOTATION_ACTOR_STORAGE_KEY,
        JSON.stringify({ id: annotationActor.id, name: annotationActor.name.trim() }),
      );
    } catch {
      // Portable messages still carry the current snapshot when device storage is unavailable.
    }
  }, [annotationActor]);
  useEffect(() => {
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    writeWorkspaceNavigation(workspaceId, navigation);
  }, [initialWorkspaceId, liveWorkspace?.rootNodeId, navigation]);
  useEffect(() => {
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    if (explorerWorkspaceRef.current === workspaceId) return;
    const restored = readExplorerState(workspaceId);
    explorerWorkspaceRef.current = workspaceId;
    setExplorerActivity(restored.activity);
    setExplorerMode(restored.mode);
    setExplorerFilter(restored.filter);
    setExplorerWidth(restored.width);
    setExplorerScrollTop(restored.scrollTop);
    setContentsBrowseLocator(restored.browseLocator);
    setCollapsedNodes(new Set(restored.collapsedNodeIds));
    setHierarchyLimit(INITIAL_NAVIGATION_WINDOW);
    setContentsLimit(INITIAL_NAVIGATION_WINDOW);
  }, [initialWorkspaceId, liveWorkspace?.rootNodeId]);
  useEffect(() => {
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    writeExplorerState(workspaceId, {
      version: 1,
      activity: explorerActivity,
      mode: explorerMode,
      collapsedNodeIds: [...collapsedNodes],
      filter: explorerFilter,
      width: explorerWidth,
      scrollTop: explorerScrollTop,
      browseLocator: contentsBrowseLocator,
    });
  }, [collapsedNodes, contentsBrowseLocator, explorerActivity, explorerFilter, explorerMode, explorerScrollTop, explorerWidth, initialWorkspaceId, liveWorkspace?.rootNodeId]);
  useLayoutEffect(() => {
    if (explorerScrollRef.current) explorerScrollRef.current.scrollTop = explorerScrollTop;
  }, [explorerActivity, explorerMode, explorerScrollTop]);
  const documentSurfaceRef = useRef<HTMLElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const writeEditorRef = useRef<HTMLTextAreaElement>(null);

  const activeNode = nodes.find((node) => node.id === selected) ?? nodes[0];
  const sharedProjection = useMemo(
    () => navigationProjection(
      liveWorkspace?.rootNodeId ?? nodes[0]?.id ?? initialWorkspaceId,
      liveWorkspace?.navigation,
      nodes,
      liveWorkspace?.content ?? [],
    ),
    [initialWorkspaceId, liveWorkspace?.content, liveWorkspace?.navigation, liveWorkspace?.rootNodeId, nodes],
  );
  const followedNodeId = focusedPane === "split" && navigation.split ? navigation.split.nodeId : selected;
  const followedNode = nodes.find((node) => node.id === followedNodeId) ?? activeNode;
  const contentsLocator = contentsBrowseLocator ?? followedNode.path ?? "";
  const hierarchyRows = useMemo(
    () => visibleHierarchy(sharedProjection, collapsedNodes, explorerFilter)
      .filter(() => (liveWorkspace ? !showTrash : true)),
    [collapsedNodes, explorerFilter, liveWorkspace, sharedProjection, showTrash],
  );
  const renderedHierarchy = incrementalWindow(hierarchyRows, hierarchyLimit);
  const contentsRows = useMemo(
    () => directContents(sharedProjection, contentsLocator, explorerFilter),
    [contentsLocator, explorerFilter, sharedProjection],
  );
  const renderedContents = incrementalWindow(contentsRows, contentsLimit);
  const contentsBreadcrumbs = locationBreadcrumbs(sharedProjection, contentsLocator);
  const activeIcon = liveDocument?.metadata?.resolvedIcon ?? activeNode.icon ?? null;
  const iconOptions = useMemo(() => {
    const builtIns = (liveWorkspace?.iconCatalog ?? []).map((icon) => ({ value: icon.id, label: icon.label, glyph: icon.glyph, kind: "built_in" as const }));
    const emoji = emojiIconCatalog.map((glyph) => ({ value: glyph, label: glyph, glyph, kind: "emoji" as const }));
    const needle = iconQuery.trim().toLocaleLowerCase();
    return [...emoji, ...builtIns].filter((icon) => !needle || icon.label.toLocaleLowerCase().includes(needle) || icon.value.toLocaleLowerCase().includes(needle));
  }, [iconQuery, liveWorkspace]);
  const breadcrumbNodes = useMemo(() => {
    const chain: TreeNode[] = [];
    let current: TreeNode | undefined = activeNode;
    const seen = new Set<string>();
    while (current && !seen.has(current.id)) {
      chain.unshift(current);
      seen.add(current.id);
      current = current.parentId ? nodes.find((node) => node.id === current?.parentId) : undefined;
    }
    return chain;
  }, [activeNode, nodes]);
  useEffect(() => {
    const validated = validatedBrowseLocator(sharedProjection, contentsBrowseLocator);
    if (contentsBrowseLocator !== null && validated === null) {
      const timer = window.setTimeout(() => {
        setContentsBrowseLocator(null);
        setToast("先前浏览位置已不可用；Explorer 已回到当前编辑节点");
      }, 0);
      return () => window.clearTimeout(timer);
    }
  }, [contentsBrowseLocator, sharedProjection]);
  useEffect(() => {
    const sample = interactionMeasurement(
      "initial_render",
      navigationRenderStartedRef.current,
      renderedHierarchy.items.length,
      hierarchyRows.length,
    );
    setNavigationPerformance((current) => current.some((entry) => entry.operation === "initial_render") ? current : [sample]);
  }, [hierarchyRows.length, renderedHierarchy.items.length]);
  const recentNodes = navigation.recent.map((id) => nodes.find((node) => node.id === id)).filter((node): node is TreeNode => Boolean(node)).slice(0, 6);
  const bookmarkedNodes = navigation.bookmarks.map((id) => nodes.find((node) => node.id === id)).filter((node): node is TreeNode => Boolean(node));
  const sampleDocument = demo?.documentFor(activeNode.id, activeNode.name) ?? unavailableDocument;
  const document = liveDocument ? {
    parent: "本机 Core",
    title: liveDocument.name,
    label: "真实文档",
    lead: "当前显示由 Rust Core 读取的精确文档源。",
    body: ["此竖切只开放一个显式选择的节点。", "保存前会先生成确定性预览。", "旧 revision 会被拒绝，不会覆盖外部修改。"],
  } : sampleDocument;
  const sampleAsciiDoc = demo?.sourceFor(selected, activeNode.name) ?? "";
  const currentSource = liveDocument ? editorSource : draftSources[selected] ?? sampleAsciiDoc;
  const nodeMetadata = liveDocument?.metadata ?? null;
  const metadataEditable = Boolean(
    liveWorkspace
      && liveDocument
      && nodeMetadata
      && currentSource === liveDocument.source
      && !dirtyNodeIds.has(liveDocument.nodeId),
  );
  const siblingRankParsed = siblingRankInput.trim() ? Number(siblingRankInput) : null;
  const siblingRankValid = siblingRankParsed === null
    || (Number.isSafeInteger(siblingRankParsed) && siblingRankParsed > 0);
  const activeProfile: DocumentProfileId = liveDocument?.profile?.profile ?? "ascii_doc_v1";
  const activeProfileName = "AsciiDoc";
  const splitProfile: DocumentProfileId = splitDocument?.profile?.profile ?? "ascii_doc_v1";
  const localResults = useMemo(() => {
    if (!demo) return [];
    const needle = query.trim().toLocaleLowerCase();
    if (!needle) return nodes;
    return nodes.filter((node) => {
      const sample = demo.documentFor(node.id, node.name);
      const source = draftSources[node.id] ?? `${node.name}\n${sample.lead}\n${sample.body.join("\n")}\ntags: weftext product`;
      return node.name.toLocaleLowerCase().includes(needle) || source.toLocaleLowerCase().includes(needle);
    });
  }, [demo, draftSources, nodes, query]);
  const results = liveWorkspace && query.trim() ? workspaceSearch : localResults;
  const effectiveSearchIndex = Math.min(searchIndex, Math.max(0, results.length - 1));
  const trashItems = liveWorkspace?.trashItems ?? [];
  const trashCount = trashItems.length;
  const selectedTrashItem = trashItems.find((item) => item.manifest.trashItemId === selectedTrashItemId) ?? null;
  const recoveryCount = draftRecovery.drafts.length;
  const recoveryIssueCount = draftRecovery.issues.length;
  const selectedOutgoing = liveWorkspace?.links.outgoing.filter((link) => link.sourceNodeId === selected) ?? [];
  const selectedBacklinks = liveWorkspace?.links.backlinks.filter((link) => link.targetNodeId === selected) ?? [];
  const selectedMentions = liveWorkspace?.links.potentialMentions.filter((mention) => mention.sourceNodeId === selected && mention.primary) ?? [];
  const annotations: DisplayAnnotation[] = liveWorkspace ? (liveAnnotations?.annotations ?? []).map((annotation) => ({
    id: annotation.id,
    author: annotation.thread[0]?.author_name ?? "无正文标记",
    avatar: annotation.thread[0]?.author_name.trim().slice(0, 1) || "记",
    time: annotation.updated_at,
    body: annotation.thread[0]?.body.source ?? "",
    resolved: annotation.state === "resolved",
    kind: annotation.kind,
    state: annotation.state,
    resolution: annotation.resolution,
    targetKind: annotation.target.kind,
    suggestedSource: annotation.suggested_source,
    mark: annotation.appearance?.mark,
    color: annotation.appearance?.theme,
    labels: annotation.labels,
    messages: annotation.thread.map((message) => ({ id: message.id, authorId: message.author_id, authorName: message.author_name, body: message.body.source, time: message.updated_at })),
  })) : demo?.annotations[selected] ?? [];
  const unresolvedAnnotations = annotations.filter((annotation) => !annotation.resolved).length;
  const commentCount = annotations.length;
  const activePropertyAnalysis = propertyAnalysisSource === currentSource ? propertyAnalysis : null;
  const properties = activePropertyAnalysis?.properties ?? [];
  const citationAvailable = Boolean(liveWorkspace && liveDocument && liveWorkspace.documentFormat?.generation === "ascii_doc_v1");
  const taskAvailable = citationAvailable;
  const activeCitationDraft = citationDraftSource === currentSource ? citationDraft : null;
  const citationComponent = activeCitationDraft?.presentation?.components.find((component) => component.componentNodeId === liveDocument?.nodeId) ?? null;
  const citationData = activeCitationDraft?.authoring.reference.citationData ?? null;
  const citationDiagnostics: CitationDiagnostic[] = [
    ...(activeCitationDraft?.authoring.reference.diagnostics ?? []).map(({ code, message, range }) => ({ code, message, range })),
    ...(activeCitationDraft?.authoring.citations.diagnostics ?? []).map(({ code, message, range }) => ({ code, message, range })),
    ...(activeCitationDraft?.analysis.diagnostics ?? []).map(({ code, message, range }) => ({ code, message, range })),
    ...(activeCitationDraft?.presentationFailure?.diagnostics ?? []).map(({ code, message, sourceRange }) => ({ code, message, range: sourceRange })),
  ].filter((diagnostic, index, all) => all.findIndex((candidate) => candidate.code === diagnostic.code && candidate.message === diagnostic.message && candidate.range?.start === diagnostic.range?.start && candidate.range?.end === diagnostic.range?.end) === index);
  const saved = !dirtyNodeIds.has(selected);
  const taskBlockedReason = dirtyNodeIds.size > 0
    ? "任务操作需要完整已保存源集；请先保存或放弃所有打开节点的修改。"
    : draftRecovery.drafts.length > 0 || draftRecovery.issues.length > 0
      ? "任务操作已阻止；请先解决设备恢复草稿与恢复问题。"
      : "";
  const intakeBlockedReason = dirtyNodeIds.size > 0
    ? "导入提交需要完整已保存工作区；请先保存或放弃所有打开节点的修改。"
    : draftRecovery.drafts.length > 0 || draftRecovery.issues.length > 0
      ? "导入提交已阻止；请先解决设备恢复草稿与恢复问题。"
      : "";
  const exportBlockedReason = dirtyNodeIds.size > 0
    ? "导出需要完整已保存源集；请先保存或放弃所有打开节点的修改。"
    : draftRecovery.drafts.length > 0 || draftRecovery.issues.length > 0
      ? "导出已阻止；请先解决设备恢复草稿与恢复问题。"
      : "";
  const backupBlockedReason = dirtyNodeIds.size > 0
    ? "完整备份与范围恢复需要完整已保存源集；请先保存或放弃所有打开节点的修改。"
    : draftRecovery.drafts.length > 0 || draftRecovery.issues.length > 0
      ? "完整备份与范围恢复已阻止；请先解决设备恢复草稿与恢复问题。"
      : "";
  const currentRecovery = liveDocument?.recoveryDraft ?? draftRecovery.drafts.find((draft) => draft.nodeId === selected);
  const currentRecoverySource = liveDocument?.recoveryDraft?.source ?? (currentRecovery && !saved ? editorSource : undefined);
  const annotationSelectionIsRange = documentSelection.start !== documentSelection.end;
  const annotationCanPreview = (() => {
    const actorReady = Boolean(annotationActor.id && annotationActor.name.trim());
    if (annotationAction === "create") {
      if (!actorReady) return false;
      if (annotationCreateKind === "comment") return Boolean(annotationBody.trim());
      if (annotationCreateKind === "mark") return annotationMark !== "none";
      if (annotationCreateKind === "suggestion_insert") return Boolean(annotationSuggestedSource);
      return annotationSelectionIsRange;
    }
    if (annotationAction === "reply") return actorReady && Boolean(annotationBody.trim());
    if (annotationAction === "edit_message") return Boolean(annotationActor.id && annotationBody.trim());
    return true;
  })();
  const workspaceName = liveWorkspace
    ? nodes.find((node) => node.id === liveWorkspace.rootNodeId)?.name ?? "Weftext 工作区"
    : demo?.workspaceName ?? "未打开工作区";
  const activeModel = liveDocument
    ? draftModel && draftModelSource === currentSource
      ? draftModel
      : draftModelState === "idle" && liveDocument.source === currentSource
        ? liveDocument.model
        : null
    : null;
  const activeEditorModel = useMemo(() => activeModel ? editorDocumentModel(currentSource, activeModel) : null, [activeModel, currentSource]);
  const splitEditorModel = useMemo(() => splitDocument && splitModel ? editorDocumentModel(splitDocument.source, splitModel) : null, [splitDocument, splitModel]);
  const bodyStart = liveDocument ? coreBodyStart(currentSource, activeModel) : demo?.bodyStart(currentSource) ?? 0;
  const splitBodyStart = splitDocument ? coreBodyStart(splitDocument.source, splitModel) : 0;
  const headings = useMemo(() => activeModel ? coreDocumentHeadings(currentSource, activeModel) : demo?.headings(currentSource) ?? [], [activeModel, currentSource, demo]);
  const documentMatches = useMemo(() => findDocumentMatches(currentSource, findQuery, view === "source", bodyStart), [bodyStart, currentSource, findQuery, view]);
  const effectiveFindIndex = Math.min(findIndex, Math.max(0, documentMatches.length - 1));
  const splitMatches = useMemo(() => splitDocument ? findDocumentMatches(splitDocument.source, splitFindQuery, navigation.split?.view === "source", splitBodyStart) : [], [navigation.split?.view, splitBodyStart, splitDocument, splitFindQuery]);
  const effectiveSplitFindIndex = Math.min(splitFindIndex, Math.max(0, splitMatches.length - 1));
  const currentHeading = [...headings].reverse().find((heading) => heading.start <= documentSelection.start) ?? headings[0];
  const nodeActionTargetNode = nodeActionTarget?.kind === "node"
    ? nodes.find((node) => node.id === nodeActionTarget.nodeId) ?? null
    : null;
  const nodeActionMenuNodeId = nodeActionInvocation?.source === "editor_command"
    ? nodeActionInvocation.focusedNodeId
    : nodeActionInvocation?.source === "explicit_node_row"
      ? nodeActionInvocation.nodeId
      : nodeActionInvocation?.source === "resource_row"
        ? nodeActionInvocation.ownerNodeId
        : null;
  const nodeActionMenuNode = nodeActionMenuNodeId
    ? nodes.find((node) => node.id === nodeActionMenuNodeId) ?? null
    : null;
  const moveTargets = nodes;
  const textCount = currentSource.slice(bodyStart).replace(/\s/g, "").length;
  const draftStatusLabel = !liveDocument
    ? coreState === "error" ? "Core 不可用" : demo ? "已保存" : "等待 Core 文档"
    : !desktopMode || saved
      ? "Core revision 已同步"
    : draftSaveState === "saving"
      ? "正在保存恢复草稿…"
      : draftSaveState === "saved"
        ? "恢复草稿已存本机 · 预览提交"
        : draftSaveState === "error"
          ? "恢复草稿保存失败"
          : "有未保存修改 · 预览提交";

  const applyLiveConnection = useCallback(async (workspace: LiveWorkspace, remembered: LiveDocument, recovery: DraftRecovery = emptyDraftRecovery, endpoint = "", token = "") => {
    const validNodeIds = new Set(workspace.nodes.map((node) => node.id));
    const restoredNavigation = readWorkspaceNavigation(workspace.rootNodeId, remembered.nodeId, validNodeIds);
    const restoredNodeId = restoredNavigation.tabs.find((tab) => tab.id === restoredNavigation.activeTabId)?.nodeId ?? remembered.nodeId;
    const next = restoredNodeId === remembered.nodeId
      ? remembered
      : (await requestCore(endpoint, token, `/api/document?nodeId=${encodeURIComponent(restoredNodeId)}&remember=false`)).document as LiveDocument;
    requireLiveDocumentContract(next);
    const recovered = next.recoveryDraft?.source;
    const recoveredSource = recovered && !next.recoveryDraft?.stale ? recovered : next.source;
    setLiveWorkspace(workspace);
    setNodes(workspaceTreeNodes(workspace));
    setNavigation(restoredNavigation);
    setSelected(next.nodeId);
    setMoveParent(workspace.rootNodeId);
    setLiveDocument(next);
    loadMetadataInputs(next.metadata);
    setEditorSource(recoveredSource);
    draftSourcesRef.current = { [next.nodeId]: recoveredSource };
    setDraftSources(draftSourcesRef.current);
    setDirtyNodeIds(recovered && !next.recoveryDraft?.stale ? new Set([next.nodeId]) : new Set());
    setDraftRecovery(recovery);
    setDraftSaveState(recovered ? "saved" : "idle");
    setDraftModel(next.model);
    setDraftModelSource(recoveredSource === next.source ? next.source : "");
    setDraftModelState(recoveredSource === next.source ? "ready" : "pending");
    setDraftModelError("");
    setPropertyAnalysis(recoveredSource === next.source ? next.properties ?? null : null);
    setPropertyAnalysisSource(recoveredSource);
    setSplitDocument(null);
    setSplitModel(null);
    setSplitModelState("idle");
    setSplitModelError("");
    setCollapsedNodes(new Set());
    const restoredState = editorStatesRef.current[deviceEditorStateKey(workspace.rootNodeId, next.nodeId)];
    const restoredStart = Math.min(restoredState?.selectionStart ?? 0, recoveredSource.length);
    const restoredEnd = Math.min(restoredState?.selectionEnd ?? restoredStart, recoveredSource.length);
    setView(restoredState?.view ?? "write");
    setDocumentSelection({ start: restoredStart, end: restoredEnd });
    setDocumentScrollTop(Math.max(0, restoredState?.scrollTop ?? 0));
    setCursor(sourcePosition(recoveredSource, restoredStart));
    setSelectionRestoreToken((current) => current + 1);
    setFindOpen(false);
    setInspectorTab("annotations");
    setCoreState("connected");
    setCoreError("");
    setRestoreError("");
    if (recovered || next.recoveryIssue || recovery.issues.length > 0) setDialog("recovery");
  }, [loadMetadataInputs]);

  const persistDesktopDraft = useCallback(async (nodeId: string, revision: string, source: string) => {
    setDraftSaveState("saving");
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/draft/save", { nodeId, revision, source });
      setDraftRecovery((payload.draftRecovery as DraftRecovery | undefined) ?? emptyDraftRecovery);
      setDraftSaveState(payload.clean ? "idle" : "saved");
      return payload;
    } catch (error) {
      const message = error instanceof Error ? error.message : "无法保存设备本地恢复草稿";
      setDraftSaveState("error");
      setDraftRecovery((current) => ({ ...current, issues: [message, ...current.issues.filter((issue) => issue !== message)] }));
      throw error;
    }
  }, [coreEndpoint, coreToken]);

  async function flushDirtyDesktopDrafts() {
    const pendingDrafts: Array<{ nodeId: string; revision: string; source: string }> = [];
    if (liveDocument && dirtyNodeIds.has(liveDocument.nodeId)) {
      pendingDrafts.push({ nodeId: liveDocument.nodeId, revision: liveDocument.revision, source: editorSource });
    }
    if (splitDocument && dirtyNodeIds.has(splitDocument.nodeId)) {
      pendingDrafts.push({ nodeId: splitDocument.nodeId, revision: splitDocument.revision, source: splitDocument.source });
    }
    for (const draft of pendingDrafts) {
      await persistDesktopDraft(draft.nodeId, draft.revision, draft.source);
    }
  }

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchCreatesTab(false);
        setExplorerActivity("search");
        setDialog(null);
        window.requestAnimationFrame(() => searchInputRef.current?.focus());
      }
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === "e") {
        event.preventDefault();
        setExplorerActivity("explorer");
        setExplorerMode((current) => current === "hierarchy" ? "contents" : "hierarchy");
      }
      if (event.altKey && event.key === "Home") {
        event.preventDefault();
        setExplorerActivity("explorer");
        setExplorerMode("contents");
        setContentsBrowseLocator(null);
      }
      if (event.key === "Escape") setDialog(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      if (window.weftextDesktop) {
        setDesktopMode(true);
        setCoreState("connecting");
        void window.weftextDesktop.restoreWorkspace()
          .then(async (raw) => {
            const payload = raw as DesktopOpenPayload;
            if (payload.opened && payload.workspace && payload.document) {
              await applyLiveConnection(payload.workspace, payload.document, payload.draftRecovery);
              setSafeMode(Boolean(payload.safeMode));
              setDerivedIndexWarning(payload.searchIndexWarning ?? null);
            } else {
              setCoreState("idle");
              setRestoreError(payload.restoreError ?? "");
              setDialog("core");
            }
          })
          .catch((error: unknown) => {
            setCoreState("error");
            setCoreError(error instanceof Error ? error.message : "无法恢复桌面工作区");
            setDialog("core");
          });
        return;
      }
      const fragment = new URLSearchParams(window.location.hash.replace(/^#/, ""));
      const endpoint = fragment.get("core") ?? "";
      const token = fragment.get("token") ?? "";
      if (!endpoint && !token) return;
      let parsed: URL;
      try {
        parsed = new URL(endpoint);
      } catch {
        setCoreState("error");
        setCoreError("Core 连接地址无效。请从命令行输出的完整链接重新打开原型。");
        return;
      }
      if (parsed.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(parsed.hostname) || !token) {
        setCoreState("error");
        setCoreError("原型只接受带访问令牌的本机 Core 连接。");
        return;
      }
      setCoreEndpoint(parsed.origin);
      setCoreToken(token);
      setCoreState("connecting");
      void Promise.all(["/api/workspace", "/api/document"].map((path) => requestCore(parsed.origin, token, path)))
        .then(async ([workspacePayload, documentPayload]) => {
          const workspace = workspacePayload.workspace as LiveWorkspace;
          const next = documentPayload.document as LiveDocument;
          await applyLiveConnection(workspace, next, workspacePayload.draftRecovery as DraftRecovery | undefined, parsed.origin, token);
          setSafeMode(false);
          setDerivedIndexWarning((workspacePayload.searchIndexWarning as DerivedIndexWarning | null | undefined) ?? null);
        })
        .catch((error: unknown) => {
          setCoreState("error");
          setCoreError(error instanceof Error ? error.message : "无法连接本机 Core");
        });
    }, 0);
    return () => window.clearTimeout(timer);
  }, [applyLiveConnection]);

  useEffect(() => {
    if (!liveWorkspace || !query.trim()) {
      return;
    }
    const requestId = ++searchRequestRef.current;
    const timer = window.setTimeout(() => {
      void requestCore(coreEndpoint, coreToken, `/api/search?q=${encodeURIComponent(query)}`)
        .then((payload) => {
          if (requestId === searchRequestRef.current) {
            setWorkspaceSearch(payload.results as SearchResult[]);
            setDerivedIndexWarning(null);
          }
        })
        .catch((error: unknown) => {
          if (requestId === searchRequestRef.current) {
            setWorkspaceSearch([]);
            setDerivedIndexWarning({
              code: "derived_search_index_unavailable",
              message: error instanceof Error ? error.message : "派生搜索索引暂不可用",
              rebuildRequired: true,
            });
          }
        });
    }, 160);
    return () => window.clearTimeout(timer);
  }, [coreEndpoint, coreToken, liveWorkspace, query]);

  useEffect(() => {
    if (!liveDocument) return;
    const requestId = ++modelRequestRef.current;
    const source = editorSource;
    const nodeId = liveDocument.nodeId;
    let active = true;
    const timer = window.setTimeout(() => {
      if (!active || requestId !== modelRequestRef.current || primaryDocumentNodeRef.current !== nodeId) return;
      setDraftModel(null);
      setDraftModelSource("");
      setDraftModelState("pending");
      setDraftModelError("");
      void requestCore(coreEndpoint, coreToken, "/api/document/model", { source })
        .then((payload) => {
          if (!active || requestId !== modelRequestRef.current || primaryDocumentNodeRef.current !== nodeId) return;
          const model = coreModelFromPayload(payload);
          setDraftModel(model);
          setDraftModelSource(source);
          setDraftModelState("ready");
          setDraftModelError("");
          setPropertyAnalysis((payload.properties as DocumentPropertyAnalysis | undefined) ?? null);
          setPropertyAnalysisSource(source);
        })
        .catch((error: unknown) => {
          if (!active || requestId !== modelRequestRef.current || primaryDocumentNodeRef.current !== nodeId) return;
          setDraftModel(null);
          setDraftModelSource(source);
          setDraftModelState("error");
          setDraftModelError(error instanceof Error ? error.message : "Core 无法解析当前草稿");
          setPropertyAnalysis(null);
          setPropertyAnalysisSource(source);
        });
    }, 120);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [coreEndpoint, coreToken, editorSource, liveDocument]);

  useEffect(() => {
    if (!citationAvailable || !liveDocument) return;
    const requestId = ++citationRequestRef.current;
    const source = editorSource;
    const nodeId = liveDocument.nodeId;
    const timer = window.setTimeout(() => {
      void requestCore(coreEndpoint, coreToken, "/api/citation/analyze", {
        nodeId,
        source,
        styleId: citationStyle,
        locale: citationLocale,
      })
        .then((payload) => {
          if (requestId !== citationRequestRef.current || primaryDocumentNodeRef.current !== nodeId) return;
          if (!isCitationDraftPayload(payload)) throw new Error("Core 返回了无效的引用分析契约");
          setCitationDraft(payload);
          setCitationDraftSource(source);
          setCitationAnalysisError("");
        })
        .catch((error: unknown) => {
          if (requestId !== citationRequestRef.current || primaryDocumentNodeRef.current !== nodeId) return;
          setCitationDraft(null);
          setCitationDraftSource(source);
          setCitationAnalysisError(error instanceof Error ? error.message : "Core 无法分析引用草稿");
        });
    }, 140);
    return () => window.clearTimeout(timer);
  }, [citationAvailable, citationLocale, citationStyle, coreEndpoint, coreToken, editorSource, liveDocument]);

  useEffect(() => {
    if (dialog !== "citation" || !citationQuery.trim() || !citationAvailable) return;
    const requestId = ++citationSearchRequestRef.current;
    const timer = window.setTimeout(() => {
      void requestCore(coreEndpoint, coreToken, `/api/citation/search?q=${encodeURIComponent(citationQuery)}&limit=40`)
        .then((payload) => {
          if (requestId === citationSearchRequestRef.current) {
            setCitationHits(payload.references as CitationReferenceHit[]);
            setCitationSearchIndex(0);
          }
        })
        .catch((error: unknown) => {
          if (requestId === citationSearchRequestRef.current) {
            setCitationHits([]);
            setCitationAnalysisError(error instanceof Error ? error.message : "参考文献检索不可用");
          }
        });
    }, 140);
    return () => window.clearTimeout(timer);
  }, [citationAvailable, citationQuery, coreEndpoint, coreToken, dialog]);

  useEffect(() => {
    const nodeId = navigation.split?.nodeId;
    if (!liveWorkspace || !nodeId || splitDocument?.nodeId === nodeId) return;
    let active = true;
    void requestCore(coreEndpoint, coreToken, `/api/document?nodeId=${encodeURIComponent(nodeId)}&remember=false`)
      .then((payload) => {
        if (!active) return;
        const next = payload.document as LiveDocument;
        const sessionSource = next.recoveryDraft?.stale ? undefined : draftSourcesRef.current[nodeId];
        const recoverySource = next.recoveryDraft && !next.recoveryDraft.stale ? next.recoveryDraft.source : undefined;
        const source = sessionSource ?? recoverySource ?? next.source;
        requireLiveDocumentContract(next, "第二栏文档");
        setSplitDocument({ ...next, source });
        setSplitModel(source === next.source ? next.model : null);
        setSplitModelState(source === next.source ? "ready" : "pending");
        setSplitModelError("");
        rememberDraft(nodeId, source);
        setDirtyNodeIds((current) => {
          const updated = new Set(current);
          if (source !== next.source) updated.add(nodeId);
          else updated.delete(nodeId);
          return updated;
        });
      })
      .catch((error: unknown) => {
        if (active) {
          const message = error instanceof Error ? error.message : "无法恢复第二编辑栏";
          setCoreError(message);
          setSplitModelState("error");
          setSplitModelError(message);
        }
      });
    return () => { active = false; };
  }, [coreEndpoint, coreToken, liveWorkspace, navigation.split?.nodeId, splitDocument?.nodeId]);

  const splitModelSource = splitDocument?.source;
  const splitModelNodeId = splitDocument?.nodeId;
  const splitSessionNodeId = navigation.split?.nodeId;
  useEffect(() => {
    if (splitModelSource === undefined || !splitModelNodeId || !splitSessionNodeId) return;
    const requestId = ++splitModelRequestRef.current;
    const source = splitModelSource;
    const nodeId = splitModelNodeId;
    let active = true;
    const timer = window.setTimeout(() => {
      if (!active || requestId !== splitModelRequestRef.current || splitDocumentNodeRef.current !== nodeId) return;
      setSplitModel(null);
      setSplitModelState("pending");
      setSplitModelError("");
      void requestCore(coreEndpoint, coreToken, "/api/document/model", { source })
        .then((payload) => {
          if (!active || requestId !== splitModelRequestRef.current || splitDocumentNodeRef.current !== nodeId) return;
          setSplitModel(coreModelFromPayload(payload));
          setSplitModelState("ready");
          setSplitModelError("");
        })
        .catch((error: unknown) => {
          if (!active || requestId !== splitModelRequestRef.current || splitDocumentNodeRef.current !== nodeId) return;
          setSplitModel(null);
          setSplitModelState("error");
          setSplitModelError(error instanceof Error ? error.message : "Core 无法解析第二栏草稿");
        });
    }, 120);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [coreEndpoint, coreToken, splitModelNodeId, splitModelSource, splitSessionNodeId]);

  useEffect(() => {
    if (!liveDocument) return;
    let active = true;
    void requestCore(coreEndpoint, coreToken, `/api/annotations?nodeId=${encodeURIComponent(liveDocument.nodeId)}`)
      .then((payload) => {
        if (!isLiveAnnotationStore(payload.annotations)) throw new Error("Core 返回的批注 sidecar 不是 weftext.annotations.v3");
        if (active) setLiveAnnotations(payload.annotations);
      })
      .catch((error: unknown) => {
        if (active) setCoreError(error instanceof Error ? error.message : "无法读取节点批注");
      });
    return () => { active = false; };
  }, [coreEndpoint, coreToken, liveDocument]);

  useEffect(() => {
    if (!desktopMode || !liveDocument || !dirtyNodeIds.has(liveDocument.nodeId)) return;
    const nodeId = liveDocument.nodeId;
    const revision = liveDocument.revision;
    const source = editorSource;
    const timer = window.setTimeout(() => {
      void persistDesktopDraft(nodeId, revision, source).catch(() => setDialog("recovery"));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [desktopMode, dirtyNodeIds, editorSource, liveDocument, persistDesktopDraft]);

  useEffect(() => {
    if (!desktopMode || !splitDocument || !dirtyNodeIds.has(splitDocument.nodeId)) return;
    const timer = window.setTimeout(() => {
      void persistDesktopDraft(splitDocument.nodeId, splitDocument.revision, splitDocument.source).catch(() => setDialog("recovery"));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [desktopMode, dirtyNodeIds, persistDesktopDraft, splitDocument]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 2200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    if (dialog === "search") searchInputRef.current?.focus();
  }, [dialog]);

  useEffect(() => {
    if (findOpen) findInputRef.current?.focus();
  }, [findOpen]);

  useLayoutEffect(() => {
    const editor = writeEditorRef.current;
    if (!editor || view !== "write") return;
    const start = Math.max(0, documentSelection.start - bodyStart);
    const end = Math.max(0, documentSelection.end - bodyStart);
    editor.focus();
    editor.setSelectionRange(Math.min(start, editor.value.length), Math.min(end, editor.value.length));
    editor.scrollTop = documentScrollTop;
  }, [bodyStart, currentSource, documentScrollTop, documentSelection, selectionRestoreToken, view]);

  useEffect(() => {
    function handleDocumentShortcut(event: KeyboardEvent) {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.key.toLocaleLowerCase() === "f") {
        event.preventDefault();
        setFindOpen(true);
        if (view === "read") setView("write");
      } else if (modifier && event.key.toLocaleLowerCase() === "h") {
        event.preventDefault();
        setFindOpen(true);
        setReplaceOpen(true);
        if (view === "read") setView("write");
      } else if (event.key === "Escape" && findOpen) {
        event.preventDefault();
        setFindOpen(false);
      }
    }
    window.addEventListener("keydown", handleDocumentShortcut);
    return () => window.removeEventListener("keydown", handleDocumentShortcut);
  }, [findOpen, view]);

  function rememberDraft(id: string, source: string) {
    draftSourcesRef.current = { ...draftSourcesRef.current, [id]: source };
    setDraftSources(draftSourcesRef.current);
  }

  function recordNavigationVisit(id: string, options: { newTab?: boolean; tabId?: string; history?: boolean } = {}) {
    setNavigation((current) => {
      let tabs = current.tabs;
      let activeTabId = current.activeTabId;
      if (options.newTab) {
        const existing = tabs.find((tab) => tab.nodeId === id);
        if (existing) activeTabId = existing.id;
        else {
          const tab = { id: `tab-${id}-${Date.now()}`, nodeId: id };
          tabs = [...tabs, tab];
          activeTabId = tab.id;
        }
      } else if (options.tabId && tabs.some((tab) => tab.id === options.tabId)) {
        activeTabId = options.tabId;
      } else {
        const existing = tabs.find((tab) => tab.nodeId === id);
        if (existing) activeTabId = existing.id;
        else tabs = tabs.map((tab) => tab.id === activeTabId ? { ...tab, nodeId: id } : tab);
      }
      return {
        ...current,
        tabs,
        activeTabId,
        back: options.history === false || id === selected ? current.back : [...current.back, selected].slice(-100),
        forward: options.history === false || id === selected ? current.forward : [],
        recent: [id, ...current.recent.filter((nodeId) => nodeId !== id)].slice(0, 20),
      };
    });
  }

  function updateCurrentSource(source: string) {
    if (liveDocument) {
      setEditorSource(source);
      setDraftModel(null);
      setDraftModelSource("");
      setDraftModelState("pending");
      setDraftModelError("");
    }
    rememberDraft(selected, source);
    if (liveDocument && source === liveDocument.source) {
      markCurrentSaved();
      setDraftSaveState("idle");
      if (desktopMode) void discardPersistentDraft(selected, false);
    } else {
      setDirtyNodeIds((current) => new Set(current).add(selected));
    }
  }

  function markCurrentSaved() {
    setDirtyNodeIds((current) => {
      const next = new Set(current);
      next.delete(selected);
      return next;
    });
  }

  async function openNode(id: string, options: { refresh?: boolean; newTab?: boolean; tabId?: string; history?: boolean; acceptCommittedSource?: boolean } = {}) {
    metadataRequestRef.current += 1;
    if (structuralContext) {
      setStructuralPlan(null);
      setStructuralContext(null);
      setStructuralDraftScope(null);
      setStructuralDraftScope(null);
    }
    if (id === selected && !options.refresh) {
      recordNavigationVisit(id, options);
      setDialog(null);
      setQuery("");
      setSearchCreatesTab(false);
      return true;
    }
    if (desktopMode && liveDocument && dirtyNodeIds.has(liveDocument.nodeId)) {
      try {
        await persistDesktopDraft(liveDocument.nodeId, liveDocument.revision, editorSource);
      } catch {
        setDialog("recovery");
        return false;
      }
    }
    const requestId = ++nodeRequestRef.current;
    let showRecovery = false;
    persistCurrentEditorState();
    if (options.acceptCommittedSource) {
      const nextDrafts = { ...draftSourcesRef.current };
      delete nextDrafts[id];
      draftSourcesRef.current = nextDrafts;
      setDraftSources(nextDrafts);
    }
    let sourceForRestore = draftSourcesRef.current[id] ?? demo?.sourceFor(id, nodes.find((node) => node.id === id)?.name ?? "未命名节点") ?? "";
    if (liveWorkspace) {
      if (liveDocument && !options.acceptCommittedSource) rememberDraft(liveDocument.nodeId, editorSource);
      try {
        const payload = await requestCore(coreEndpoint, coreToken, `/api/document?nodeId=${encodeURIComponent(id)}`);
        if (requestId !== nodeRequestRef.current) return false;
        const next = payload.document as LiveDocument;
        requireLiveDocumentContract(next);
        const recovered = next.recoveryDraft?.source;
        const nextSource = recovered && !next.recoveryDraft?.stale ? recovered : next.source;
        const sessionSource = next.recoveryDraft?.stale ? undefined : draftSourcesRef.current[id];
        setLiveDocument(next);
        loadMetadataInputs(next.metadata);
        setLiveAnnotations(null);
        const selectedSource = sessionSource ?? nextSource;
        sourceForRestore = selectedSource;
        setEditorSource(selectedSource);
        rememberDraft(id, selectedSource);
        setDirtyNodeIds((current) => {
          const updated = new Set(current);
          if (recovered && !next.recoveryDraft?.stale) updated.add(id);
          else if (!draftSourcesRef.current[id] || draftSourcesRef.current[id] === next.source) updated.delete(id);
          return updated;
        });
        setDraftSaveState(recovered ? "saved" : "idle");
        setDraftModel(next.model);
        setDraftModelSource(selectedSource === next.source ? next.source : "");
        setDraftModelState(selectedSource === next.source ? "ready" : "pending");
        setDraftModelError("");
        setPropertyAnalysis(selectedSource === next.source ? next.properties ?? null : null);
        setPropertyAnalysisSource(selectedSource);
        setCoreError("");
        if (next.recoveryIssue) {
          setDraftRecovery((current) => ({ ...current, issues: [next.recoveryIssue!, ...current.issues] }));
        }
        showRecovery = Boolean(recovered || next.recoveryIssue);
      } catch (error) {
        if (requestId !== nodeRequestRef.current) return false;
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了节点读取");
        setDialog("conflict");
        return false;
      }
    }
    recordNavigationVisit(id, options);
    setSelected(id);
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    const restoredState = editorStatesRef.current[deviceEditorStateKey(workspaceId, id)];
    const restoredStart = Math.min(restoredState?.selectionStart ?? 0, sourceForRestore.length);
    const restoredEnd = Math.min(restoredState?.selectionEnd ?? restoredStart, sourceForRestore.length);
    setView(restoredState?.view ?? view);
    setCursor(sourcePosition(sourceForRestore, restoredStart));
    setDocumentSelection({ start: restoredStart, end: restoredEnd });
    setDocumentScrollTop(Math.max(0, restoredState?.scrollTop ?? 0));
    setSelectionRestoreToken((current) => current + 1);
    setFindQuery("");
    setFindIndex(0);
    setDialog(showRecovery ? "recovery" : null);
    setQuery("");
    setSearchCreatesTab(false);
    return true;
  }

  async function goBack() {
    const target = navigation.back.at(-1);
    if (!target) return;
    const opened = await openNode(target, { history: false });
    if (!opened) return;
    setNavigation((current) => ({
      ...current,
      back: current.back.slice(0, -1),
      forward: [selected, ...current.forward].slice(0, 100),
    }));
  }

  async function goForward() {
    const target = navigation.forward[0];
    if (!target) return;
    const opened = await openNode(target, { history: false });
    if (!opened) return;
    setNavigation((current) => ({
      ...current,
      back: [...current.back, selected].slice(-100),
      forward: current.forward.slice(1),
    }));
  }

  async function closeTab(tabId: string) {
    if (navigation.tabs.length === 1) return;
    const index = navigation.tabs.findIndex((tab) => tab.id === tabId);
    if (index < 0) return;
    const tabs = navigation.tabs.filter((tab) => tab.id !== tabId);
    const next = tabs[Math.min(index, tabs.length - 1)];
    const wasActive = navigation.activeTabId === tabId;
    if (wasActive) {
      const opened = await openNode(next.nodeId, { tabId: next.id, history: false });
      if (!opened) return;
    }
    setNavigation((current) => ({ ...current, tabs, activeTabId: wasActive ? next.id : current.activeTabId }));
  }

  function toggleBookmark() {
    setNavigation((current) => ({
      ...current,
      bookmarks: current.bookmarks.includes(selected)
        ? current.bookmarks.filter((id) => id !== selected)
        : [...current.bookmarks, selected],
    }));
    setToast(navigation.bookmarks.includes(selected) ? "已移除书签" : "已添加书签");
  }

  async function toggleSplit() {
    if (navigation.split) {
      if (desktopMode && splitDocument && dirtyNodeIds.has(splitDocument.nodeId)) {
        try {
          await persistDesktopDraft(splitDocument.nodeId, splitDocument.revision, splitDocument.source);
        } catch {
          setDialog("recovery");
          return;
        }
      }
      setNavigation((current) => ({ ...current, split: null }));
      setSplitDocument(null);
      setSplitModel(null);
      setSplitModelState("idle");
      setSplitModelError("");
      setSplitFindOpen(false);
      if (splitDocument && dirtyNodeIds.has(splitDocument.nodeId)) setToast("第二栏草稿已保留，可重新打开后预览提交");
      return;
    }
    const candidate = navigation.tabs.find((tab) => tab.nodeId !== selected)?.nodeId
      ?? navigation.recent.find((id) => id !== selected)
      ?? nodes.find((node) => node.id !== selected)?.id;
    if (!candidate) {
      setToast("需要另一个可用节点才能打开分屏");
      return;
    }
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    const stored = editorStatesRef.current[deviceEditorStateKey(workspaceId, candidate)];
    setNavigation((current) => ({
      ...current,
      split: {
        nodeId: candidate,
        selectionStart: stored?.selectionStart ?? 0,
        selectionEnd: stored?.selectionEnd ?? 0,
        scrollTop: stored?.scrollTop ?? 0,
        view: stored?.view ?? "read",
      },
    }));
    if (liveWorkspace) {
      try {
        const payload = await requestCore(coreEndpoint, coreToken, `/api/document?nodeId=${encodeURIComponent(candidate)}&remember=false`);
        const next = payload.document as LiveDocument;
        requireLiveDocumentContract(next, "第二栏文档");
        const sessionSource = next.recoveryDraft?.stale ? undefined : draftSourcesRef.current[candidate];
        const recoverySource = next.recoveryDraft && !next.recoveryDraft.stale ? next.recoveryDraft.source : undefined;
        const source = sessionSource ?? recoverySource ?? next.source;
        const document = { ...next, source };
        setSplitDocument(document);
        setSplitModel(source === next.source ? next.model : null);
        setSplitModelState(source === next.source ? "ready" : "pending");
        setSplitModelError("");
        rememberDraft(candidate, source);
        setDirtyNodeIds((current) => {
          const updated = new Set(current);
          if (source !== next.source) updated.add(candidate);
          else updated.delete(candidate);
          return updated;
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : "无法打开分屏节点";
        setCoreError(message);
        setSplitModelState("error");
        setSplitModelError(message);
        setNavigation((current) => ({ ...current, split: null }));
      }
    }
  }

  async function switchSplitNode(nodeId: string) {
    if (!navigation.split) return false;
    if (nodeId === navigation.split.nodeId) return true;
    if (desktopMode && splitDocument && dirtyNodeIds.has(splitDocument.nodeId)) {
      try {
        await persistDesktopDraft(splitDocument.nodeId, splitDocument.revision, splitDocument.source);
      } catch {
        setDialog("recovery");
        return false;
      }
    }
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    const stored = editorStatesRef.current[deviceEditorStateKey(workspaceId, nodeId)];
    setSplitDocument(null);
    setSplitModel(null);
    setSplitModelState("pending");
    setSplitModelError("");
    setNavigation((current) => current.split ? {
      ...current,
      split: {
        nodeId,
        selectionStart: stored?.selectionStart ?? 0,
        selectionEnd: stored?.selectionEnd ?? 0,
        scrollTop: stored?.scrollTop ?? 0,
        view: stored?.view ?? current.split.view,
      },
    } : current);
    return true;
  }

  async function chooseDesktopWorkspace() {
    if (!window.weftextDesktop) return;
    if (dirtyNodeIds.size > 0) {
      try {
        await flushDirtyDesktopDrafts();
      } catch {
        setDialog("recovery");
        return;
      }
    }
    setCoreState("connecting");
    try {
      const raw = await window.weftextDesktop.chooseWorkspace();
      if (!raw) {
        setCoreState(liveWorkspace ? "connected" : "idle");
        return;
      }
      const payload = raw as DesktopOpenPayload;
      if (!payload.opened || !payload.workspace || !payload.document) {
        throw new Error("所选目录没有打开");
      }
      await applyLiveConnection(payload.workspace, payload.document, payload.draftRecovery, coreEndpoint, coreToken);
      setSafeMode(Boolean(payload.safeMode));
      setDerivedIndexWarning(payload.searchIndexWarning ?? null);
      setDialog(null);
      setToast(payload.searchIndexWarning
        ? "工作区已打开；搜索索引暂不可用，可稍后重建"
        : "工作区已打开；会在下次启动时自动恢复");
    } catch (error) {
      setCoreState("error");
      setCoreError(error instanceof Error ? error.message : "无法打开工作区");
      setRestoreError(error instanceof Error ? error.message : "无法打开工作区");
    }
  }

  async function discardPersistentDraft(nodeId: string, restoreDisk: boolean) {
    if (!desktopMode) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/draft/discard", { nodeId });
      setDraftRecovery((payload.draftRecovery as DraftRecovery | undefined) ?? emptyDraftRecovery);
      if (liveDocument?.nodeId === nodeId) {
        const next = { ...liveDocument, recoveryDraft: null };
        setLiveDocument(next);
        setLiveAnnotations(null);
        if (restoreDisk) {
          setEditorSource(next.source);
          rememberDraft(nodeId, next.source);
          markCurrentSaved();
          setDraftModel(next.model);
          setDraftModelSource(next.source);
          setDraftModelState("ready");
          setDraftModelError("");
        }
      }
      setDraftSaveState("idle");
      setDialog(null);
      if (restoreDisk) setToast("已保留磁盘版本并放弃设备草稿");
    } catch (error) {
      const message = error instanceof Error ? error.message : "无法放弃设备草稿";
      setDraftRecovery((current) => ({ ...current, issues: [message, ...current.issues] }));
      setDraftSaveState("error");
      setDialog("recovery");
    }
  }

  function recoverPersistentDraft() {
    const recovered = liveDocument?.recoveryDraft?.source;
    if (!liveDocument || recovered === undefined) return;
    setEditorSource(recovered);
    rememberDraft(liveDocument.nodeId, recovered);
    setDirtyNodeIds((current) => new Set(current).add(liveDocument.nodeId));
    setDraftSaveState("saved");
    setDialog(null);
    setToast(liveDocument.recoveryDraft?.stale ? "已选择恢复草稿；提交前仍会检查当前 revision" : "已恢复设备本地草稿");
  }

  async function refreshDiagnostics() {
    if (!desktopMode || !liveWorkspace) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/diagnostics");
      setDiagnostics(payload.diagnostics as DesktopDiagnostics);
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "无法读取桌面诊断");
      setDialog("conflict");
    }
  }

  async function toggleSafeMode() {
    if (!desktopMode || !liveWorkspace) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/safe-mode", { enabled: !safeMode });
      setSafeMode(Boolean(payload.safeMode));
      setDiagnostics(payload.diagnostics as DesktopDiagnostics);
      setToast(payload.safeMode ? "安全模式已启用；工作区提交已暂停" : "安全模式已关闭；Core 提交已恢复");
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "无法切换安全模式");
      setDialog("conflict");
    }
  }

  function openNodeActionChooser(invocation: WorkspaceActionInvocation) {
    setNodeActionInvocation(Object.freeze({ ...invocation }));
    setDialog("node_actions");
  }

  function currentEditorInvocation(): WorkspaceActionInvocation {
    return Object.freeze({ source: "editor_command", focusedNodeId: followedNodeId });
  }

  function beginNodeAction(action: "create" | "rename" | "move" | "copy" | "trash_node" | "chrono", invocation = nodeActionInvocation) {
    if (!invocation) return;
    let target: FrozenNodeActionTarget;
    try {
      const resolved = resolveWorkspaceActionTarget(action, invocation);
      if (resolved.kind !== "node") throw new Error("节点操作没有解析为明确节点");
      target = resolved;
    } catch (error) {
      setToast(error instanceof Error ? error.message : "无法确定节点操作目标");
      return;
    }
    const targetNode = nodes.find((node) => node.id === target.nodeId);
    if (!targetNode) {
      setToast("该节点已不可用；请从当前工作区重新选择");
      return;
    }
    setNodeActionTarget(target);
    if (action === "create") setNewName("");
    if (action === "rename" || action === "move" || action === "copy") setMoveName(targetNode.name);
    setMoveParent(targetNode.parentId ?? liveWorkspace?.rootNodeId ?? target.nodeId);
    setDialog(({ create: "new", rename: "rename", move: "move", copy: "copy", trash_node: "node_trash", chrono: "chrono" } as const)[action]);
  }

  function beginResourceTrash(invocation = nodeActionInvocation) {
    if (!invocation) return;
    try {
      const target = resolveWorkspaceActionTarget("trash_resource", invocation);
      setNodeActionTarget(target);
      setTrashResourceNames(target.kind === "resource" ? target.resourceName : "");
      setDialog("resource_trash");
    } catch (error) {
      setToast(error instanceof Error ? error.message : "无法确定资源操作目标");
    }
  }

  function createNode() {
    const name = newName.trim();
    const target = nodeActionTarget?.kind === "node" && nodeActionTarget.action === "create" ? nodeActionTarget : null;
    if (!name || !target) return;
    const targetNode = nodes.find((node) => node.id === target.nodeId);
    if (!targetNode) return;
    if (liveWorkspace) {
      void previewWorkspaceAction({ action: "create", parentId: target.nodeId, name }, target);
      return;
    }
    if (!demo) {
      setCoreError("请先打开 Core 工作区再创建节点");
      setDialog("core");
      return;
    }
    let suffix = nodes.length;
    while (nodes.some((node) => node.id === `new-${suffix}`)) suffix += 1;
    const id = `new-${suffix}`;
    setNodes((current) => [...current, { id, name, depth: targetNode.depth + 1, parentId: target.nodeId, kind: "note" }]);
    setCollapsedNodes((current) => {
      const next = new Set(current);
      next.delete(target.nodeId);
      return next;
    });
    setSelected(id);
    setNewName("");
    setDialog(null);
    setToast(`已创建节点“${name}”`);
  }

  function rememberDocumentSelection(start: number, end = start) {
    const next = {
      start: Math.max(0, Math.min(start, currentSource.length)),
      end: Math.max(0, Math.min(end, currentSource.length)),
    };
    setDocumentSelection(next);
    setCursor(sourcePosition(currentSource, next.start));
    persistEditorState(next, documentScrollTop, view);
  }

  function persistEditorState(selection = documentSelection, scrollTop = documentScrollTop, stateView = view) {
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    const key = deviceEditorStateKey(workspaceId, selected);
    editorStatesRef.current = {
      ...editorStatesRef.current,
      [key]: {
        selectionStart: selection.start,
        selectionEnd: selection.end,
        scrollTop: Math.max(0, scrollTop),
        view: stateView,
      },
    };
    writeDeviceEditorStates(editorStatesRef.current);
  }

  function persistCurrentEditorState() {
    persistEditorState(documentSelection, documentScrollTop, view);
  }

  function rememberDocumentScroll(scrollTop: number) {
    setDocumentScrollTop(scrollTop);
    persistEditorState(documentSelection, scrollTop, view);
  }

  function updateSplitSource(source: string) {
    if (!splitDocument) return;
    setSplitDocument({ ...splitDocument, source });
    setSplitModel(null);
    setSplitModelState("pending");
    setSplitModelError("");
    rememberDraft(splitDocument.nodeId, source);
    setDirtyNodeIds((current) => new Set(current).add(splitDocument.nodeId));
  }

  function updateSplitSession(patch: Partial<DeviceEditorState>) {
    setNavigation((current) => current.split ? { ...current, split: { ...current.split, ...patch } } : current);
    const split = navigation.split;
    if (!split) return;
    const next = { ...split, ...patch };
    const workspaceId = liveWorkspace?.rootNodeId ?? initialWorkspaceId;
    editorStatesRef.current = {
      ...editorStatesRef.current,
      [deviceEditorStateKey(workspaceId, split.nodeId)]: {
        selectionStart: next.selectionStart,
        selectionEnd: next.selectionEnd,
        scrollTop: next.scrollTop,
        view: next.view,
      },
    };
    writeDeviceEditorStates(editorStatesRef.current);
  }

  function restoreSplitSelection(start: number, end = start) {
    updateSplitSession({ selectionStart: start, selectionEnd: end });
    setSplitSelectionRestoreToken((current) => current + 1);
  }

  async function requestCoreFormat(source: string, start: number, end: number, command: DocumentFormatCommand) {
    const payload = await requestCore(coreEndpoint, coreToken, "/api/document/format", {
      source,
      start: utf8ByteOffsetAtString(source, start),
      end: utf8ByteOffsetAtString(source, end),
      command,
    });
    return payload.plan as DocumentFormatPlan;
  }

  function formatBaseStillCurrent(nodeId: string, source: string, pane: "primary" | "split") {
    const activeNodeId = pane === "primary" ? primaryDocumentNodeRef.current : splitDocumentNodeRef.current;
    if (activeNodeId === nodeId && draftSourcesRef.current[nodeId] === source) return true;
    setToast("格式命令返回时草稿已经变化；已保留较新的内容，请重试");
    return false;
  }

  async function applySplitInlineFormat(action: InlineFormatAction) {
    if (!splitDocument || !navigation.split) return;
    const nodeId = splitDocument.nodeId;
    const source = splitDocument.source;
    const start = Math.min(navigation.split.selectionStart, source.length);
    const end = Math.min(navigation.split.selectionEnd, source.length);
    if (liveWorkspace) {
      try {
        const plan = await requestCoreFormat(source, start, end, { kind: action });
        if (!formatBaseStillCurrent(nodeId, source, "split")) return;
        updateSplitSource(plan.source);
        restoreSplitSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast("已由 Rust Core 应用 AsciiDoc 格式命令");
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了第二栏格式命令");
        setDialog("conflict");
      }
      return;
    }
    if (!demo) return;
    const fallback = demo.inlineFormat(action);
    if (!fallback) return;
    const [prefix, suffix, placeholder] = fallback;
    const content = splitDocument.source.slice(start, end) || placeholder;
    updateSplitSource(`${splitDocument.source.slice(0, start)}${prefix}${content}${suffix}${splitDocument.source.slice(end)}`);
    restoreSplitSelection(start + prefix.length, start + prefix.length + content.length);
  }

  function handleSplitFormatKeyDown(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "key" | "preventDefault">) {
    if (!(event.ctrlKey || event.metaKey)) return;
    const key = event.key.toLocaleLowerCase();
    const action: InlineFormatAction | null = key === "b" ? "bold" : key === "i" ? "emphasis" : key === "e" ? "inline_code" : null;
    if (!action) return;
    event.preventDefault();
    void applySplitInlineFormat(action);
  }

  async function applySplitHeadingLevel(level: number) {
    if (!splitDocument || !navigation.split) return;
    const nodeId = splitDocument.nodeId;
    const source = splitDocument.source;
    const selection = navigation.split.selectionStart;
    if (liveWorkspace) {
      try {
        const plan = await requestCoreFormat(source, selection, selection, { kind: "heading", level });
        if (!formatBaseStillCurrent(nodeId, source, "split")) return;
        updateSplitSource(plan.source);
        restoreSplitSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了第二栏标题格式");
        setDialog("conflict");
      }
      return;
    }
    const block = splitEditorModel?.blocks.find((candidate) => candidate.kind !== "frontmatter" && candidate.start <= selection && selection <= candidate.end);
    if (!block) {
      setToast("正在等待 Core 确定第二栏语义块范围");
      return;
    }
    if (!demo) return;
    const replacement = demo.headingReplacement(splitDocument.source.slice(block.start, block.end), level);
    updateSplitSource(`${splitDocument.source.slice(0, block.start)}${replacement}${splitDocument.source.slice(block.end)}`);
    restoreSplitSelection(block.start, block.start + replacement.length);
  }

  async function applySplitBlockFormat(action: BlockFormatAction) {
    if (!splitDocument || !navigation.split) return;
    const nodeId = splitDocument.nodeId;
    const source = splitDocument.source;
    const selection = navigation.split.selectionStart;
    if (liveWorkspace) {
      try {
        const plan = await requestCoreFormat(source, selection, selection, { kind: action === "code" ? "code_block" : action });
        if (!formatBaseStillCurrent(nodeId, source, "split")) return;
        updateSplitSource(plan.source);
        restoreSplitSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了第二栏块格式");
        setDialog("conflict");
      }
      return;
    }
    const block = splitEditorModel?.blocks.find((candidate) => candidate.kind !== "frontmatter" && candidate.start <= selection && selection <= candidate.end);
    if (!block) {
      setToast("正在等待 Core 确定第二栏语义块范围");
      return;
    }
    const original = splitDocument.source.slice(block.start, block.end);
    if (!demo) return;
    const replacement = demo.blockReplacement(original, action, demo.lineEnding(splitDocument.source, block.start));
    updateSplitSource(`${splitDocument.source.slice(0, block.start)}${replacement}${splitDocument.source.slice(block.end)}`);
    restoreSplitSelection(block.start, block.start + replacement.length);
  }

  async function insertSplitTable() {
    if (!splitDocument || !navigation.split) return;
    const nodeId = splitDocument.nodeId;
    const source = splitDocument.source;
    const start = Math.min(navigation.split.selectionStart, source.length);
    const end = Math.min(navigation.split.selectionEnd, source.length);
    if (liveWorkspace) {
      try {
        const plan = await requestCoreFormat(source, start, end, { kind: "table_insert" });
        if (!formatBaseStillCurrent(nodeId, source, "split")) return;
        updateSplitSource(plan.source);
        restoreSplitSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了第二栏表格插入");
        setDialog("conflict");
      }
      return;
    }
    if (!demo) return;
    const ending = demo.lineEnding(splitDocument.source, start);
    const table = `|===${ending}|列 1 |列 2${ending}| |${ending}|===`;
    updateSplitSource(`${splitDocument.source.slice(0, start)}${table}${splitDocument.source.slice(end)}`);
    restoreSplitSelection(start, start + table.length);
  }

  async function extendSplitTable(operation: "row" | "column") {
    if (!splitDocument || !navigation.split) return;
    const nodeId = splitDocument.nodeId;
    const source = splitDocument.source;
    const cursor = navigation.split.selectionStart;
    if (liveWorkspace) {
      try {
        const plan = await requestCoreFormat(source, cursor, cursor, { kind: operation === "row" ? "table_add_row" : "table_add_column" });
        if (!formatBaseStillCurrent(nodeId, source, "split")) return;
        updateSplitSource(plan.source);
        restoreSplitSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了第二栏表格命令");
        setDialog("conflict");
      }
      return;
    }
    const table = splitEditorModel?.blocks.find((block) => block.kind === "table" && block.start <= cursor && cursor <= block.end);
    const result = table && demo ? demo.extendTable(splitDocument.source, table, cursor, operation) : null;
    if (!result) {
      setToast("正在等待 Core 确定第二栏表格块范围");
      return;
    }
    updateSplitSource(result.source);
    restoreSplitSelection(result.cursor);
  }

  function navigateSplitMatch(index: number) {
    if (!splitMatches.length || !navigation.split) return;
    const nextIndex = (index + splitMatches.length) % splitMatches.length;
    const match = splitMatches[nextIndex];
    setSplitFindIndex(nextIndex);
    if (navigation.split.view === "read") updateSplitSession({ view: "write" });
    restoreSplitSelection(match.start, match.end);
  }

  function replaceSplitMatch(all: boolean) {
    if (!splitDocument || !splitMatches.length) return;
    if (all) {
      let source = splitDocument.source;
      for (const match of [...splitMatches].reverse()) {
        source = `${source.slice(0, match.start)}${splitReplaceText}${source.slice(match.end)}`;
      }
      updateSplitSource(source);
      restoreSplitSelection(splitMatches[0].start, splitMatches[0].start + splitReplaceText.length);
      setSplitFindIndex(0);
      return;
    }
    const match = splitMatches[effectiveSplitFindIndex];
    const source = `${splitDocument.source.slice(0, match.start)}${splitReplaceText}${splitDocument.source.slice(match.end)}`;
    updateSplitSource(source);
    restoreSplitSelection(match.start, match.start + splitReplaceText.length);
  }

  async function applyInlineFormat(action: InlineFormatAction) {
    const nodeId = liveDocument?.nodeId ?? selected;
    const source = currentSource;
    const start = Math.min(documentSelection.start, source.length);
    const end = Math.min(documentSelection.end, source.length);
    if (liveDocument) {
      try {
        const plan = await requestCoreFormat(source, start, end, { kind: action });
        if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
        updateCurrentSource(plan.source);
        restoreDocumentSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast(`已由 Rust Core 应用 ${activeProfileName} 格式命令；草稿尚未提交`);
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了格式命令");
        setDialog("conflict");
      }
      return;
    }
    const demoWorkspace = demo;
    if (!demoWorkspace) return;
    const fallback = demoWorkspace.inlineFormat(action);
    if (!fallback) return;
    const [prefix, suffix, placeholder] = fallback;
    const selectedText = currentSource.slice(start, end);
    const content = selectedText || placeholder;
    const replacement = `${prefix}${content}${suffix}`;
    const nextSource = `${currentSource.slice(0, start)}${replacement}${currentSource.slice(end)}`;
    updateCurrentSource(nextSource);
    restoreDocumentSelection(start + prefix.length, start + prefix.length + content.length);
    setToast(demoWorkspace.messages.inlineFormatted);
  }

  async function applyHeadingLevel(level: number) {
    if (liveDocument) {
      const nodeId = liveDocument.nodeId;
      const source = currentSource;
      try {
        const plan = await requestCoreFormat(source, documentSelection.start, documentSelection.start, { kind: "heading", level });
        if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
        updateCurrentSource(plan.source);
        restoreDocumentSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast(`已由 Rust Core 按 ${activeProfileName} 规则设为 H${level}`);
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了标题格式");
        setDialog("conflict");
      }
      return;
    }
    const block = activeEditorModel?.blocks.find((candidate) => candidate.kind !== "frontmatter" && candidate.start <= documentSelection.start && documentSelection.start <= candidate.end);
    if (!block) {
      setToast("正在等待 Core 确定当前语义块范围");
      return;
    }
    if (!demo) return;
    const replacement = demo.headingReplacement(currentSource.slice(block.start, block.end), level);
    updateCurrentSource(`${currentSource.slice(0, block.start)}${replacement}${currentSource.slice(block.end)}`);
    restoreDocumentSelection(block.start, block.start + replacement.length);
    setToast(`已按 Core 块范围设为 H${level}`);
  }

  async function applyBlockFormat(action: BlockFormatAction) {
    if (liveDocument) {
      const nodeId = liveDocument.nodeId;
      const source = currentSource;
      try {
        const plan = await requestCoreFormat(source, documentSelection.start, documentSelection.start, { kind: action === "code" ? "code_block" : action });
        if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
        updateCurrentSource(plan.source);
        restoreDocumentSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast(`已由 Rust Core 按 ${activeProfileName} 语义块规则更新草稿`);
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了块格式命令");
        setDialog("conflict");
      }
      return;
    }
    const block = activeEditorModel?.blocks.find((candidate) => candidate.kind !== "frontmatter" && candidate.start <= documentSelection.start && documentSelection.start <= candidate.end);
    if (!block) {
      setToast("正在等待 Core 确定当前语义块范围");
      return;
    }
    if (!demo) return;
    const lineEnding = demo.lineEnding(currentSource, block.start);
    const original = currentSource.slice(block.start, block.end);
    const replacement = demo.blockReplacement(original, action, lineEnding);
    const nextSource = `${currentSource.slice(0, block.start)}${replacement}${currentSource.slice(block.end)}`;
    updateCurrentSource(nextSource);
    restoreDocumentSelection(block.start, block.start + replacement.length);
    setToast(demo.messages.blockFormatted);
  }

  function handleFormatKeyDown(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "key" | "preventDefault">) {
    if (!(event.ctrlKey || event.metaKey)) return;
    const key = event.key.toLocaleLowerCase();
    const action: InlineFormatAction | null = key === "b" ? "bold" : key === "i" ? "emphasis" : key === "e" ? "inline_code" : null;
    if (!action) return;
    event.preventDefault();
    void applyInlineFormat(action);
  }

  function restoreDocumentSelection(start: number, end = start) {
    rememberDocumentSelection(start, end);
    setSelectionRestoreToken((current) => current + 1);
  }

  function navigateDocumentMatch(index: number) {
    if (!documentMatches.length) return;
    const nextIndex = (index + documentMatches.length) % documentMatches.length;
    const match = documentMatches[nextIndex];
    setFindIndex(nextIndex);
    setFindNavigated(true);
    if (view === "read") setView("write");
    restoreDocumentSelection(match.start, match.end);
  }

  function replaceCurrentMatch() {
    const match = documentMatches[effectiveFindIndex];
    if (!match) return;
    const nextSource = `${currentSource.slice(0, match.start)}${replaceText}${currentSource.slice(match.end)}`;
    updateCurrentSource(nextSource);
    setDocumentSelection({ start: match.start, end: match.start + replaceText.length });
    setCursor(sourcePosition(nextSource, match.start));
    setSelectionRestoreToken((current) => current + 1);
    setToast("已替换当前匹配；草稿尚未提交");
  }

  function replaceAllMatches() {
    if (!documentMatches.length) return;
    let nextSource = currentSource;
    for (const match of [...documentMatches].reverse()) {
      nextSource = `${nextSource.slice(0, match.start)}${replaceText}${nextSource.slice(match.end)}`;
    }
    updateCurrentSource(nextSource);
    setFindIndex(0);
    setFindNavigated(false);
    setDocumentSelection({ start: documentMatches[0].start, end: documentMatches[0].start + replaceText.length });
    setCursor(sourcePosition(nextSource, documentMatches[0].start));
    setSelectionRestoreToken((current) => current + 1);
    setToast(`已替换 ${documentMatches.length} 处；草稿尚未提交`);
  }

  function navigateHeading(heading: DocumentHeading, index: number) {
    rememberDocumentSelection(heading.start);
    if (view === "read") {
      window.requestAnimationFrame(() => {
        documentSurfaceRef.current?.querySelectorAll("h1, h2, h3, h4, h5, h6")[index]?.scrollIntoView({ block: "start" });
      });
    } else {
      restoreDocumentSelection(heading.start);
    }
  }

  function chooseView(next: ViewMode) {
    setView(next);
    persistEditorState(documentSelection, documentScrollTop, next);
    if (next !== "read") setSelectionRestoreToken((current) => current + 1);
    setToast(`已切换到${{ write: "写作", source: "源码", read: "阅读" }[next]}视图`);
  }

  function currentDirtyDraftNodeIds() {
    return new Set([
      ...dirtyNodeIdsRef.current,
      ...draftRecoveryRef.current.drafts.map((draft) => draft.nodeId),
    ]);
  }

  function acceptStructuralPreview(plan: StructuralPlan, context: StructuralContext) {
    if (!Array.isArray(plan.targetNodeIds) || !Array.isArray(plan.draftSensitiveNodeIds)
      || !Array.isArray(plan.identityMap) || !("capturedTarget" in plan)
      || !("scopeSummary" in plan)) {
      throw new Error("Core 事务预览缺少闭合的目标、身份或草稿范围证据");
    }
    const sortedTargets = [...plan.targetNodeIds].sort();
    if (new Set(plan.targetNodeIds).size !== plan.targetNodeIds.length
      || sortedTargets.some((nodeId, index) => nodeId !== plan.targetNodeIds[index])) {
      throw new Error("Core 事务预览返回了非规范的目标 UUID 集合");
    }
    const target = context?.kind === "node_action" ? context.target : context?.kind === "trash" ? context.target : null;
    const expectedResolution = target
      ? context?.kind === "trash" && context.purpose === "resource_trash" && target.kind === "node"
        ? "caller_explicit"
        : targetResolution(target)
      : null;
    if (expectedResolution && plan.capturedTarget && plan.capturedTarget.resolvedBy !== expectedResolution) {
      throw new Error("Core 事务预览的动作目标来源与触发位置不一致");
    }
    if (target?.kind === "node") {
      const resourceBatch = context?.kind === "trash" && context.purpose === "resource_trash";
      const targetMatches = resourceBatch
        ? plan.targetNodeIds.includes(target.nodeId)
          && (!plan.capturedTarget || (plan.capturedTarget.kind === "owned_resource" && plan.capturedTarget.ownerNodeId === target.nodeId))
        : plan.targetNodeIds.includes(target.nodeId)
          && plan.capturedTarget?.kind === "node"
          && plan.capturedTarget.nodeId === target.nodeId;
      if (!targetMatches) {
        throw new Error("Core 事务预览与触发时固定的节点 UUID 不一致");
      }
    } else if (target?.kind === "trash_item") {
      if (plan.capturedTarget?.kind !== "trash_item"
        || plan.capturedTarget.trashItemId !== target.trashItemId) {
        throw new Error("Core 事务预览与触发时固定的废纸篓条目不一致");
      }
    } else if (target?.kind === "resource") {
      if (plan.capturedTarget?.kind !== "owned_resource"
        || plan.capturedTarget.ownerNodeId !== target.ownerNodeId
        || plan.capturedTarget.name !== target.resourceName) {
        throw new Error("Core 事务预览与触发时固定的节点资源不一致");
      }
    }
    const draftScope = freezeCoreReviewedDraftScope({
      draftSensitiveNodeIds: plan.draftSensitiveNodeIds,
    });
    const conflicts = conflictingDirtyNodeIds(draftScope, currentDirtyDraftNodeIds());
    if (conflicts.length) {
      setStructuralPlan(null);
      setStructuralContext(null);
      setStructuralDraftScope(null);
      setCoreError(`操作范围命中 ${conflicts.length} 个未保存草稿；请保存或明确放弃后重新预览`);
      setDialog("conflict");
      return false;
    }
    setStructuralPlan(plan);
    setStructuralContext(context);
    setStructuralDraftScope(draftScope);
    setCoreError("");
    setDialog("structure");
    return true;
  }

  async function previewWorkspaceAction(action: Record<string, string>, target?: FrozenNodeActionTarget) {
    metadataRequestRef.current += 1;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/workspace/action/preview", {
        ...action,
        ...(target ? { resolvedBy: targetResolution(target) } : {}),
      });
      acceptStructuralPreview(
        payload.plan as StructuralPlan,
        target ? { kind: "node_action", target, label: WORKSPACE_ACTION_REGISTRY[target.action].label } : null,
      );
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成事务预览");
      setDialog("conflict");
    }
  }

  function trashPreviewReady(allowLegacyMigration = false) {
    if (!liveWorkspace) return false;
    if (safeMode) {
      setToast("安全模式已启用；Trash 事务预览已暂停");
      return false;
    }
    if (liveWorkspace.trashReconciliation?.required) {
      setToast("Trash 需要协调；当前仅可只读查看诊断");
      return false;
    }
    if (liveWorkspace.trashLegacyMigrationRequired && !allowLegacyMigration) {
      setToast("旧 Trash 格式必须先完成工作区外快照支持的显式迁移；当前仅可只读查看");
      return false;
    }
    return true;
  }

  async function previewNodeTrash(target: FrozenNodeActionTarget) {
    if (!trashPreviewReady() || !liveWorkspace) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/trash/node/preview", {
        nodeId: target.nodeId,
        baseWorkspaceRevision: liveWorkspace.revision,
        trashedAt: new Date().toISOString(),
        resolvedBy: targetResolution(target),
      });
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "trash",
        purpose: "node_trash",
        itemIds: [],
        target,
      });
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成节点 Trash 预览");
      setDialog("conflict");
    }
  }

  async function previewTrashResources() {
    const target = nodeActionTarget;
    if (!trashPreviewReady() || !liveWorkspace || !target) return;
    const ownerNodeId = target.kind === "resource" ? target.ownerNodeId : target.kind === "node" ? target.nodeId : null;
    if (!ownerNodeId) return;
    const names = trashResourceNames.split(/\r?\n/u).map((name) => name.trim()).filter(Boolean);
    const folded = new Set(names.map((name) => name.toLocaleLowerCase()));
    if (!names.length || folded.size !== names.length) {
      setToast("请输入一个或多个不重复的节点资源名，每行一个");
      return;
    }
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/trash/resources/preview", {
        baseWorkspaceRevision: liveWorkspace.revision,
        trashedAt: new Date().toISOString(),
        resources: names.map((name) => ({ ownerNodeId, name })),
        resolvedBy: target.kind === "resource" ? targetResolution(target) : "caller_explicit",
      });
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "trash",
        purpose: "resource_trash",
        itemIds: [],
        target,
      });
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成资源 Trash 批量预览");
      setDialog("conflict");
    }
  }

  async function previewLegacyTrashMigration() {
    if (!trashPreviewReady(true) || !liveWorkspace?.trashLegacyMigrationRequired) return;
    if (!desktopMode || typeof window.weftextDesktop?.chooseBackupDirectory !== "function") {
      setToast("旧 Trash 迁移必须先由 Desktop 或 CLI 选择工作区外快照目录");
      return;
    }
    try {
      const backupParent = await window.weftextDesktop.chooseBackupDirectory("backup_parent");
      if (!backupParent) return;
      const payload = await requestCore(coreEndpoint, coreToken, "/api/trash/migrate-legacy/preview", {
        baseWorkspaceRevision: liveWorkspace.revision,
        trashedAt: new Date().toISOString(),
        backupParentCapability: backupParent.capability,
      });
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "trash",
        purpose: "migration",
        itemIds: [],
      });
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成旧 Trash 显式迁移预览");
      setDialog("conflict");
    }
  }

  function openTrashItem(item: TrashItemSummary) {
    const invocation = Object.freeze({ source: "trash_item_row", trashItemId: item.manifest.trashItemId } as const);
    setNodeActionInvocation(invocation);
    setSelectedTrashItemId(item.manifest.trashItemId);
    setDialog("trash_item");
  }

  function openTrashRestore(item: TrashItemSummary) {
    const target = resolveWorkspaceActionTarget("restore_item", {
      source: "trash_item_row",
      trashItemId: item.manifest.trashItemId,
    });
    const defaultMode = item.restore.originalAvailable
      ? "original"
      : item.restore.withAncestorsAvailable
        ? "with_ancestors"
        : "existing_target";
    setSelectedTrashItemId(item.manifest.trashItemId);
    setNodeActionTarget(target);
    setTrashRestoreMode(defaultMode);
    setTrashRestoreTarget("");
    setTrashRestoreName(item.manifest.originalName);
    setDialog("trash_restore");
  }

  function openTrashPermanentDelete(item: TrashItemSummary) {
    const target = resolveWorkspaceActionTarget("permanently_delete_item", {
      source: "trash_item_row",
      trashItemId: item.manifest.trashItemId,
    });
    setSelectedTrashItemId(item.manifest.trashItemId);
    setNodeActionTarget(target);
    setPermanentDeleteConfirmed(false);
    setDialog("trash_permanent");
  }

  async function previewTrashRestore() {
    const target = nodeActionTarget?.kind === "trash_item" && nodeActionTarget.action === "restore_item" ? nodeActionTarget : null;
    if (!selectedTrashItem || !target || selectedTrashItem.manifest.trashItemId !== target.trashItemId || !trashPreviewReady() || !liveWorkspace) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/trash/restore/preview", {
        trashItemId: target.trashItemId,
        baseWorkspaceRevision: liveWorkspace.revision,
        mode: trashRestoreMode,
        resolvedBy: targetResolution(target),
        ...(trashRestoreMode === "existing_target" ? {
          targetNodeId: trashRestoreTarget,
          name: trashRestoreName,
        } : {}),
      });
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "trash",
        purpose: "restore",
        itemIds: [target.trashItemId],
        target,
      });
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成 Trash 恢复预览");
      setDialog("conflict");
    }
  }

  async function previewTrashPermanentDelete() {
    const target = nodeActionTarget?.kind === "trash_item" && nodeActionTarget.action === "permanently_delete_item" ? nodeActionTarget : null;
    if (!selectedTrashItem || !target || selectedTrashItem.manifest.trashItemId !== target.trashItemId || !trashPreviewReady() || !liveWorkspace) return;
    const manifest = selectedTrashItem.manifest;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/trash/permanent-delete/preview", {
        baseWorkspaceRevision: liveWorkspace.revision,
        resolvedBy: targetResolution(target),
        items: [{
          trashItemId: manifest.trashItemId,
          payloadSha256: trashPayloadSha256(manifest),
          payloadByteLength: trashPayloadByteLength(manifest),
        }],
      });
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "trash",
        purpose: "permanent_delete",
        itemIds: [manifest.trashItemId],
        target,
      });
      setPermanentDeleteConfirmed(false);
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了永久删除预览");
      setDialog("conflict");
    }
  }

  async function previewNodeMetadata(intent: Record<string, unknown>, summary: string) {
    const document = liveDocumentRef.current;
    if (!document || !liveWorkspace || !document.metadata) {
      setToast("当前 Core 文档没有可用的节点元数据投影");
      return;
    }
    const controlledSource = draftSourcesRef.current[document.nodeId] ?? document.source;
    if (controlledSource !== document.source || dirtyNodeIds.has(document.nodeId)) {
      setToast("请先保存或放弃当前文档草稿，再修改节点系统元数据");
      return;
    }
    const context = { nodeId: document.nodeId, revision: document.revision };
    const requestId = ++metadataRequestRef.current;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/node/metadata/preview", {
        ...intent,
        nodeId: context.nodeId,
        revision: context.revision,
      });
      const current = liveDocumentRef.current;
      const currentSource = current ? draftSourcesRef.current[current.nodeId] ?? current.source : null;
      if (requestId !== metadataRequestRef.current
        || current?.nodeId !== context.nodeId
        || current.revision !== context.revision
        || currentSource !== current.source) return;
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "node_metadata",
        ...context,
        summary,
      });
    } catch (error) {
      const current = liveDocumentRef.current;
      const currentSource = current ? draftSourcesRef.current[current.nodeId] ?? current.source : null;
      if (requestId !== metadataRequestRef.current
        || current?.nodeId !== context.nodeId
        || current.revision !== context.revision
        || currentSource !== current.source) return;
      setCoreError(error instanceof Error ? error.message : "Core 无法生成节点元数据预览");
      setDialog("conflict");
    }
  }

  async function commitWorkspaceAction() {
    if (!structuralPlan || !structuralDraftScope) return;
    const committedContext = structuralContext;
    if (safeMode) {
      setToast("安全模式已启用；工作区事务不会提交");
      return;
    }
    const draftConflicts = conflictingDirtyNodeIds(
      structuralDraftScope,
      currentDirtyDraftNodeIds(),
    );
    if (draftConflicts.length) {
      setStructuralPlan(null);
      setStructuralContext(null);
      setStructuralDraftScope(null);
      setCoreError(`预览后有 ${draftConflicts.length} 个受影响节点变为未保存草稿；请保存或明确放弃后重新预览`);
      setDialog("conflict");
      return;
    }
    if (committedContext?.kind === "trash") {
      if (liveWorkspace?.trashReconciliation?.required
        || (liveWorkspace?.trashLegacyMigrationRequired && committedContext.purpose !== "migration")) {
        setStructuralPlan(null);
        setStructuralContext(null);
        setStructuralDraftScope(null);
        setCoreError("Trash authority 已进入只读状态；当前预览已失效，请先完成迁移或协调");
        setDialog("conflict");
        return;
      }
    }
    if (structuralContext?.kind === "node_metadata") {
      const current = liveDocumentRef.current;
      const controlledSource = current ? draftSourcesRef.current[current.nodeId] ?? current.source : null;
      if (current?.nodeId !== structuralContext.nodeId
        || current.revision !== structuralContext.revision
        || controlledSource !== current.source) {
        setStructuralPlan(null);
        setStructuralContext(null);
        setStructuralDraftScope(null);
        setCoreError("节点或文档 revision 已变化；元数据预览已失效，请重新预览");
        setDialog("conflict");
        return;
      }
    }
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/workspace/action/commit", { planId: structuralPlan.planId });
      const workspace = payload.workspace as LiveWorkspace;
      setLiveWorkspace(workspace);
      setNodes(workspaceTreeNodes(workspace));
      const pathChange = chronoTargetName
        ? structuralPlan.pathChanges.find((change) => change.newPath === chronoTargetName || change.newPath.endsWith(`/${chronoTargetName}`))
        : structuralPlan.pathChanges[0];
      const nextSelected = pathChange?.nodeId ?? selected;
      setStructuralPlan(null);
      setStructuralContext(null);
      setStructuralDraftScope(null);
      setChronoTargetName("");
      setNewName("");
      setCoreError("");
      setDialog(null);
      setToast(payload.searchIndexWarning
        ? "工作区事务已提交；派生搜索索引刷新失败，稍后可安全重建（请勿重试事务）"
        : "Rust Core 已提交工作区事务");
      if (workspace.nodes.some((node) => node.id === nextSelected)) {
        await openNode(nextSelected, {
          refresh: Boolean(committedContext),
          acceptCommittedSource: Boolean(committedContext),
        });
      } else {
        await openNode(workspace.rootNodeId);
      }
    } catch (error) {
      setStructuralPlan(null);
      setStructuralContext(null);
      setStructuralDraftScope(null);
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了事务提交");
      setDialog("conflict");
    }
  }

  function openCitationDialog() {
    if (!citationAvailable || !liveDocument || !liveWorkspace) {
      setToast("引用功能只在已连接的 Weftext AsciiDoc 工作区可用");
      return;
    }
    setCitationAnalysisError("");
    const start = new TextEncoder().encode(currentSource.slice(0, documentSelection.start)).length;
    const end = new TextEncoder().encode(currentSource.slice(0, documentSelection.end)).length;
    const containsSelection = (range: CitationRange) => start === end ? start >= range.start && start <= range.end : start === range.start && end === range.end;
    const cluster = activeCitationDraft?.analysis.clusters?.find((candidate) => containsSelection(candidate.range));
    const nocite = activeCitationDraft?.analysis.nocites?.find((candidate) => containsSelection(candidate.range));
    const bibliography = activeCitationDraft?.analysis.bibliography;
    if (cluster) {
      setCitationEditRange(cluster.range);
      setCitationAction("citation");
      setCitationForm(cluster.form);
      setCitationItems(cluster.items.map((item) => ({
        referenceNodeId: item.reference.nodeId,
        key: item.reference.citationData.key,
        title: item.reference.citationData.title,
        label: item.locator ? item.label : null,
        locator: item.locator,
        prefix: item.prefix,
        suffix: item.suffix,
      })));
    } else if (nocite) {
      setCitationEditRange(nocite.range);
      setCitationAction("nocite");
      setCitationItems(nocite.references.map((reference) => ({ referenceNodeId: reference.nodeId, key: reference.citationData.key, title: reference.citationData.title, label: null, locator: null, prefix: null, suffix: null })));
    } else if (bibliography && containsSelection(bibliography.range)) {
      setCitationEditRange(bibliography.range);
      setCitationAction("bibliography");
      setBibliographyInclusion(bibliography.inclusion);
      setCitationItems([]);
    } else {
      setCitationEditRange(null);
      setCitationAction("citation");
      setCitationForm("parenthetical");
      setCitationItems([]);
    }
    setCitationSelectedReference(null);
    setCitationQuery("");
    setCitationHits([]);
    setCitationSearchIndex(0);
    setCitationLocator("");
    setCitationPrefix("");
    setCitationSuffix("");
    setDialog("citation");
  }

  function focusCitationRange(range: CitationRange | null) {
    if (!range) {
      chooseView("source");
      return;
    }
    const start = stringOffsetAtUtf8Byte(currentSource, range.start);
    const end = stringOffsetAtUtf8Byte(currentSource, range.end);
    chooseView("source");
    restoreDocumentSelection(start, end);
  }

  async function applyCitationMacro() {
    if (!liveDocument) return;
    const source = currentSource;
    const nodeId = liveDocument.nodeId;
    const start = new TextEncoder().encode(source.slice(0, documentSelection.start)).length;
    const target = citationEditRange ? { kind: "replace", range: citationEditRange } : { kind: "insert", offset: start };
    const selectedReference = citationHits.find((hit) => hit.nodeId === citationSelectedReference && hit.selectable);
    const pendingItem = selectedReference ? {
      referenceNodeId: selectedReference.nodeId,
      key: selectedReference.key,
      title: selectedReference.title,
      label: citationLocator.trim() ? citationLabel : null,
      locator: citationLocator.trim() || null,
      prefix: citationPrefix.trim() || null,
      suffix: citationSuffix.trim() || null,
    } satisfies CitationIntentItem : null;
    const items = pendingItem ? [...citationItems, pendingItem] : citationItems;
    if (citationAction !== "bibliography" && !items.length) {
      setCitationAnalysisError("请先选择一个可用的参考文献");
      return;
    }
    if (citationAction === "citation" && citationForm === "narrative" && items.length !== 1) {
      setCitationAnalysisError("叙述式引用按规范只能包含一个参考文献项");
      return;
    }
    const intentItems = items.map(({ referenceNodeId, label, locator, prefix, suffix }) => ({ referenceNodeId, label, locator, prefix, suffix }));
    const intent = citationAction === "citation"
      ? { kind: "citation", cluster: { form: citationForm, items: intentItems } }
      : citationAction === "nocite"
        ? { kind: "no_cite", referenceNodeIds: items.map((item) => item.referenceNodeId) }
        : { kind: "bibliography", inclusion: bibliographyInclusion };
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/citation/macro-edit-preview", { nodeId, source, target, intent });
      if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
      const plan = payload.plan as { proposedSource: string; edit: { start: number; end: number; replacement: string } };
      updateCurrentSource(plan.proposedSource);
      const selectionStart = stringOffsetAtUtf8Byte(plan.proposedSource, plan.edit.start);
      const replacementLength = new TextEncoder().encode(plan.edit.replacement).length;
      restoreDocumentSelection(selectionStart, stringOffsetAtUtf8Byte(plan.proposedSource, plan.edit.start + replacementLength));
      setDialog(null);
      setCitationQuery("");
      setCitationHits([]);
      setCitationSelectedReference(null);
      setCitationItems([]);
      setCitationEditRange(null);
      setToast("Core 已把 UUID 选择转换为当前引用键并应用到草稿；仍需保存预览");
    } catch (error) {
      setCitationAnalysisError(error instanceof Error ? error.message : "Core 拒绝了引用编辑");
    }
  }

  function addCitationItem() {
    const selectedReference = citationHits.find((hit) => hit.nodeId === citationSelectedReference && hit.selectable);
    if (!selectedReference) return;
    if (citationAction === "citation" && citationForm === "narrative" && citationItems.length > 0) {
      setCitationAnalysisError("叙述式引用按规范只能包含一个参考文献项");
      return;
    }
    setCitationItems((current) => [...current, {
      referenceNodeId: selectedReference.nodeId,
      key: selectedReference.key,
      title: selectedReference.title,
      label: citationLocator.trim() ? citationLabel : null,
      locator: citationLocator.trim() || null,
      prefix: citationPrefix.trim() || null,
      suffix: citationSuffix.trim() || null,
    }]);
    setCitationSelectedReference(null);
    setCitationQuery("");
    setCitationHits([]);
    setCitationSearchIndex(0);
    setCitationLocator("");
    setCitationPrefix("");
    setCitationSuffix("");
    setCitationAnalysisError("");
  }

  function moveCitationItem(index: number, direction: -1 | 1) {
    setCitationItems((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }

  function openPropertyEditor(key = "", value = "") {
    setPropertyKey(key);
    setPropertyValue(value);
    setPropertyExisting(Boolean(key));
    setDialog("property");
  }

  async function applyPropertyPatch(remove = false) {
    if (!liveWorkspace) {
      setToast("连接 Desktop 或本机 Core 后才能执行无损属性补丁");
      return;
    }
    const key = propertyKey.trim();
    if (!key || (!remove && !propertyValue.trim())) return;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/document/property", {
        source: currentSource,
        key,
        value: propertyValue,
        remove,
        nodeId: liveDocument?.nodeId,
        revision: liveDocument?.revision,
      });
      updateCurrentSource(payload.source as string);
      setDialog(null);
      setToast(remove ? `已从草稿移除属性 ${key}` : `已窄范围更新属性 ${key}；草稿尚未提交`);
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了用户属性补丁");
      setDialog("conflict");
    }
  }

  async function applyPortableIcon(icon: { value: string; glyph: string; kind: "emoji" | "built_in" } | null) {
    if (!liveDocument) {
      setToast("连接 Desktop 或本机 Core 后才能设置便携图标");
      return;
    }
    setIconQuery("");
    await previewNodeMetadata(
      icon ? { action: "icon", icon: icon.value } : { action: "icon", remove: true },
      icon ? `把图标设为 ${icon.value}` : "清除便携节点图标",
    );
  }

  async function previewImageImport(file: File) {
    if (!liveWorkspace || !liveDocument) {
      setToast("连接 Desktop 或本机 Core 后才能导入节点资源");
      return;
    }
    if (!file.type.startsWith("image/")) {
      setToast("第一版资源插入只接受图片文件");
      return;
    }
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const payload = await requestCore(coreEndpoint, coreToken, "/api/resource/preview", {
        nodeId: selected,
        name: file.name,
        bytes,
      });
      setResourcePlan(payload.plan as ResourcePlan);
      setDialog("resource");
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成资源导入预览");
      setDialog("conflict");
    } finally {
      if (imageInputRef.current) imageInputRef.current.value = "";
    }
  }

  async function commitImageImport() {
    if (!resourcePlan) return;
    if (safeMode) {
      setToast("安全模式已启用；资源不会写入工作区");
      return;
    }
    try {
      const nodeId = liveDocument?.nodeId;
      const source = currentSource;
      if (!nodeId) throw new Error("图片资源只能插入已打开的 Core 文档");
      const referencePlan = await requestCoreFormat(source, documentSelection.start, documentSelection.end, {
        kind: "image",
        target: resourcePlan.name,
        alt: resourcePlan.name,
      });
      if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
      const payload = await requestCore(coreEndpoint, coreToken, "/api/resource/commit", { planId: resourcePlan.planId });
      const workspace = payload.workspace as LiveWorkspace;
      setLiveWorkspace(workspace);
      setNodes(workspaceTreeNodes(workspace));
      updateCurrentSource(referencePlan.source);
      restoreDocumentSelection(
        stringOffsetAtUtf8Byte(referencePlan.source, referencePlan.selectionStart),
        stringOffsetAtUtf8Byte(referencePlan.source, referencePlan.selectionEnd),
      );
      setResourcePlan(null);
      setDialog(null);
      setToast("资源已通过 Core 原子写入；图片引用已加入草稿，尚未提交文档");
    } catch (error) {
      setResourcePlan(null);
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了资源导入提交");
      setDialog("conflict");
    }
  }

  function replaceCurrentSelection(replacement: string) {
    const start = Math.min(documentSelection.start, currentSource.length);
    const end = Math.min(documentSelection.end, currentSource.length);
    const nextSource = `${currentSource.slice(0, start)}${replacement}${currentSource.slice(end)}`;
    updateCurrentSource(nextSource);
    restoreDocumentSelection(start, start + replacement.length);
  }

  async function insertTable() {
    if (liveDocument) {
      const nodeId = liveDocument.nodeId;
      const source = currentSource;
      try {
        const plan = await requestCoreFormat(source, documentSelection.start, documentSelection.end, { kind: "table_insert" });
        if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
        updateCurrentSource(plan.source);
        restoreDocumentSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast(`已由 Rust Core 插入 ${activeProfileName} 表格；草稿尚未提交`);
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了表格插入");
        setDialog("conflict");
      }
      return;
    }
    if (!demo) return;
    const lineEnding = demo.lineEnding(currentSource, documentSelection.start);
    const template = `|===${lineEnding}|列 1 |列 2${lineEnding}| |${lineEnding}|===`;
    replaceCurrentSelection(template);
    setToast("已插入 AsciiDoc 表格；草稿尚未提交");
  }

  async function extendTable(operation: "row" | "column") {
    const cursorOffset = Math.min(documentSelection.start, currentSource.length);
    if (liveDocument) {
      const nodeId = liveDocument.nodeId;
      const source = currentSource;
      try {
        const plan = await requestCoreFormat(source, cursorOffset, cursorOffset, { kind: operation === "row" ? "table_add_row" : "table_add_column" });
        if (!formatBaseStillCurrent(nodeId, source, "primary")) return;
        updateCurrentSource(plan.source);
        restoreDocumentSelection(
          stringOffsetAtUtf8Byte(plan.source, plan.selectionStart),
          stringOffsetAtUtf8Byte(plan.source, plan.selectionEnd),
        );
        setToast(operation === "row" ? "Core 已新增表格行；草稿尚未提交" : "Core 已新增表格列；草稿尚未提交");
      } catch (error) {
        setCoreError(error instanceof Error ? error.message : "Core 拒绝了表格命令");
        setDialog("conflict");
      }
      return;
    }
    const table = activeEditorModel?.blocks.find((block) => block.kind === "table" && block.start <= cursorOffset && cursorOffset <= block.end);
    const result = table && demo ? demo.extendTable(currentSource, table, cursorOffset, operation) : null;
    if (!result) {
      setToast("正在等待 Core 确定当前表格块范围");
      return;
    }
    updateCurrentSource(result.source);
    restoreDocumentSelection(result.cursor);
    setToast(operation === "row" ? "已新增表格行；草稿尚未提交" : "已新增表格列；草稿尚未提交");
  }

  function openAnnotationEditor(action: AnnotationActionName, target: string | null = null, messageId: string | null = null) {
    if (!liveWorkspace) {
      setToast("连接 Desktop 或本机 Core 后才能写入节点批注");
      return;
    }
    const annotation = liveAnnotations?.annotations.find((candidate) => candidate.id === target);
    const message = annotation?.thread.find((candidate) => candidate.id === messageId);
    setAnnotationAction(action);
    setAnnotationTarget(target);
    setAnnotationMessageTarget(messageId);
    setAnnotationCreateKind("comment");
    setAnnotationBody(message?.body.source ?? "");
    setAnnotationSuggestedSource("");
    setAnnotationMark(annotation?.appearance?.mark ?? "highlight");
    setAnnotationColor(annotation?.appearance?.theme ?? "yellow");
    setAnnotationLabels(annotation?.labels.join(", ") ?? "");
    setAnnotationPlan(null);
    setDialog("annotation");
  }

  async function previewAnnotationAction() {
    if (!liveWorkspace) return;
    if (dirtyNodeIds.has(selected) || currentRecovery || recoveryIssueCount > 0) {
      setCoreError("批注动作必须绑定磁盘中的精确 revision；请先提交或处理当前设备草稿与恢复问题");
      setDialog("conflict");
      return;
    }
    const selectionStart = Math.min(documentSelection.start, documentSelection.end);
    const selectionEnd = Math.max(documentSelection.start, documentSelection.end);
    const start = new TextEncoder().encode(currentSource.slice(0, selectionStart)).length;
    const end = new TextEncoder().encode(currentSource.slice(0, selectionEnd)).length;
    const request: Record<string, unknown> = {
      action: annotationAction,
      nodeId: selected,
      timestamp: new Date().toISOString(),
    };
    if (annotationAction === "create") {
      request.kind = annotationCreateKind;
      request.target = annotationCreateKind === "suggestion_insert"
        ? { kind: "insertion_point", position: start }
        : annotationCreateKind === "suggestion_delete" || end > start
          ? { kind: "text_range", start, end }
          : { kind: "block_at", sourceOffset: start };
      if (annotationMark !== "none") request.appearance = { mark: annotationMark, theme: annotationColor };
      request.labels = annotationLabels.split(",").map((label) => label.trim()).filter(Boolean);
      if (annotationBody.trim()) request.bodySource = annotationBody;
      if (annotationCreateKind === "suggestion_insert") request.suggestedSource = annotationSuggestedSource;
      request.authorId = annotationActor.id;
      request.authorName = annotationActor.name.trim();
    } else {
      request.annotationId = annotationTarget;
      if (annotationAction === "reply") {
        request.bodySource = annotationBody;
        request.authorId = annotationActor.id;
        request.authorName = annotationActor.name.trim();
      } else if (annotationAction === "edit_message") {
        request.messageId = annotationMessageTarget;
        request.bodySource = annotationBody;
        request.authorId = annotationActor.id;
      } else if (annotationAction === "set_appearance") {
        request.appearance = annotationMark === "none" ? { mark: "none" } : { mark: annotationMark, theme: annotationColor };
      } else if (annotationAction === "set_labels") {
        request.labels = annotationLabels.split(",").map((label) => label.trim()).filter(Boolean);
      }
    }
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/annotation/preview", request);
      setAnnotationPlan(payload.plan as AnnotationPlan);
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成批注事务预览");
      setDialog("conflict");
    }
  }

  async function commitAnnotationAction() {
    if (!annotationPlan) return;
    if (safeMode) {
      setToast("安全模式已启用；批注事务不会提交");
      return;
    }
    try {
      const payload = await requestCore(coreEndpoint, coreToken, `/api/annotation/commit?nodeId=${encodeURIComponent(selected)}`, { planId: annotationPlan.planId });
      if (!isLiveAnnotationStore(payload.annotations)) throw new Error("Core 返回的批注 sidecar 不是 weftext.annotations.v3");
      setLiveAnnotations(payload.annotations);
      const workspace = payload.workspace as LiveWorkspace;
      setLiveWorkspace(workspace);
      setNodes(workspaceTreeNodes(workspace));
      if (payload.document || annotationAction === "accept_suggestion") {
        await openNode(selected, { refresh: true });
      }
      setAnnotationPlan(null);
      setDialog(null);
      setToast("批注 sidecar 已通过可恢复 Core 事务提交");
    } catch (error) {
      setAnnotationPlan(null);
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了批注事务提交");
      setDialog("conflict");
    }
  }

  function previewFocusedChrono(period: "year" | "quarter" | "month" | "week" | "day") {
    const resolved = resolveWorkspaceActionTarget("chrono", currentEditorInvocation());
    if (resolved.kind === "node") void previewChrono(period, resolved);
  }

  async function previewChrono(period: "year" | "quarter" | "month" | "week" | "day", target: FrozenNodeActionTarget) {
    const targetName = chronoNodeName(period, chronoDate);
    if (!liveWorkspace) {
      const target = demo?.chronoTarget(period);
      if (target) await openExplorerNode(target);
      else if (demo) setToast(demo.messages.chronoUnavailable);
      else {
        setCoreError("请先打开 Core 工作区再创建时间节点");
        setDialog("core");
        return;
      }
      setDialog(null);
      return;
    }
    const root = liveWorkspace.nodes.find((node) => node.id === target.nodeId);
    if (!root) return;
    const [year] = chronoDate.split("-");
    const rootPrefix = root.path ? `${root.path}/` : "";
    const expectedPath = period === "year" ? `${rootPrefix}${year}` : `${rootPrefix}${year}/${targetName}`;
    const existing = liveWorkspace.nodes.find((node) => node.path === expectedPath);
    if (existing) {
      setDialog(null);
      await openExplorerNode(existing.id);
      return;
    }
    const [yearText, monthText, dayText] = chronoDate.split("-");
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/chrono/preview", {
        chronoRootId: target.nodeId,
        year: Number(yearText),
        month: Number(monthText),
        day: Number(dayText),
        periods: [period],
      });
      setChronoTargetName(targetName);
      acceptStructuralPreview(payload.plan as StructuralPlan, {
        kind: "node_action",
        target,
        label: WORKSPACE_ACTION_REGISTRY.chrono.label,
      });
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成 Chrono 事务预览");
      setDialog("conflict");
    }
  }

  async function applyTaskCommit(payload: Record<string, unknown>) {
    const workspace = payload.workspace as LiveWorkspace;
    const documentPayload = await requestCore(coreEndpoint, coreToken, `/api/document?nodeId=${encodeURIComponent(selected)}&remember=false`);
    const next = documentPayload.document as LiveDocument;
    requireLiveDocumentContract(next);
    setLiveWorkspace(workspace);
    setNodes(workspaceTreeNodes(workspace));
    setLiveDocument(next);
    loadMetadataInputs(next.metadata);
    setEditorSource(next.source);
    rememberDraft(next.nodeId, next.source);
    setDraftModel(next.model);
    setDraftModelSource(next.source);
    setDraftModelState("ready");
    setDraftModelError("");
    setDirtyNodeIds((current) => {
      const updated = new Set(current);
      updated.delete(next.nodeId);
      return updated;
    });
    setDraftSaveState("idle");
    setDerivedIndexWarning((payload.searchIndexWarning as DerivedIndexWarning | null | undefined) ?? null);
    setToast(payload.searchIndexWarning
      ? "任务已提交；派生搜索索引需要稍后重建（请勿重试提交）"
      : "任务已通过预览的 Core 事务提交");
  }

  async function applyImportCommit(payload: Record<string, unknown>) {
    const workspace = payload.workspace as LiveWorkspace;
    const previousIds = new Set(liveWorkspace?.nodes.map((node) => node.id) ?? []);
    const importedNode = workspace.nodes.find((node) => !previousIds.has(node.id));
    setLiveWorkspace(workspace);
    setNodes(workspaceTreeNodes(workspace));
    setDerivedIndexWarning((payload.searchIndexWarning as DerivedIndexWarning | null | undefined) ?? null);
    setToast(payload.searchIndexWarning
      ? "导入已提交；派生搜索索引需要稍后重建（请勿重试提交）"
      : "固定 Import IR 预览已通过 Core 工作区事务提交");
    await openNode(importedNode?.id ?? workspace.rootNodeId, { refresh: true });
  }

  async function requestSavePreview() {
    if (!liveDocument) {
      if (!demo) {
        setCoreError("当前没有可由 Core 保存的文档");
        setDialog("core");
        return;
      }
      markCurrentSaved();
      setToast(demo.messages.sessionSaved);
      return;
    }
    if (safeMode) {
      setToast("安全模式已启用；草稿已保留，但不会提交到工作区");
      return;
    }
    await requestDocumentSavePreview("primary", liveDocument, editorSource);
  }

  async function requestSplitSavePreview() {
    if (!splitDocument) return;
    if (safeMode) {
      setToast("安全模式已启用；第二栏草稿已保留，但不会提交到工作区");
      return;
    }
    await requestDocumentSavePreview("split", splitDocument, splitDocument.source);
  }

  async function requestDocumentSavePreview(target: "primary" | "split", document: LiveDocument, source: string) {
    try {
      if (desktopMode && dirtyNodeIds.has(document.nodeId)) {
        await persistDesktopDraft(document.nodeId, document.revision, source);
      }
      const payload = await requestCore(coreEndpoint, coreToken, "/api/document/preview", { nodeId: document.nodeId, revision: document.revision, source });
      setSaveTarget(target);
      setSavePlan(payload.plan as SavePlan);
      setDialog("save");
    } catch (error) {
      setCoreError(error instanceof Error ? error.message : "Core 无法生成保存预览");
      setDialog("conflict");
    }
  }

  async function commitSave() {
    const document = saveTarget === "split" ? splitDocument : liveDocument;
    if (!document) return;
    const source = saveTarget === "split" ? document.source : editorSource;
    try {
      const payload = await requestCore(coreEndpoint, coreToken, "/api/document/commit", { nodeId: document.nodeId, revision: document.revision, source });
      const commit = payload.commit as { revision: string; length: number };
      const committedDocument = { ...document, revision: commit.revision, length: commit.length, source, recoveryDraft: null };
      if (saveTarget === "split") setSplitDocument(committedDocument);
      else setLiveDocument(committedDocument);
      if (Object.prototype.hasOwnProperty.call(payload, "icon")) {
        const committedIcon = (payload.icon as ResolvedNodeIcon | null | undefined) ?? null;
        setNodes((current) => current.map((node) => node.id === document.nodeId ? { ...node, icon: committedIcon } : node));
        setLiveWorkspace((current) => current ? { ...current, nodes: current.nodes.map((node) => node.id === document.nodeId ? { ...node, icon: committedIcon } : node) } : current);
      }
      setDraftRecovery((payload.draftRecovery as DraftRecovery | undefined) ?? emptyDraftRecovery);
      if (payload.draftWarning) {
        setDraftRecovery((current) => ({ ...current, issues: [String(payload.draftWarning), ...current.issues] }));
      }
      rememberDraft(document.nodeId, source);
      setDirtyNodeIds((current) => {
        const updated = new Set(current);
        updated.delete(document.nodeId);
        return updated;
      });
      setDraftSaveState(payload.draftWarning ? "error" : "idle");
      setSavePlan(null);
      setDialog(null);
      setCoreError("");
      setToast(payload.searchIndexWarning
        ? "文档已提交；派生搜索索引刷新失败，稍后可安全重建（请勿重试提交）"
        : saveTarget === "split" ? "第二栏文档已由 Rust Core 提交新 revision" : "Rust Core 已验证并提交新 revision");
      if (saveTarget === "primary") await openNode(selected, { refresh: true });
    } catch (error) {
      setSavePlan(null);
      setCoreError(error instanceof Error ? error.message : "Core 拒绝了文档提交");
      setDialog("conflict");
    }
  }

  function measureNavigationAfterRender(
    operation: NavigationPerformanceSample["operation"],
    startedAt: number,
    renderedItems: number,
    totalItems: number,
  ) {
    window.requestAnimationFrame(() => {
      setNavigationPerformance((current) => [
        ...current.slice(-24),
        interactionMeasurement(operation, startedAt, renderedItems, totalItems),
      ]);
    });
  }

  function chooseExplorerActivity(activity: ExplorerActivity) {
    const startedAt = performance.now();
    setExplorerActivity(activity);
    if (activity === "search") setSearchCreatesTab(false);
    measureNavigationAfterRender("mode_switch", startedAt, 1, 1);
  }

  function chooseExplorerMode(mode: ExplorerMode) {
    const startedAt = performance.now();
    setExplorerActivity("explorer");
    setExplorerMode(mode);
    setHierarchyLimit(INITIAL_NAVIGATION_WINDOW);
    setContentsLimit(INITIAL_NAVIGATION_WINDOW);
    measureNavigationAfterRender(
      "mode_switch",
      startedAt,
      mode === "hierarchy" ? Math.min(hierarchyRows.length, INITIAL_NAVIGATION_WINDOW) : Math.min(contentsRows.length, INITIAL_NAVIGATION_WINDOW),
      mode === "hierarchy" ? hierarchyRows.length : contentsRows.length,
    );
  }

  function toggleHierarchyNode(nodeId: string) {
    const startedAt = performance.now();
    setCollapsedNodes((current) => {
      const next = new Set(current);
      if (next.has(nodeId)) next.delete(nodeId); else next.add(nodeId);
      return next;
    });
    measureNavigationAfterRender("expand", startedAt, renderedHierarchy.items.length, hierarchyRows.length);
  }

  async function openExplorerNode(nodeId: string) {
    if (focusedPane === "split" && navigation.split) return switchSplitNode(nodeId);
    return openNode(nodeId);
  }

  function focusHierarchyName(nodeId: string) {
    const target = window.document.querySelector<HTMLButtonElement>(`[data-hierarchy-node="${CSS.escape(nodeId)}"]`);
    target?.focus();
  }

  function handleHierarchyKey(event: React.KeyboardEvent<HTMLButtonElement>, index: number) {
    const startedAt = performance.now();
    const node = renderedHierarchy.items[index];
    if (!node) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Home" || event.key === "End") {
      event.preventDefault();
      let targetIndex = index;
      if (event.key === "ArrowDown") targetIndex = Math.min(hierarchyRows.length - 1, index + 1);
      if (event.key === "ArrowUp") targetIndex = Math.max(0, index - 1);
      if (event.key === "Home") targetIndex = 0;
      if (event.key === "End") targetIndex = hierarchyRows.length - 1;
      if (targetIndex >= hierarchyLimit) setHierarchyLimit(targetIndex + INITIAL_NAVIGATION_WINDOW);
      window.requestAnimationFrame(() => focusHierarchyName(hierarchyRows[targetIndex]?.nodeId ?? node.nodeId));
    } else if (event.key === "ArrowRight" && node.childCount > 0) {
      event.preventDefault();
      if (collapsedNodes.has(node.nodeId)) toggleHierarchyNode(node.nodeId);
      else if (hierarchyRows[index + 1]?.parentNodeId === node.nodeId) focusHierarchyName(hierarchyRows[index + 1].nodeId);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (node.childCount > 0 && !collapsedNodes.has(node.nodeId)) toggleHierarchyNode(node.nodeId);
      else if (node.parentNodeId) focusHierarchyName(node.parentNodeId);
    } else if (event.key === "Enter") {
      event.preventDefault();
      void openExplorerNode(node.nodeId);
    } else {
      return;
    }
    measureNavigationAfterRender("keyboard_move", startedAt, renderedHierarchy.items.length, hierarchyRows.length);
  }

  function browseContents(locator: string) {
    const startedAt = performance.now();
    setContentsBrowseLocator(locator);
    setContentsLimit(INITIAL_NAVIGATION_WINDOW);
    measureNavigationAfterRender("incremental_refresh", startedAt, Math.min(contentsRows.length, INITIAL_NAVIGATION_WINDOW), contentsRows.length);
  }

  function returnContentsToFocusedNode() {
    setContentsBrowseLocator(null);
    window.requestAnimationFrame(() => explorerScrollRef.current?.focus());
  }

  return (
    <main className="app-shell" data-theme={theme} style={{ "--explorer-width": `${explorerWidth}px` } as React.CSSProperties}>
      <header className="topbar">
        <span className="brand-mark" aria-hidden="true" />
        <div className="brand-copy"><strong>文缕</strong><span>Weftext</span></div>
        <span className={`prototype-badge ${liveDocument ? "live" : ""}`}>{desktopMode ? (liveDocument ? "Desktop Alpha" : "桌面端") : liveDocument ? "Core 实机" : demo?.badge ?? "Core 未连接"}</span>
        <button className="workspace-switcher" onClick={() => setDialog("core")}><span className="workspace-dot" />{workspaceName}<span className="chevron">⌄</span></button>
        <button className="global-search" aria-label="搜索工作区" onClick={() => chooseExplorerActivity("search")}><span className="search-symbol">⌕</span><span>搜索文档、标签或内容</span><kbd>Ctrl K</kbd></button>
        <div className="top-actions"><span className={`local-device ${safeMode ? "safe" : ""}`}>{safeMode ? "安全模式" : "本机"}</span></div>
      </header>

      <aside className="activity-rail" aria-label="活动栏">
        <button className={`rail-button ${explorerActivity === "explorer" ? "active" : ""}`} aria-label="Explorer" aria-pressed={explorerActivity === "explorer"} onClick={() => chooseExplorerActivity("explorer")}>▤</button>
        <button className={`rail-button ${explorerActivity === "search" ? "active" : ""}`} aria-label="Search" aria-pressed={explorerActivity === "search"} onClick={() => chooseExplorerActivity("search")}>⌕</button>
        <button className={`rail-button ${explorerActivity === "chrono" ? "active" : ""}`} aria-label="Chrono" aria-pressed={explorerActivity === "chrono"} onClick={() => chooseExplorerActivity("chrono")}>◷</button>
        <button className={`rail-button ${dialog === "query" ? "active" : ""}`} aria-label="查询与视图" aria-pressed={dialog === "query"} onClick={() => setDialog("query")}>▦</button>
        {desktopMode && <button className={`rail-button ${dialog === "intake" ? "active" : ""}`} aria-label="导入中心" aria-pressed={dialog === "intake"} onClick={() => setDialog("intake")}>⇥</button>}
        {desktopMode && <button className={`rail-button ${dialog === "export" ? "active" : ""}`} aria-label="Markdown 导出" aria-pressed={dialog === "export"} disabled={!liveDocument} onClick={() => setDialog("export")}>⇤</button>}
        <button className={`rail-button ${commentsOpen ? "soft-active" : ""}`} aria-label="Inspector" onClick={() => { setCommentsOpen((value) => !value); setInspectorTab("annotations"); }}>◫{commentCount > 0 && <span className="rail-count">{commentCount}</span>}</button>
        <button className={`rail-button ${coreError ? "warning" : ""}`} aria-label={coreError ? "冲突中心有待处理问题" : "冲突中心"} onClick={() => { if (coreError) setDialog("conflict"); else setToast("当前没有待处理冲突"); }}>◇{coreError && <span className="rail-alert" />}</button>
        <button className={`rail-button ${recoveryCount || recoveryIssueCount ? "warning" : ""}`} aria-label="草稿恢复中心" onClick={() => setDialog("recovery")}>↶{recoveryCount > 0 && <span className="rail-count">{recoveryCount}</span>}</button>
        <span className="rail-spacer" />
        <button className="rail-button" aria-label="工作区设置与连接" onClick={() => setDialog("core")}>⚙</button>
      </aside>

      <aside className="node-panel" aria-label={`${explorerActivity === "explorer" ? "Explorer" : explorerActivity === "search" ? "Search" : "Chrono"} 侧栏`}>
        <div className="panel-title-row">
          <div><span className="eyebrow">{explorerActivity === "explorer" ? "EXPLORER" : explorerActivity === "search" ? "SEARCH" : "CHRONO"}</span><h2>{explorerActivity === "explorer" ? "工作区" : explorerActivity === "search" ? "搜索" : "时间节点"}</h2></div>
          {explorerActivity === "explorer" && <button className="add-button" aria-label="新建节点" disabled={!liveWorkspace && !demo} onClick={() => beginNodeAction("create", currentEditorInvocation())}>＋</button>}
        </div>
        {explorerActivity === "explorer" && <>
          <div className="explorer-mode-switch" role="tablist" aria-label="Explorer 模式">
            <button role="tab" aria-selected={explorerMode === "hierarchy"} onClick={() => chooseExplorerMode("hierarchy")}>层级</button>
            <button role="tab" aria-selected={explorerMode === "contents"} onClick={() => chooseExplorerMode("contents")}>内容</button>
          </div>
          <label className="explorer-filter">筛选当前视图<input value={explorerFilter} onChange={(event) => { setExplorerFilter(event.target.value); setHierarchyLimit(INITIAL_NAVIGATION_WINDOW); setContentsLimit(INITIAL_NAVIGATION_WINDOW); }} /></label>
          <div className="explorer-scroll" ref={explorerScrollRef} tabIndex={-1} onScroll={(event) => setExplorerScrollTop(event.currentTarget.scrollTop)}>
            {explorerMode === "hierarchy" ? showTrash ? <section className="trash-item-list" aria-label="工作区废纸篓条目">
              <header><strong>工作区废纸篓</strong><small>同步删除状态，不是备份</small></header>
              {liveWorkspace?.trashLegacyMigrationRequired && <div className="trash-reconciliation" role="alert"><strong>检测到旧废纸篓格式</strong><span>旧条目没有可信来源；迁移前必须建立工作区外精确快照，迁移后会标记来源未知，不会猜测恢复目标。</span><button disabled={safeMode || !desktopMode} onClick={() => void previewLegacyTrashMigration()}>选择快照位置并预览显式迁移</button></div>}
              {liveWorkspace?.trashReconciliation?.required && <div className="trash-reconciliation" role="alert"><strong>废纸篓需要协调</strong><span>{liveWorkspace.trashReconciliation.issueCount} 项证据不完整或冲突；当前仅可查看诊断，恢复和清理均已暂停。</span></div>}
              <div className="trash-resource-batch"><button disabled={safeMode || !liveDocument || liveWorkspace?.trashReconciliation?.required || liveWorkspace?.trashLegacyMigrationRequired} onClick={() => beginResourceTrash(currentEditorInvocation())}>将当前节点资源移入废纸篓</button><small>批量操作仍按独立可恢复条目计数，并共享一个 operationId。</small></div>
              {trashItems.map((item) => {
                const manifest = item.manifest;
                const bytes = trashPayloadByteLength(manifest);
                const originLabel = ({
                  active: "来源可用",
                  in_trash: "来源也在废纸篓",
                  missing: "来源缺失",
                  unknown: "来源未知",
                  reconciliation_required: "需要协调",
                } as const)[item.restore.originResolution];
                return <button key={manifest.trashItemId} className="trash-item-row" onClick={() => openTrashItem(item)}>
                  <span aria-hidden="true">{manifest.kind === "node" ? "文" : "档"}</span>
                  <div><strong dir="auto">{manifest.originalName}</strong><small>{manifest.kind === "node" ? "节点子树" : "独立资源"} · {bytes.toLocaleString()} 字节</small><small>{originLabel} · {new Date(manifest.trashedAt).toLocaleString()}</small></div>
                </button>;
              })}
              {!trashItems.length && !liveWorkspace?.trashReconciliation?.required && <p className="explorer-empty" role="status">工作区废纸篓为空。</p>}
            </section> : <div className="hierarchy-tree" role="tree" aria-label="工作区层级" aria-multiselectable="false">
              {renderedHierarchy.items.map((node, index) => {
                const collapsed = collapsedNodes.has(node.nodeId);
                const selectedInExplorer = followedNodeId === node.nodeId;
                const glyph = workspaceIconGlyph(node.displayIcon as WorkspaceItemIcon);
                return <div className={`hierarchy-row ${selectedInExplorer ? "active" : ""}`} role="treeitem" aria-level={node.depth + 1} aria-selected={selectedInExplorer} aria-expanded={node.childCount ? !collapsed : undefined} key={node.nodeId} style={{ paddingInlineStart: `${6 + node.depth * 16}px` }}>
                  {node.childCount ? <button className="disclosure-button" aria-label={`${collapsed ? "展开" : "折叠"} ${node.name}`} onClick={() => toggleHierarchyNode(node.nodeId)}><span className={`tree-caret ${collapsed ? "collapsed" : "expanded"}`} aria-hidden="true" /></button> : <span className="disclosure-placeholder" aria-hidden="true" />}
                  <button className="hierarchy-name" data-hierarchy-node={node.nodeId} tabIndex={selectedInExplorer || (!followedNodeId && index === 0) ? 0 : -1} aria-label={`打开节点 ${node.name}`} onKeyDown={(event) => handleHierarchyKey(event, index)} onClick={() => void openExplorerNode(node.nodeId)}>
                    {iconPreferences.placement === "before" && glyph && <span className="node-icon" aria-hidden="true">{glyph}</span>}<span dir="auto">{node.name}</span>{iconPreferences.placement === "after" && glyph && <span className="node-icon after" aria-hidden="true">{glyph}</span>}
                  </button>
                  <button className="row-action-button" aria-label={`${node.name} 的节点操作`} onClick={() => openNodeActionChooser({ source: "explicit_node_row", surface: "hierarchy", nodeId: node.nodeId })}>•••</button>
                </div>;
              })}
              {!hierarchyRows.length && <p className="explorer-empty" role="status">没有匹配的托管节点。</p>}
              {renderedHierarchy.remaining > 0 && <button className="load-more" onClick={() => setHierarchyLimit((current) => current + INITIAL_NAVIGATION_WINDOW)}>继续显示 {Math.min(INITIAL_NAVIGATION_WINDOW, renderedHierarchy.remaining)} 项</button>}
            </div> : <section className="contents-browser" aria-label="当前位置内容">
              <header>
                <span>正在浏览：<strong dir="auto">{contentsLocator || "工作区根"}</strong></span>
                <small>跟随{focusedPane === "split" && navigation.split ? "第二编辑栏" : "主编辑栏"}：{followedNode.name}</small>
                {contentsBrowseLocator !== null && <button aria-keyshortcuts="Alt+Home" onClick={returnContentsToFocusedNode}>回到当前节点</button>}
              </header>
              <nav className="contents-breadcrumbs" aria-label="浏览位置面包屑">{contentsBreadcrumbs.map((crumb, index) => <span key={crumb.locator}>{index > 0 && <b aria-hidden="true">/</b>}<button aria-current={crumb.locator === contentsLocator ? "location" : undefined} onClick={() => crumb.unmanaged ? browseContents(crumb.locator) : crumb.nodeId ? void openExplorerNode(crumb.nodeId) : returnContentsToFocusedNode()}>{crumb.name}</button></span>)}</nav>
              <div className="contents-list" role="list" aria-label={`${contentsLocator || "工作区根"} 的直接子项`}>
                {renderedContents.items.map((item) => {
                  const glyph = workspaceIconGlyph(item.displayIcon as WorkspaceItemIcon);
                  const kindLabel = { managed_node: "托管节点", unmanaged_directory: "非托管文件夹", unmanaged_markdown: "非托管 Markdown，只读", resource: "节点资源，只读" }[item.kind];
                  return <div className={`contents-row ${item.kind}`} role="listitem" key={`${item.kind}-${item.locator}`}>
                    {item.kind === "managed_node" && item.nodeId ? <><button aria-label={`打开节点 ${item.name}`} onClick={() => void openExplorerNode(item.nodeId!)}>{glyph && <span aria-hidden="true">{glyph}</span>}<span dir="auto">{item.name}</span><small>{kindLabel}</small></button><button className="row-action-button" aria-label={`${item.name} 的节点操作`} onClick={() => openNodeActionChooser({ source: "explicit_node_row", surface: "contents", nodeId: item.nodeId! })}>•••</button></>
                      : item.kind === "unmanaged_directory" ? <button aria-label={`浏览文件夹 ${item.name}`} onClick={() => browseContents(item.locator)}>{glyph && <span aria-hidden="true">{glyph}</span>}<span dir="auto">{item.name}</span><small>{kindLabel}</small></button>
                      : item.kind === "resource" && item.ownerNodeId ? <><div aria-label={`${item.name}，${kindLabel}`}>{glyph && <span aria-hidden="true">{glyph}</span>}<span dir="auto">{item.name}</span><small>{kindLabel}</small></div><button className="row-action-button" aria-label={`将资源 ${item.name} 移入废纸篓`} onClick={() => beginResourceTrash({ source: "resource_row", ownerNodeId: item.ownerNodeId!, resourceName: item.name })}>•••</button></>
                      : <div aria-label={`${item.name}，${kindLabel}`}>{glyph && <span aria-hidden="true">{glyph}</span>}<span dir="auto">{item.name}</span><small>{kindLabel}</small></div>}
                  </div>;
                })}
                {!contentsRows.length && <p className="explorer-empty" role="status">此位置没有 Core 可见的直接子项。</p>}
                {renderedContents.remaining > 0 && <button className="load-more" onClick={() => setContentsLimit((current) => current + INITIAL_NAVIGATION_WINDOW)}>继续显示 {Math.min(INITIAL_NAVIGATION_WINDOW, renderedContents.remaining)} 项</button>}
              </div>
            </section>}
          </div>
          {!showTrash && recentNodes.length > 0 && <section className="navigation-list" aria-label="最近节点"><strong>最近</strong>{recentNodes.map((node) => { const glyph = workspaceIconGlyph(node.displayIcon, node.icon); return <button key={node.id} onClick={() => void openExplorerNode(node.id)}>{glyph && iconPreferences.placement === "before" && <span aria-hidden="true">{glyph}</span>}{node.name}{glyph && iconPreferences.placement === "after" && <span aria-hidden="true">{glyph}</span>}</button>; })}</section>}
          {!showTrash && bookmarkedNodes.length > 0 && <section className="navigation-list" aria-label="书签"><strong>书签</strong>{bookmarkedNodes.map((node) => { const glyph = workspaceIconGlyph(node.displayIcon, node.icon); return <button key={node.id} onClick={() => void openExplorerNode(node.id)}>{glyph && iconPreferences.placement === "before" && <span aria-hidden="true">{glyph}</span>}{node.name}{glyph && iconPreferences.placement === "after" && <span aria-hidden="true">{glyph}</span>}</button>; })}</section>}
          {explorerMode === "hierarchy" && <button className={`trash-link ${showTrash ? "active" : ""}`} onClick={() => { if (liveWorkspace) setShowTrash((value) => !value); else if (demo) setToast(demo.messages.trashEmpty); else setDialog("core"); }}>♲ {showTrash ? "返回文档" : "废纸篓"} <span>{trashCount}</span></button>}
        </>}
        {explorerActivity === "search" && <section className="activity-search" role="search" aria-label="搜索工作区">
          <input ref={searchInputRef} value={query} onChange={(event) => { setQuery(event.target.value); setWorkspaceSearch([]); setSearchIndex(0); }} onKeyDown={(event) => { if (event.key === "ArrowDown") { event.preventDefault(); setSearchIndex((current) => results.length ? (current + 1) % results.length : 0); } else if (event.key === "ArrowUp") { event.preventDefault(); setSearchIndex((current) => results.length ? (current - 1 + results.length) % results.length : 0); } else if (event.key === "Enter" && results[effectiveSearchIndex]) { event.preventDefault(); void openExplorerNode(results[effectiveSearchIndex].id); } }} aria-label="搜索节点、用户属性或正文" placeholder="搜索节点、用户属性或正文…" />
          <div className="activity-results" role="list" aria-label="搜索结果">{results.map((node, index) => <div role="listitem" key={node.id}><button className={index === effectiveSearchIndex ? "selected" : ""} onClick={() => void openExplorerNode(node.id)}><strong>{node.name}</strong><small>{"snippet" in node ? node.snippet : node.path ?? "托管节点"}</small></button><button className="row-action-button" aria-label={`${node.name} 的节点操作`} onClick={() => openNodeActionChooser({ source: "explicit_node_row", surface: "search", nodeId: node.id })}>•••</button></div>)}{query.trim() && !results.length && <p role="status">没有结果。</p>}</div>
        </section>}
        {explorerActivity === "chrono" && <section className="activity-chrono" aria-label="Chrono 导航">
          <label>日期<input type="date" value={chronoDate} onChange={(event) => setChronoDate(event.target.value)} /></label>
          <p>目标：<strong>{followedNode.name}</strong>（{focusedPane === "split" && navigation.split ? "第二编辑栏" : "主编辑栏"}）</p>
          <div className="chrono-list">{(["year", "quarter", "month", "week", "day"] as const).map((period) => <button key={period} onClick={() => previewFocusedChrono(period)}><span>{{ year: "年", quarter: "季度", month: "月", week: "ISO 周", day: "日" }[period]}</span><strong>{chronoNodeName(period, chronoDate)}</strong></button>)}</div>
        </section>}
        <div className="explorer-resize" aria-label="Explorer 宽度"><button aria-label="缩窄 Explorer" onClick={() => setExplorerWidth((value) => Math.max(220, value - 24))}>−</button><span>{explorerWidth}px</span><button aria-label="加宽 Explorer" onClick={() => setExplorerWidth((value) => Math.min(480, value + 24))}>＋</button></div>
        <details className="navigation-performance"><summary>导航性能</summary>{navigationPerformance.slice(-5).map((sample, index) => <div key={`${sample.operation}-${index}`}><span>{sample.operation}</span><strong>{sample.durationMs.toFixed(2)} ms</strong><small>{sample.renderedItems}/{sample.totalItems} UI 项；不含 Core 扫描</small></div>)}</details>
        <div className="panel-footer"><span>{nodes.length} 个托管节点</span>{coreError ? <button onClick={() => setDialog("conflict")}><i />1 个冲突待处理</button> : recoveryCount || recoveryIssueCount ? <button onClick={() => setDialog("recovery")}><i />{recoveryCount + recoveryIssueCount} 个恢复项</button> : <span>没有待处理冲突</span>}</div>
      </aside>

      <section className="workspace-main">
        <div className="tabs-row">
          <button className="navigation-arrow" aria-label="后退" disabled={!navigation.back.length} onClick={() => void goBack()}>←</button><button className="navigation-arrow" aria-label="前进" disabled={!navigation.forward.length} onClick={() => void goForward()}>→</button>
          {navigation.tabs.map((tab) => { const node = nodes.find((candidate) => candidate.id === tab.nodeId); return <div className={`tab-item ${tab.id === navigation.activeTabId ? "active" : ""}`} key={tab.id}><button className="tab" onClick={() => void openNode(tab.nodeId, { tabId: tab.id })}>{iconPreferences.showInTitle && node?.icon && <span className="title-node-icon" aria-hidden="true">{node.icon.glyph}</span>}{node?.name ?? "已移除节点"}</button><button className="tab-close" aria-label={`关闭标签 ${node?.name ?? "已移除节点"}`} disabled={navigation.tabs.length === 1} onClick={() => void closeTab(tab.id)}>×</button></div>; })}
          <button className="new-tab" aria-label="新标签快速打开节点" onClick={() => { setSearchCreatesTab(true); setDialog("search"); }}>＋</button><span className="tabs-spacer" />
          <button className={`plain-icon ${navigation.split ? "active" : ""}`} aria-label={navigation.split ? "关闭双栏分屏" : "打开双栏分屏"} onClick={() => void toggleSplit()}>◫</button>
          <button className={`plain-icon ${navigation.bookmarks.includes(selected) ? "active" : ""}`} aria-label={navigation.bookmarks.includes(selected) ? "移除当前书签" : "收藏当前节点"} onClick={toggleBookmark}>☆</button>
          <button className="plain-icon" aria-label="当前编辑栏节点操作" onClick={() => openNodeActionChooser(currentEditorInvocation())}>•••</button>
        </div>

        <div className="document-toolbar">
          <nav className="breadcrumbs" aria-label="面包屑">{breadcrumbNodes.map((node, index) => <span key={node.id}>{index > 0 && <b>/</b>}<button aria-current={node.id === selected ? "page" : undefined} onClick={() => void openNode(node.id)}>{node.name}</button></span>)}</nav>
          <button className={`toolbar-button ${commentsOpen && inspectorTab === "outline" ? "active" : ""}`} aria-pressed={commentsOpen && inspectorTab === "outline"} onClick={() => { setCommentsOpen(true); setInspectorTab("outline"); }}>大纲</button>
          <button className={`toolbar-button ${findOpen ? "active" : ""}`} aria-keyshortcuts="Control+F" onClick={() => { setFindOpen(true); if (view === "read") setView("write"); }}>查找</button>
          <div className="view-switch" aria-label="文档视图">
            {(["write", "source", "read"] as const).map((item) => <button key={item} onClick={() => chooseView(item)} className={view === item ? "active" : ""}>{{ write: "写作", source: "源码", read: "阅读" }[item]}</button>)}
          </div>
          <button className={`toolbar-button ${dialog === "citation" ? "active" : ""}`} disabled={!citationAvailable} title={citationAvailable ? "插入引用" : "需要 Weftext AsciiDoc 工作区"} onClick={openCitationDialog}>引用</button>
          <button className="toolbar-button" aria-label="设置节点图标" onClick={() => setDialog("icon")}>{activeIcon ? <span aria-hidden="true">{activeIcon.glyph}</span> : "图标"}</button>
          <button className={`comments-toggle ${commentsOpen && inspectorTab === "annotations" ? "active" : ""}`} onClick={() => { setCommentsOpen(true); setInspectorTab("annotations"); }}>批注 {commentCount > 0 && <span>{commentCount}</span>}</button>
          <button className="toolbar-more" aria-label="当前编辑栏节点操作" onClick={() => openNodeActionChooser(currentEditorInvocation())}>•••</button>
        </div>

        <div className={`content-frame ${commentsOpen ? "" : "comments-closed"} ${navigation.split ? "split-open" : ""}`}>
          <article ref={documentSurfaceRef} className={`document-surface view-${view}`} onFocusCapture={() => setFocusedPane("primary")} data-explorer-followed={focusedPane === "primary"}>
            {findOpen && <section className="document-find" role="search" aria-label="查找当前文档">
              <div className="find-row"><input ref={findInputRef} aria-label="查找文本" value={findQuery} onChange={(event) => { setFindQuery(event.target.value); setFindIndex(0); setFindNavigated(false); }} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); navigateDocumentMatch(findNavigated ? effectiveFindIndex + (event.shiftKey ? -1 : 1) : event.shiftKey ? -1 : 0); } else if (event.key === "Escape") { setFindOpen(false); } }} placeholder="查找当前文档" /><span aria-live="polite">{documentMatches.length ? `${effectiveFindIndex + 1} / ${documentMatches.length}` : "0 / 0"}</span><button aria-label="上一个匹配" disabled={!documentMatches.length} onClick={() => navigateDocumentMatch(findNavigated ? effectiveFindIndex - 1 : -1)}>↑</button><button aria-label="下一个匹配" disabled={!documentMatches.length} onClick={() => navigateDocumentMatch(findNavigated ? effectiveFindIndex + 1 : 0)}>↓</button><button aria-pressed={replaceOpen} onClick={() => setReplaceOpen((value) => !value)}>替换</button><button aria-label="关闭文档查找" onClick={() => setFindOpen(false)}>×</button></div>
              {replaceOpen && <div className="find-row replace-row"><input aria-label="替换文本" value={replaceText} onChange={(event) => setReplaceText(event.target.value)} placeholder="替换为" /><button disabled={!documentMatches.length} onClick={replaceCurrentMatch}>替换当前</button><button disabled={!documentMatches.length} onClick={replaceAllMatches}>全部替换</button></div>}
            </section>}
            <div className="document-meta"><span className="status-pill">{document.label}</span><span>{liveDocument ? "Rust Core 精确字节源" : demo?.sourceLabel ?? (coreState === "error" ? "Core 连接失败" : "正在等待 Core")}</span><span className="revision">{liveDocument ? `rev ${liveDocument.revision.slice(0, 8)}` : demo ? saved ? "已保存到会话" : "草稿未保存" : coreState}</span></div>
            {view !== "read" && (liveDocument || demo) && <><div className="document-format-tools" role="toolbar" aria-label={`光标处 ${activeProfileName} 格式`}><select aria-label="标题级别" value="" onChange={(event) => { if (event.target.value) void applyHeadingLevel(Number(event.target.value)); }}><option value="">H1–H9</option>{Array.from({ length: 9 }, (_, index) => <option value={index + 1} key={index + 1}>H{index + 1}</option>)}</select><button aria-label="段落" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyBlockFormat("paragraph")}>¶</button><button aria-label="列表" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyBlockFormat("list")}>≡</button><button aria-label="引用加深" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyBlockFormat("quote_increase")}>❯＋</button><button aria-label="引用变浅" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyBlockFormat("quote_decrease")}>❮－</button><button aria-label="代码块" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyBlockFormat("code")}>⌗</button><span aria-hidden="true" /><button aria-label="加粗" aria-keyshortcuts="Control+B" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyInlineFormat("bold")}><strong>B</strong></button><button aria-label="强调" aria-keyshortcuts="Control+I" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyInlineFormat("emphasis")}><em>I</em></button><button aria-label="行内代码" aria-keyshortcuts="Control+E" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyInlineFormat("inline_code")}>&lt;/&gt;</button><button aria-label="链接" onMouseDown={(event) => event.preventDefault()} onClick={() => void applyInlineFormat("link")}>↗</button><span aria-hidden="true" /><button aria-label="插入图片资源" disabled={!liveWorkspace} onClick={() => imageInputRef.current?.click()}>图片</button><button aria-label="插入表格" onClick={() => void insertTable()}>表格</button><button aria-label="新增表格行" onClick={() => void extendTable("row")}>＋行</button><button aria-label="新增表格列" onClick={() => void extendTable("column")}>＋列</button><button aria-label="插入引用" disabled={!citationAvailable} onMouseDown={(event) => event.preventDefault()} onClick={openCitationDialog}>引用</button></div><input ref={imageInputRef} className="resource-input" type="file" accept="image/*" aria-label="选择图片资源" onChange={(event) => { const file = event.currentTarget.files?.[0]; if (file) void previewImageImport(file); }} />{citationAvailable && <div className={`citation-projection-status ${citationDiagnostics.length || citationAnalysisError ? "warning" : ""}`} role="status"><span>Core 引用投影</span><strong>{citationAnalysisError ? "暂不可用" : activeCitationDraft ? `${activeCitationDraft.analysis.diagnostics.length} 个解析诊断 · ${citationComponent?.citations.length ?? 0} 个已渲染引用` : "正在解析当前草稿…"}</strong></div>}</>}
            {!liveDocument && !demo ? (
              <div className="core-model-pending" role={coreState === "error" ? "alert" : "status"}><strong>{coreState === "error" ? "Core 无法打开文档" : "正在等待 Core 文档"}</strong>{(coreError || restoreError) && <span>{coreError || restoreError}</span>}</div>
            ) : view === "source" ? (
              <SourceEditor
                profile={activeProfile}
                value={currentSource}
                selectionStart={documentSelection.start}
                selectionEnd={documentSelection.end}
                scrollTop={documentScrollTop}
                restoreToken={selectionRestoreToken}
                onChange={updateCurrentSource}
                onSelectionChange={rememberDocumentSelection}
                onScroll={rememberDocumentScroll}
                onKeyDown={handleFormatKeyDown}
                onFind={(replace) => { setFindOpen(true); if (replace) setReplaceOpen(true); }}
                stateKey={`${liveWorkspace?.rootNodeId ?? initialWorkspaceId}/${selected}`}
              />
            ) : view === "write" ? (
              activeEditorModel ? <><div className="core-layout-evidence"><span>Core 结构化 Write 模型</span><strong>{activeEditorModel.blocks.filter((block) => block.kind !== "frontmatter").length} 个语义块 · {activeEditorModel.diagnostics.length} 个诊断</strong></div><WriteEditor
                profile={activeProfile}
                source={currentSource}
                model={activeEditorModel}
                selectionStart={documentSelection.start}
                selectionEnd={documentSelection.end}
                scrollTop={documentScrollTop}
                restoreToken={selectionRestoreToken}
                onChange={updateCurrentSource}
                onSelectionChange={rememberDocumentSelection}
                onScroll={rememberDocumentScroll}
                onKeyDown={handleFormatKeyDown}
                onFind={(replace) => { setFindOpen(true); if (replace) setReplaceOpen(true); }}
              /></> : demo ? <textarea ref={writeEditorRef} className="live-write-editor" aria-label="AsciiDoc 正文" value={currentSource.slice(bodyStart)} onChange={(event) => updateCurrentSource(`${currentSource.slice(0, bodyStart)}${event.target.value}`)} onSelect={(event) => rememberDocumentSelection(bodyStart + event.currentTarget.selectionStart, bodyStart + event.currentTarget.selectionEnd)} onScroll={(event) => rememberDocumentScroll(event.currentTarget.scrollTop)} onKeyDown={handleFormatKeyDown} /> : draftModelState === "error" ? <div className="core-model-pending" role="alert"><strong>Core 无法解析当前草稿</strong><span>{draftModelError}</span><button onClick={() => setView("source")}>打开精确源码</button></div> : <div className="core-model-pending" role="status">正在等待 Core 解析当前精确草稿…</div>
            ) : (
              <div className="live-document-preview">{activeModel ? renderModel(activeModel, currentSource, citationComponent) : demo ? demo.render(currentSource, false) : draftModelState === "error" ? <div className="core-model-pending" role="alert"><strong>Core 无法生成阅读模型</strong><span>{draftModelError}</span><button onClick={() => setView("source")}>打开精确源码</button></div> : <div className="core-model-pending" role="status">正在等待 Core 阅读模型…</div>}{commentCount > 0 && <button className="comment-marker" aria-label={`查看 ${commentCount} 条批注`} onClick={() => setCommentsOpen(true)}>{commentCount}</button>}</div>
            )}
          </article>

          {navigation.split && <aside className="split-pane" aria-label="第二编辑栏" onFocusCapture={() => setFocusedPane("split")} data-explorer-followed={focusedPane === "split"}>
            <header><strong>{nodes.find((node) => node.id === navigation.split?.nodeId)?.name ?? "第二编辑栏"}</strong><select aria-label="切换第二编辑栏节点" value={navigation.split.nodeId} onChange={(event) => void switchSplitNode(event.target.value)}>{nodes.filter((node) => node.id !== selected || node.id === navigation.split?.nodeId).map((node) => <option value={node.id} key={node.id}>{node.name}</option>)}</select><button aria-label="保存第二编辑栏" disabled={!splitDocument || !dirtyNodeIds.has(splitDocument.nodeId)} onClick={() => void requestSplitSavePreview()}>保存</button><button aria-label="查找第二编辑栏" onClick={() => setSplitFindOpen((open) => !open)}>查找</button><div className="view-switch" aria-label="第二编辑栏视图">{(["write", "source", "read"] as const).map((item) => <button key={item} className={navigation.split?.view === item ? "active" : ""} onClick={() => updateSplitSession({ view: item })}>{{ write: "写作", source: "源码", read: "阅读" }[item]}</button>)}</div><button aria-label="关闭第二编辑栏" onClick={() => void toggleSplit()}>×</button></header>
            {splitDocument?.recoveryDraft?.stale && <div className="split-recovery-warning" role="alert"><span>检测到基于旧 revision 的第二栏恢复草稿；当前显示磁盘版本，未静默套用草稿。</span><button onClick={() => void openNode(splitDocument.nodeId)}>在主栏比较草稿</button></div>}
            {splitFindOpen && <section className="document-find split-find" role="search" aria-label="查找第二编辑栏"><div className="find-row"><input aria-label="查找第二编辑栏文本" value={splitFindQuery} onChange={(event) => { setSplitFindQuery(event.target.value); setSplitFindIndex(0); }} onKeyDown={(event) => { if (event.key === "Enter") navigateSplitMatch(effectiveSplitFindIndex + (event.shiftKey ? -1 : 1)); }} /><span aria-live="polite">{splitMatches.length ? `${effectiveSplitFindIndex + 1} / ${splitMatches.length}` : "0 / 0"}</span><button aria-label="第二栏上一个匹配" disabled={!splitMatches.length} onClick={() => navigateSplitMatch(effectiveSplitFindIndex - 1)}>↑</button><button aria-label="第二栏下一个匹配" disabled={!splitMatches.length} onClick={() => navigateSplitMatch(effectiveSplitFindIndex + 1)}>↓</button><button aria-pressed={splitReplaceOpen} onClick={() => setSplitReplaceOpen((open) => !open)}>替换</button><button aria-label="关闭第二栏查找" onClick={() => setSplitFindOpen(false)}>×</button></div>{splitReplaceOpen && <div className="find-row replace-row"><input aria-label="第二栏替换文本" value={splitReplaceText} onChange={(event) => setSplitReplaceText(event.target.value)} /><button disabled={!splitMatches.length} onClick={() => replaceSplitMatch(false)}>替换当前</button><button disabled={!splitMatches.length} onClick={() => replaceSplitMatch(true)}>全部替换</button></div>}</section>}
            {splitDocument && splitModel && splitEditorModel && navigation.split.view !== "read" && <div className="document-format-tools split-tools" role="toolbar" aria-label="第二编辑栏 AsciiDoc 格式"><select aria-label="第二栏标题级别" value="" onChange={(event) => { if (event.target.value) void applySplitHeadingLevel(Number(event.target.value)); }}><option value="">H1–H9</option>{Array.from({ length: 9 }, (_, index) => <option value={index + 1} key={index + 1}>H{index + 1}</option>)}</select><button aria-label="第二栏段落" onClick={() => void applySplitBlockFormat("paragraph")}>¶</button><button aria-label="第二栏列表" onClick={() => void applySplitBlockFormat("list")}>≡</button><button aria-label="第二栏引用加深" onClick={() => void applySplitBlockFormat("quote_increase")}>❯＋</button><button aria-label="第二栏引用变浅" onClick={() => void applySplitBlockFormat("quote_decrease")}>❮－</button><button aria-label="第二栏代码块" onClick={() => void applySplitBlockFormat("code")}>⌗</button><button aria-label="第二栏加粗" onClick={() => void applySplitInlineFormat("bold")}><strong>B</strong></button><button aria-label="第二栏强调" onClick={() => void applySplitInlineFormat("emphasis")}><em>I</em></button><button aria-label="第二栏行内代码" onClick={() => void applySplitInlineFormat("inline_code")}>&lt;/&gt;</button><button aria-label="第二栏插入表格" onClick={() => void insertSplitTable()}>表格</button><button aria-label="第二栏新增表格行" onClick={() => void extendSplitTable("row")}>＋行</button><button aria-label="第二栏新增表格列" onClick={() => void extendSplitTable("column")}>＋列</button></div>}
            {splitDocument ? navigation.split.view === "source" ? <SourceEditor profile={splitProfile} value={splitDocument.source} selectionStart={navigation.split.selectionStart} selectionEnd={navigation.split.selectionEnd} scrollTop={navigation.split.scrollTop} restoreToken={splitSelectionRestoreToken} onChange={updateSplitSource} onSelectionChange={(start, end) => updateSplitSession({ selectionStart: start, selectionEnd: end })} onScroll={(scrollTop) => updateSplitSession({ scrollTop })} onKeyDown={handleSplitFormatKeyDown} onFind={(replace) => { setSplitFindOpen(true); if (replace) setSplitReplaceOpen(true); }} stateKey={`${liveWorkspace?.rootNodeId ?? initialWorkspaceId}/${navigation.split.nodeId}`} /> : splitModel && splitEditorModel ? navigation.split.view === "write" ? <WriteEditor profile={splitProfile} source={splitDocument.source} model={splitEditorModel} selectionStart={navigation.split.selectionStart} selectionEnd={navigation.split.selectionEnd} scrollTop={navigation.split.scrollTop} restoreToken={splitSelectionRestoreToken} onChange={updateSplitSource} onSelectionChange={(start, end) => updateSplitSession({ selectionStart: start, selectionEnd: end })} onScroll={(scrollTop) => updateSplitSession({ scrollTop })} onKeyDown={handleSplitFormatKeyDown} onFind={(replace) => { setSplitFindOpen(true); if (replace) setSplitReplaceOpen(true); }} /> : <div className="live-document-preview">{renderModel(splitModel, splitDocument.source)}</div> : splitModelState === "error" ? <div className="core-model-pending" role="alert"><strong>Core 无法解析第二栏草稿</strong><span>{splitModelError}</span><button onClick={() => updateSplitSession({ view: "source" })}>打开精确源码</button></div> : <div className="core-model-pending" role="status">正在等待 Core 解析第二栏草稿…</div> : demo ? <div className="live-document-preview">{demo.render(demo.sourceFor(navigation.split.nodeId, nodes.find((node) => node.id === navigation.split?.nodeId)?.name ?? "第二节点"), false)}</div> : splitModelState === "error" ? <div className="core-model-pending" role="alert">{splitModelError}</div> : <div className="core-model-pending" role="status">正在从 Core 打开第二节点…</div>}
          </aside>}

          {commentsOpen && <aside className="annotation-panel inspector-panel" aria-label="Inspector">
            <div className="annotation-heading"><div><span className="eyebrow">INSPECTOR</span><h3>{followedNode.name}</h3></div><button aria-label="关闭 Inspector" onClick={() => setCommentsOpen(false)}>×</button></div>
            <nav className="inspector-tabs" aria-label="Inspector 面板">
              {(["outline", "properties", "tasks", "citations", "annotations", "backlinks"] as const).map((item) => <button key={item} aria-current={inspectorTab === item ? "page" : undefined} onClick={() => setInspectorTab(item)}>{{ outline: "Outline", properties: "Properties", tasks: "Tasks", citations: "Citations", annotations: "Annotations", backlinks: "Backlinks" }[item]}</button>)}
              <button disabled title="多角色 ACL 尚未实现">Permissions（后续）</button>
            </nav>
            {inspectorTab === "outline" && <section className="document-outline inspector-section" aria-label="文档大纲">
              {headings.length ? <nav aria-label="文档大纲">{headings.map((heading, index) => <button key={`${heading.start}-${heading.text}`} className={currentHeading?.start === heading.start ? "active" : ""} style={{ paddingLeft: `${10 + (heading.level - 1) * 12}px` }} aria-current={currentHeading?.start === heading.start ? "location" : undefined} onClick={() => navigateHeading(heading, index)}><span>{heading.text}</span><small>{heading.line}</small></button>)}</nav> : <p>当前文档没有标题。</p>}
            </section>}
            {inspectorTab === "properties" && <section className="properties-panel inspector-section" aria-label="文档属性">
              <div className="properties-heading"><span>节点系统元数据</span><small>仅由 Core 投影与窄事务修改</small></div>
              {nodeMetadata ? <div className="node-metadata-controls" role="group" aria-label="节点系统元数据">
                <div className="node-metadata-readonly"><span>节点 ID</span><code>{nodeMetadata.id}</code></div>
                <label>别名（每行一个）<textarea aria-label="节点别名（每行一个）" value={aliasesInput} disabled={!metadataEditable} onChange={(event) => setAliasesInput(event.target.value)} onKeyDown={(event) => { if (event.nativeEvent.isComposing) return; if ((event.ctrlKey || event.metaKey) && event.key === "Enter") { event.preventDefault(); void previewNodeMetadata({ action: "aliases", aliases: aliasesInput.split(/\r?\n/).filter((alias) => alias.length > 0) }, "更新有序节点别名"); } }} /></label>
                <button disabled={!metadataEditable} onClick={() => void previewNodeMetadata({ action: "aliases", aliases: aliasesInput.split(/\r?\n/).filter((alias) => alias.length > 0) }, "更新有序节点别名")}>预览别名事务</button>
                <div className="node-metadata-row"><span>便携图标</span><strong>{nodeMetadata.resolvedIcon?.glyph ?? nodeMetadata.icon ?? "未设置"}</strong><button disabled={!metadataEditable} onClick={() => setDialog("icon")}>选择</button></div>
                <div className="node-sort-controls"><label>直接子节点排序<select aria-label="直接子节点排序" value={childSortInput} disabled={!metadataEditable} onChange={(event) => setChildSortInput(event.target.value as "name" | "manual")}><option value="name">自然名称</option><option value="manual">手动稀疏排序</option></select></label><label>方向<select aria-label="子节点排序方向" value={childSortDirectionInput} disabled={!metadataEditable || childSortInput === "manual"} onChange={(event) => setChildSortDirectionInput(event.target.value as "ascending" | "descending")}><option value="ascending">升序</option><option value="descending">降序</option></select></label><button disabled={!metadataEditable} onClick={() => void previewNodeMetadata({ action: "child_sort", mode: childSortInput, ...(childSortInput === "name" ? { direction: childSortDirectionInput } : {}) }, "更新直接子节点排序")}>预览排序事务</button></div>
                {nodeMetadata.adjacentHeadingBody !== null ? <div className="node-metadata-row"><span>紧邻标题 + 正文</span><strong>{nodeMetadata.adjacentHeadingBody === "run_in" ? "混排" : "分开"}</strong><button disabled={!metadataEditable} onClick={() => void previewWorkspaceAction({ action: "presentation", value: nodeMetadata.adjacentHeadingBody === "run_in" ? "separate" : "run_in" })}>切换并预览</button></div> : <div className="node-rank-controls"><label>同级稀疏 rank<input aria-label="同级稀疏 rank" inputMode="numeric" value={siblingRankInput} disabled={!metadataEditable} onChange={(event) => setSiblingRankInput(event.target.value)} onKeyDown={(event) => { if (event.nativeEvent.isComposing) return; if (event.key === "Enter" && siblingRankValid) { event.preventDefault(); void previewNodeMetadata(siblingRankParsed === null ? { action: "sibling_rank", remove: true } : { action: "sibling_rank", siblingRank: siblingRankParsed }, siblingRankParsed === null ? "清除同级稀疏 rank" : `把同级稀疏 rank 设为 ${siblingRankParsed}`); } }} /></label><button disabled={!metadataEditable || !siblingRankValid} onClick={() => void previewNodeMetadata(siblingRankParsed === null ? { action: "sibling_rank", remove: true } : { action: "sibling_rank", siblingRank: siblingRankParsed }, siblingRankParsed === null ? "清除同级稀疏 rank" : `把同级稀疏 rank 设为 ${siblingRankParsed}`)}>{siblingRankParsed === null ? "预览清除" : "预览 rank 事务"}</button></div>}
                {nodeMetadata.diagnostics.map((diagnostic) => <button className="property-diagnostic" key={`${diagnostic.field}-${diagnostic.range.start}`} onClick={() => chooseView("source")}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span></button>)}
                {!metadataEditable && <p className="safety-note">{currentSource !== liveDocument?.source ? "当前有未保存草稿；先保存或放弃后才能修改系统元数据。" : "当前节点元数据只读。"}</p>}
              </div> : <p className="empty-properties">连接新版 Core 后显示节点 ID、别名、图标和排序。</p>}
              <div className="properties-heading document-properties-heading"><span>文档属性</span><small>AsciiDoc 文档头 attribute · 不写入 YAML</small></div>
              <div className="properties-content">{properties.length ? properties.map((property) => <div className="property-row" key={`${property.name}-${property.range.start}`}><strong>{property.name}</strong><span>{property.value}</span>{liveWorkspace ? <button aria-label={`编辑属性 ${property.name}`} onClick={() => openPropertyEditor(property.name, property.value)}>编辑</button> : <button aria-label={`在源码中编辑属性 ${property.name}`} onClick={() => chooseView("source")}>源码</button>}</div>) : <span className="empty-properties">没有可投影的文档头属性</span>}{activePropertyAnalysis?.diagnostics.map((diagnostic, index) => <button className="property-diagnostic" key={`${diagnostic.code}-${diagnostic.range.start}-${index}`} onClick={() => chooseView("source")}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span></button>)}<button className="add-property" disabled={!liveWorkspace} onClick={() => openPropertyEditor()}>{liveWorkspace ? "＋ 新增文档头属性" : "连接 Core 后编辑"}</button></div>
            </section>}
            {inspectorTab === "tasks" && <section className="task-inspector inspector-section" aria-label="任务">
              <TaskSurface
                enabled={taskAvailable}
                nodeId={liveDocument?.nodeId ?? selected}
                workspaceRevision={liveWorkspace?.revision ?? ""}
                documentRevision={liveDocument?.revision ?? ""}
                blockedReason={taskBlockedReason}
                safeMode={safeMode}
                request={sharedCoreRequest}
                onCommitted={applyTaskCommit}
              />
            </section>}
            {inspectorTab === "citations" && <section className="citation-inspector inspector-section" aria-label="引用与参考文献">
              {citationAvailable ? <>
                <div className="citation-profile-controls"><label>样式<select aria-label="引用样式" value={citationStyle} onChange={(event) => setCitationStyle(event.target.value)}><option value="apa">APA</option><option value="vancouver">Vancouver</option><option value="chicago-notes">Chicago Notes</option></select></label><label>语言<select aria-label="引用语言" value={citationLocale} onChange={(event) => setCitationLocale(event.target.value)}><option value="en-US">English (US)</option><option value="zh-CN">简体中文</option><option value="ar">العربية</option></select></label></div>
                <div className="citation-summary"><div><span>引用</span><strong>{citationComponent?.citations.length ?? 0}</strong></div><div><span>书目条目</span><strong>{citationComponent?.bibliography?.entries.length ?? 0}</strong></div><div><span>诊断</span><strong>{citationDiagnostics.length}</strong></div></div>
                {citationData && <div className="reference-summary"><span>历史参考文献记录（只读）</span><strong>{citationData.title}</strong><code>@{citationData.key} · {citationData.itemType}</code></div>}
                {citationComponent?.citations.map((citation, index) => <button className="rendered-citation-row" key={`${citation.sourceRange.start}-${index}`} onClick={() => focusCitationRange(citation.sourceRange)}><span>{renderCitationRichText(citation.content, `inspector-citation-${index}`)}</span><small>{citation.form === "narrative" ? "叙述式" : "括号式"} · {citation.referenceNodeIds.length} 项</small></button>)}
                {citationDiagnostics.length > 0 ? <div className="citation-diagnostics" aria-label="引用诊断">{citationDiagnostics.map((diagnostic, index) => <button key={`${diagnostic.code}-${diagnostic.range?.start ?? "document"}-${index}`} onClick={() => focusCitationRange(diagnostic.range)}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>{diagnostic.range ? `字节 ${diagnostic.range.start}–${diagnostic.range.end}` : "文档级诊断"}</small></button>)}</div> : <p className="citation-clean">✓ 当前精确草稿可解析、解析引用并生成呈现模型。</p>}
                {citationAnalysisError && <p className="citation-analysis-error" role="alert">{citationAnalysisError}</p>}
                <div className="citation-inspector-actions"><button onClick={openCitationDialog}>插入引用</button></div>
                <p className="safety-note">{REFERENCE_RECORD_READ_ONLY_NOTICE}</p>
              </> : <p className="empty-properties">引用检查器只对已连接的 Weftext AsciiDoc 工作区开放。</p>}
            </section>}
            {inspectorTab === "backlinks" && <section className="inspector-section" aria-label="反向链接">
              {liveWorkspace ? <div className="link-evidence"><div><span>出链</span><strong>{selectedOutgoing.length}</strong></div><div><span>反链</span><strong>{selectedBacklinks.length}</strong></div><div><span>潜在链接</span><strong>{selectedMentions.length}</strong></div>{selectedMentions.slice(0, 6).map((mention) => <button key={`${mention.matchedText}-${mention.matchedScalarLength}`}><strong>{mention.matchedText}</strong><small>最长匹配 · {mention.targetNodeIds.length} 个候选</small></button>)}</div> : <p className="empty-properties">连接 Core 后显示链接证据。</p>}
            </section>}
            {inspectorTab === "annotations" && <section className="inspector-section" aria-label="批注">
              <div className="review-filter"><button className="active">未解决 {unresolvedAnnotations}</button><button>全部 {commentCount}</button></div>
              {annotations.map((annotation) => {
                const suggestion = annotation.kind === "suggestion_insert" || annotation.kind === "suggestion_delete";
                const accepted = annotation.resolution === "accepted";
                return <div className={`annotation-card ${annotation.resolved ? "resolved" : annotation.state === "orphaned" ? "orphaned" : "active"}`} key={annotation.id}>
                  <div className="annotation-author"><span className={`mini-avatar ${annotation.resolved ? "gold" : ""}`}>{annotation.avatar}</span><strong>{annotation.author}</strong><time>{annotation.time}</time></div>
                  <div className="annotation-appearance"><span>{annotation.kind?.replaceAll("_", " ") ?? "comment"}</span><small>{annotation.targetKind?.replaceAll("_", " ")}</small>{annotation.mark && <><i className={`annotation-color ${annotation.color ?? "yellow"}`} /><em>{annotation.mark}</em></>}{annotation.labels?.map((label) => <em key={label}>{label}</em>)}</div>
                  {annotation.state === "orphaned" && <p role="status">目标无法唯一重锚；Core 未猜测位置。</p>}
                  {annotation.suggestedSource && <pre className="annotation-suggestion">{annotation.suggestedSource}</pre>}
                  {annotation.messages?.length ? annotation.messages.map((message) => <div className="annotation-message" key={message.id}><small>{message.authorName}</small><p>{message.body}</p>{liveWorkspace && message.authorId === annotationActor.id && <button onClick={() => openAnnotationEditor("edit_message", annotation.id, message.id)}>编辑消息</button>}</div>) : annotation.body && <p>{annotation.body}</p>}
                  <div className="annotation-actions">
                    {liveWorkspace && !accepted && <button onClick={() => openAnnotationEditor(annotation.resolved ? "reopen" : "resolve", annotation.id)}>{annotation.resolved ? "重新打开" : "解决"}</button>}
                    {!annotation.resolved && <button onClick={() => liveWorkspace ? openAnnotationEditor("reply", annotation.id) : setToast("连接 Core 后回复批注")}>回复</button>}
                    {liveWorkspace && <button onClick={() => openAnnotationEditor("set_appearance", annotation.id)}>外观</button>}
                    {liveWorkspace && <button onClick={() => openAnnotationEditor("set_labels", annotation.id)}>标签</button>}
                    {liveWorkspace && annotation.state === "orphaned" && <button onClick={() => openAnnotationEditor("reanchor", annotation.id)}>重新定位</button>}
                    {liveWorkspace && suggestion && annotation.state === "open" && <><button onClick={() => openAnnotationEditor("accept_suggestion", annotation.id)}>接受建议</button><button onClick={() => openAnnotationEditor("reject_suggestion", annotation.id)}>拒绝建议</button></>}
                    {annotation.resolved && <span className="resolved-label">✓ {annotation.resolution === "accepted" ? "已接受" : annotation.resolution === "rejected" ? "已拒绝" : "已解决"}</span>}
                  </div>
                </div>;
              })}
              {commentCount === 0 && <p className="empty-properties">当前节点没有批注。</p>}
              <button className="new-comment" onClick={() => openAnnotationEditor("create")}>＋ 添加批注或建议</button>
            </section>}
          </aside>}
        </div>

        <footer className="statusbar">
          <button className={saved ? "saved" : draftSaveState === "error" ? "draft-error" : "unsaved"} onClick={() => void requestSavePreview()}><i className="ok-dot" />{draftStatusLabel}</button>
          <span>{textCount} 字</span><span>第 {cursor.line} 行，第 {cursor.column} 列</span><span className="status-spacer" /><span>{new TextEncoder().encode(currentSource).length} 字节</span><span>UTF-8</span><span>{activeProfileName}</span>
        </footer>
      </section>

      {dialog === "search" && <div className="modal-backdrop" role="presentation"><section className="command-palette" role="dialog" aria-modal="true" aria-label="搜索工作区">
        <div className="command-input"><span>⌕</span><input ref={searchInputRef} value={query} onChange={(event) => { setQuery(event.target.value); setWorkspaceSearch([]); setSearchIndex(0); }} onKeyDown={(event) => {
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setSearchIndex((current) => results.length ? (current + 1) % results.length : 0);
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setSearchIndex((current) => results.length ? (current - 1 + results.length) % results.length : 0);
          } else if (event.key === "Enter" && results[effectiveSearchIndex]) {
            event.preventDefault();
            if (searchCreatesTab) void openNode(results[effectiveSearchIndex].id, { newTab: true });
            else void openExplorerNode(results[effectiveSearchIndex].id);
          }
        }} placeholder="搜索节点、用户属性或正文…" /><kbd>Esc</kbd></div>
        <div className="command-label">节点</div>
        {derivedIndexWarning && <div className="search-index-warning" role="alert">搜索索引暂不可用：{derivedIndexWarning.message}</div>}
        <div className="search-results">{results.map((node, index) => <button key={node.id} onMouseEnter={() => setSearchIndex(index)} onClick={() => searchCreatesTab ? void openNode(node.id, { newTab: true }) : void openExplorerNode(node.id)} className={index === effectiveSearchIndex ? "selected" : ""}>{node.icon && iconPreferences.placement === "before" && <span aria-hidden="true">{node.icon.glyph}</span>}<div><strong>{node.name}{node.icon && iconPreferences.placement === "after" && <span aria-hidden="true">{node.icon.glyph}</span>}</strong><small>{"snippet" in node ? `${node.path} · ${node.snippet}` : `${workspaceName} / ${node.name}`}</small></div><kbd>↵</kbd></button>)}</div>
        <footer><span>↑↓ 选择</span><span>Enter 打开</span><span>Esc 关闭</span></footer>
      </section></div>}

      {dialog === "query" && <div className="modal-backdrop query-modal-backdrop" role="presentation"><section className="dialog-card query-dialog" role="dialog" aria-modal="true" aria-labelledby="query-title">
        <header className="query-dialog-heading"><div><span className="eyebrow">UNIFIED QUERY</span><h2 id="query-title">查询与派生表格</h2><p>规范查询块由 Core 解析并在授权过滤后执行；当前切片提供只读表格与同一结果的 Core CSV。</p></div><button type="button" aria-label="关闭查询与视图" onClick={() => setDialog(null)}>×</button></header>
        <QuerySurface
          enabled={taskAvailable}
          nodeId={liveDocument?.nodeId ?? selected}
          documentSource={currentSource}
          request={sharedCoreRequest}
          onOpenNode={async (nodeId) => { setDialog(null); await openExplorerNode(nodeId); }}
        />
      </section></div>}

      {dialog === "intake" && <div className="modal-backdrop intake-modal-backdrop" role="presentation"><IntakeSurface
        enabled={desktopMode && taskAvailable}
        safeMode={safeMode}
        blockedReason={intakeBlockedReason}
        destinationParentId={liveDocument?.nodeId ?? liveWorkspace?.rootNodeId ?? ""}
        destinationParentName={liveDocument?.name ?? workspaceName}
        workspaceRevision={liveWorkspace?.revision ?? ""}
        request={sharedCoreRequest}
        chooseTaskReceiptDestination={async (suggestedName) => window.weftextDesktop?.chooseTaskImportReceiptDestination(suggestedName) ?? null}
        onCommitted={applyImportCommit}
        onClose={() => setDialog(null)}
      /></div>}

      {dialog === "export" && liveDocument && <div className="modal-backdrop intake-modal-backdrop" role="presentation"><ExportSurface
        enabled={desktopMode && taskAvailable}
        safeMode={safeMode}
        blockedReason={exportBlockedReason}
        nodeId={liveDocument.nodeId}
        nodeName={liveDocument.name}
        request={sharedCoreRequest}
        chooseDestination={async (suggestedName) => window.weftextDesktop?.chooseMarkdownExportDestination(suggestedName) ?? null}
        onCommitted={(receipt) => setToast(`Markdown 已发布并校验：${receipt.artifactByteLength.toLocaleString()} 字节`)}
        onClose={() => setDialog(null)}
      /></div>}

      {dialog === "node_actions" && nodeActionMenuNode && nodeActionInvocation && <div className="modal-backdrop" role="presentation"><section className="dialog-card action-chooser" role="dialog" aria-modal="true" aria-labelledby="node-actions-title">
        <span className="eyebrow">节点操作</span><h2 id="node-actions-title" dir="auto">“{nodeActionMenuNode.name}”的操作</h2><p>目标节点已在打开此菜单时按 UUID 固定；后续预览和提交不会跟随选择、焦点或第二栏变化。</p>
        <div className="action-choice-list">
          <button onClick={() => beginNodeAction("create")}>{WORKSPACE_ACTION_REGISTRY.create.label}<small>以固定节点作为父节点</small></button>
          <button disabled={nodeActionMenuNode.id === liveWorkspace?.rootNodeId} onClick={() => beginNodeAction("rename")}>{WORKSPACE_ACTION_REGISTRY.rename.label}<small>只更改当前节点名称</small></button>
          <button disabled={nodeActionMenuNode.id === liveWorkspace?.rootNodeId} onClick={() => beginNodeAction("move")}>{WORKSPACE_ACTION_REGISTRY.move.label}<small>保留分支中的永久节点身份</small></button>
          <button disabled={nodeActionMenuNode.id === liveWorkspace?.rootNodeId} onClick={() => beginNodeAction("copy")}>{WORKSPACE_ACTION_REGISTRY.copy.label}<small>为完整副本重新分配节点身份</small></button>
          <button className="danger" disabled={nodeActionMenuNode.id === liveWorkspace?.rootNodeId || safeMode || liveWorkspace?.trashReconciliation?.required || liveWorkspace?.trashLegacyMigrationRequired} onClick={() => beginNodeAction("trash_node")}>{WORKSPACE_ACTION_REGISTRY.trash_node.label}<small>一个根分支生成一个可恢复条目</small></button>
          <button onClick={() => beginResourceTrash()}>{WORKSPACE_ACTION_REGISTRY.trash_resource.label}<small>每个文件都是独立可恢复条目</small></button>
          <button onClick={() => beginNodeAction("chrono")}>{WORKSPACE_ACTION_REGISTRY.chrono.label}<small>以固定节点作为时间节点根</small></button>
        </div>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button></div>
      </section></div>}

      {dialog === "new" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="new-node-title">
        <span className="eyebrow">新建子节点</span><h2 id="new-node-title">新建节点</h2><p>将在固定父节点“{nodeActionTargetNode.name}”下创建同名目录与 {activeProfileName} 文档。真实工作区会先生成事务预览。</p>
        <label>节点名称<input value={newName} onChange={(event) => setNewName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") createNode(); }} placeholder="例如：会议记录" /></label>
        <div className="path-preview"><span>预览</span><code>{liveWorkspace?.documentFormat ? `${newName || "会议记录"}/${newName || "会议记录"}.${liveWorkspace.documentFormat.canonicalExtension}` : demo ? `${newName || "会议记录"}/${newName || "会议记录"}.${demo.documentFormat.canonicalExtension}` : "由 Core 事务计划决定"}</code></div>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="primary" onClick={createNode}>创建节点</button></div>
      </section></div>}

      {dialog === "property" && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="property-title">
        <span className="eyebrow">LOSSLESS PROPERTIES</span><h2 id="property-title">{propertyExisting ? "编辑文档属性" : "新增文档属性"}</h2><p>属性写入 AsciiDoc 文档头。Core 只替换目标值的确切字节；重复名、续行值或处理器控制属性会拒绝修改。结果先进入设备草稿，仍需保存预览。</p>
        <label>属性名<input value={propertyKey} disabled={propertyExisting} onChange={(event) => setPropertyKey(event.target.value)} placeholder="例如：status" /></label>
        <label>字符串值<input value={propertyValue} onChange={(event) => setPropertyValue(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") void applyPropertyPatch(); }} placeholder="例如：in-progress" /></label>
        <div className="dialog-actions">{propertyExisting && <button className="danger" onClick={() => void applyPropertyPatch(true)}>移除属性</button>}<button onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={!propertyKey.trim() || !propertyValue.trim()} onClick={() => void applyPropertyPatch()}>应用到草稿</button></div>
      </section></div>}

      {dialog === "icon" && <div className="modal-backdrop" role="presentation"><section className="dialog-card icon-picker" role="dialog" aria-modal="true" aria-labelledby="icon-title">
        <span className="eyebrow">PORTABLE NODE ICON</span><h2 id="icon-title">节点图标</h2><p><code>weftext.icon</code> 只接受单个 emoji 或 Weftext 内置 token，由 Core 做严格校验与窄范围修改。</p>
        <label>搜索图标<input aria-label="搜索图标" value={iconQuery} onChange={(event) => setIconQuery(event.target.value)} placeholder="搜索 emoji 或内置符号…" /></label>
        <div className="icon-grid" role="group" aria-label="可用节点图标">{iconOptions.map((icon) => <button key={icon.value} aria-label={`选择图标 ${icon.label}`} aria-pressed={activeIcon?.value === icon.value} onClick={() => void applyPortableIcon(icon)}><span aria-hidden="true">{icon.glyph}</span><small>{icon.label}</small></button>)}</div>
        {!iconOptions.length && <p className="empty-properties">没有匹配的受支持图标。</p>}
        <div className="icon-preferences"><label>紧凑列表位置<select aria-label="节点图标列表位置" value={iconPreferences.placement} onChange={(event) => setIconPreferences((current) => ({ ...current, placement: event.target.value as IconPlacement }))}><option value="before">名称前</option><option value="after">名称后</option><option value="hidden">隐藏</option></select></label><label className="toggle-row"><input type="checkbox" checked={iconPreferences.showInTitle} onChange={(event) => setIconPreferences((current) => ({ ...current, showInTitle: event.target.checked }))} />在文档标题显示图标</label></div>
        <div className="dialog-actions"><button className="danger" disabled={!activeIcon} onClick={() => void applyPortableIcon(null)}>清除便携图标</button><button onClick={() => { setIconQuery(""); setDialog(null); }}>关闭</button></div>
      </section></div>}

      {dialog === "citation" && <div className="modal-backdrop" role="presentation"><section className="dialog-card citation-dialog" role="dialog" aria-modal="true" aria-labelledby="citation-title">
        <span className="eyebrow">CORE CITATION AUTHORING</span><h2 id="citation-title">插入引用</h2>
        <p>搜索与表单只提交节点 UUID 和编辑意图；引用解析、源码补丁和最终呈现都由 Core 决定。</p>
        <p className="safety-note">{REFERENCE_RECORD_READ_ONLY_NOTICE}</p>

        <div className="citation-dialog-body">
          <div className="citation-choice-row" role="group" aria-label="插入类型">{(["citation", "nocite", "bibliography"] as const).map((action) => <button key={action} aria-pressed={citationAction === action} onClick={() => setCitationAction(action)}>{{ citation: "正文引用", nocite: "仅加入书目", bibliography: "参考文献表" }[action]}</button>)}</div>
          {citationAction === "citation" && <label>引用形式<select value={citationForm} onChange={(event) => setCitationForm(event.target.value as "parenthetical" | "narrative")}><option value="parenthetical">括号式</option><option value="narrative">叙述式</option></select></label>}
          {citationAction !== "bibliography" ? <>
            {citationItems.length > 0 && <div className="citation-item-list" aria-label="当前引用项">{citationItems.map((item, index) => <div key={`${item.referenceNodeId}-${index}`}><span><strong>{item.title}</strong><small>@{item.key}{item.locator ? ` · ${item.label ?? "page"} ${item.locator}` : ""}</small></span><span><button aria-label={`上移 ${item.title}`} disabled={index === 0} onClick={() => moveCitationItem(index, -1)}>↑</button><button aria-label={`下移 ${item.title}`} disabled={index === citationItems.length - 1} onClick={() => moveCitationItem(index, 1)}>↓</button><button aria-label={`移除 ${item.title}`} onClick={() => setCitationItems((current) => current.filter((_, itemIndex) => itemIndex !== index))}>×</button></span></div>)}</div>}
            <label>搜索参考文献<input role="combobox" aria-label="搜索参考文献" aria-controls="citation-reference-results" aria-expanded={Boolean(citationQuery.trim())} aria-activedescendant={citationHits[citationSearchIndex] ? `citation-reference-${citationHits[citationSearchIndex].nodeId}` : undefined} aria-autocomplete="list" value={citationQuery} onChange={(event) => { setCitationQuery(event.target.value); setCitationSelectedReference(null); setCitationSearchIndex(0); }} onKeyDown={(event) => { if (event.nativeEvent.isComposing) return; if (event.key === "ArrowDown") { event.preventDefault(); setCitationSearchIndex((current) => citationHits.length ? (current + 1) % citationHits.length : 0); } else if (event.key === "ArrowUp") { event.preventDefault(); setCitationSearchIndex((current) => citationHits.length ? (current - 1 + citationHits.length) % citationHits.length : 0); } else if (event.key === "Enter" && citationHits[citationSearchIndex]?.selectable) { event.preventDefault(); setCitationSelectedReference(citationHits[citationSearchIndex].nodeId); } }} placeholder="标题、作者、DOI、ISBN 或引用键…" /></label>
            <div id="citation-reference-results" className="citation-search-results" role="listbox" aria-label="参考文献搜索结果">{citationHits.map((hit, index) => <button id={`citation-reference-${hit.nodeId}`} className={citationSearchIndex === index ? "active" : ""} key={hit.nodeId} role="option" aria-selected={citationSelectedReference === hit.nodeId} disabled={!hit.selectable} onMouseEnter={() => setCitationSearchIndex(index)} onClick={() => setCitationSelectedReference(hit.nodeId)}><strong>{hit.title}</strong><span>@{hit.key} · {hit.itemType}</span><small>{hit.contributors.join(" · ") || hit.identifiers.DOI || hit.identifiers.ISBN || hit.matchedFields.join(" · ")}</small>{!hit.selectable && <em>键冲突，不能选择</em>}</button>)}{citationQuery.trim() && !citationHits.length && <p aria-live="polite">没有匹配的可见参考文献。</p>}{!citationQuery.trim() && <p>输入查询后由 Core 搜索当前完整工作区范围。</p>}<span className="sr-only" aria-live="polite">{citationQuery.trim() ? `${citationHits.length} 个参考文献结果` : ""}</span></div>
            {citationAction === "citation" && <div className="citation-locator-grid"><label>定位类型<select value={citationLabel} onChange={(event) => setCitationLabel(event.target.value)}><option value="page">页</option><option value="chapter">章</option><option value="section">节</option><option value="figure">图</option><option value="paragraph">段</option></select></label><label>定位值<input value={citationLocator} onChange={(event) => setCitationLocator(event.target.value)} placeholder="例如 42–44" /></label><label>前缀<input value={citationPrefix} onChange={(event) => setCitationPrefix(event.target.value)} placeholder="例如 参见" /></label><label>后缀<input value={citationSuffix} onChange={(event) => setCitationSuffix(event.target.value)} placeholder="例如 强调处" /></label></div>}
            <div className="citation-inline-actions"><button disabled={!citationSelectedReference || (citationAction === "citation" && citationForm === "narrative" && citationItems.length > 0)} onClick={addCitationItem}>{citationAction === "nocite" ? "加入 nocite 列表" : "加入引用项"}</button></div>
          </> : <label>收录范围<select value={bibliographyInclusion} onChange={(event) => setBibliographyInclusion(event.target.value as "cited" | "all")}><option value="cited">只收录已引用和 nocite 项</option><option value="all">收录所有参考文献节点</option></select></label>}
          {citationComponent && <div className="citation-current-render"><span>当前 Core 呈现</span>{citationComponent.citations.slice(0, 3).map((citation, index) => <div key={`${citation.sourceRange.start}-${index}`}>{renderCitationRichText(citation.content, `dialog-citation-${index}`)}</div>)}{citationComponent.bibliography && <small>{citationComponent.bibliography.entries.length} 条书目项 · {activeCitationDraft?.presentation?.profile.styleId} / {activeCitationDraft?.presentation?.profile.locale}</small>}</div>}
          <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={safeMode || (citationAction !== "bibliography" && !citationSelectedReference && citationItems.length === 0)} onClick={() => void applyCitationMacro()}>{safeMode ? "安全模式：编辑已暂停" : citationEditRange ? "替换当前 Core 引用面" : "插入到精确草稿"}</button></div>
        </div>
        {citationAnalysisError && <p className="citation-analysis-error" role="alert">{citationAnalysisError}</p>}
      </section></div>}

      {dialog === "resource" && resourcePlan && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="resource-title">
        <span className="eyebrow">CORE RESOURCE PLAN</span><h2 id="resource-title">导入图片资源</h2><p>文件将成为当前节点拥有的普通资源。Core 会在提交前复核工作区 revision、文件名和目标冲突；不会创建松散 canonical 文档。</p>
        <div className="path-preview"><span>节点资源</span><code>{resourcePlan.name}</code><small>{resourcePlan.byteLength.toLocaleString()} 字节</small></div>
        <div className="dialog-actions"><button onClick={() => { setResourcePlan(null); setDialog(null); }}>取消</button><button className="primary" disabled={safeMode} onClick={() => void commitImageImport()}>{safeMode ? "安全模式：提交已暂停" : "写入资源并插入引用"}</button></div>
      </section></div>}

      {dialog === "annotation" && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="annotation-title">
        <span className="eyebrow">ANNOTATION V3 TRANSACTION</span><h2 id="annotation-title">{{ create: "添加批注或建议", reply: "回复批注", edit_message: "编辑自己的消息", set_appearance: "更新批注外观", set_labels: "更新批注标签", resolve: "解决批注", reopen: "重新打开批注", reanchor: "确定性重新定位", accept_suggestion: "接受修改建议", reject_suggestion: "拒绝修改建议" }[annotationAction]}</h2><p>{annotationAction === "create" ? "文本选择使用精确 UTF-8 range；空选择使用当前 Core 块。插入建议使用光标位置，删除建议要求非空选择。" : "批注 ID、消息作者、锚点证据和 sidecar 外键将在提交前由 Core 重新验证。"}</p>
        {annotationAction === "create" && <label>类型<select aria-label="批注类型" value={annotationCreateKind} onChange={(event) => setAnnotationCreateKind(event.target.value as AnnotationKind)}><option value="comment">评论</option><option value="mark">仅标记</option><option value="suggestion_insert">插入建议</option><option value="suggestion_delete">删除建议</option></select></label>}
        {(annotationAction === "create" || annotationAction === "reply") && <label>显示名快照<input value={annotationActor.name} onChange={(event) => setAnnotationActor((current) => ({ ...current, name: event.target.value }))} placeholder="例如 Zhang San" /></label>}
        {(annotationAction === "reply" || annotationAction === "edit_message" || (annotationAction === "create" && annotationCreateKind !== "mark")) && <label>{annotationCreateKind === "comment" || annotationAction !== "create" ? "内容" : "说明（可选）"}<textarea aria-label="批注内容" value={annotationBody} onChange={(event) => setAnnotationBody(event.target.value)} placeholder="输入受限 AsciiDoc 行内内容…" /></label>}
        {annotationAction === "create" && annotationCreateKind === "suggestion_insert" && <label>拟插入的 AsciiDoc 源<textarea aria-label="拟插入源码" value={annotationSuggestedSource} onChange={(event) => setAnnotationSuggestedSource(event.target.value)} /></label>}
        {annotationAction === "create" && annotationCreateKind === "suggestion_delete" && !annotationSelectionIsRange && <p className="safety-note" role="alert">删除建议需要先在文档中选择一个非空文本范围。</p>}
        {(annotationAction === "set_appearance" || annotationAction === "create") && <div className="annotation-options"><label>标记<select aria-label="批注标记" value={annotationMark} onChange={(event) => setAnnotationMark(event.target.value)}><option value="highlight">高亮</option><option value="underline">下划线</option><option value="squiggle">波浪线</option><option value="strike">删除线</option><option value="none">无视觉标记</option></select></label><label>主题<select aria-label="批注主题" value={annotationColor} disabled={annotationMark === "none"} onChange={(event) => setAnnotationColor(event.target.value)}><option value="yellow">黄色</option><option value="red">红色</option><option value="green">绿色</option><option value="blue">蓝色</option><option value="purple">紫色</option><option value="pink">粉色</option><option value="gray">灰色</option></select></label></div>}
        {(annotationAction === "create" || annotationAction === "set_labels") && <label>标签<input aria-label="批注标签" value={annotationLabels} onChange={(event) => setAnnotationLabels(event.target.value)} placeholder="question, verify" /></label>}
        {annotationPlan && <div className="core-connection-note"><span>预览</span><strong>工作区版本 {annotationPlan.baseRevision.slice(0, 12)} · 固定批注文件</strong></div>}
        <div className="dialog-actions"><button onClick={() => { setAnnotationPlan(null); setDialog(null); }}>取消</button>{annotationPlan ? <button className="primary" disabled={safeMode} onClick={() => void commitAnnotationAction()}>{safeMode ? "安全模式：提交已暂停" : "确认提交批注事务"}</button> : <button className="primary" disabled={!annotationCanPreview} onClick={() => void previewAnnotationAction()}>生成事务预览</button>}</div>
      </section></div>}

      {dialog === "chrono" && nodeActionTarget?.kind === "node" && nodeActionTarget.action === "chrono" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card" role="dialog" aria-modal="true" aria-labelledby="chrono-title">
        <span className="eyebrow">时间节点</span><h2 id="chrono-title">创建时间节点</h2><p>以固定节点“{nodeActionTargetNode.name}”为时间节点根，使用规范名称创建缺失节点。实际写入前仍会展示完整事务预览。</p>
        <label>日期<input type="date" value={chronoDate} onChange={(event) => setChronoDate(event.target.value)} /></label>
        <div className="chrono-grid">
          {(["year", "quarter", "month", "week", "day"] as const).map((period) => <button key={period} onClick={() => void previewChrono(period, nodeActionTarget)}><span>{{ year: "年", quarter: "季度", month: "月", week: "ISO 周", day: "日" }[period]}</span><strong>{chronoNodeName(period, chronoDate)}</strong></button>)}
        </div>
        <div className="chrono-path">{liveWorkspace ? `${nodeActionTargetNode.name} / ` : "时间节点 / "}<strong>{chronoDate}</strong></div>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>关闭</button></div>
      </section></div>}

      {dialog === "conflict" && <div className="modal-backdrop" role="presentation"><section className="dialog-card conflict-dialog" role="dialog" aria-modal="true" aria-labelledby="conflict-title">
        <div className="conflict-title"><span className="conflict-icon">!</span><div><span className="eyebrow">CONFLICT CENTER</span><h2 id="conflict-title">{coreError ? "Core 已拒绝这次操作" : "没有待处理冲突"}</h2></div></div>
        {coreError ? <><div className="conflict-item"><div><strong>{liveDocument?.name ?? workspaceName}</strong><span>{coreError}</span></div><span className="conflict-time">未写入</span></div><p className="safety-note">{desktopMode && draftSaveState === "saved" ? "当前草稿已写入设备恢复区" : "当前草稿仍保留在本次会话中"}，磁盘文档没有被这次请求覆盖。</p><div className="dialog-actions"><button onClick={() => setDialog(null)}>返回编辑</button><button className="primary" onClick={() => void openNode(selected, { refresh: true })}>{desktopMode ? "读取磁盘版本并比较" : "重新读取 Core"}</button></div></> : <div className="dialog-actions"><button className="primary" onClick={() => setDialog(null)}>关闭</button></div>}
      </section></div>}

      {dialog === "recovery" && <div className="modal-backdrop" role="presentation"><section className="dialog-card recovery-dialog" role="dialog" aria-modal="true" aria-labelledby="recovery-title">
        <div className="conflict-title"><span className="conflict-icon recovery-icon">↶</span><div><span className="eyebrow">RECOVERY CENTER</span><h2 id="recovery-title">草稿恢复中心</h2></div></div>
        <p>设备草稿保存在应用配置目录，不进入工作区。恢复内容提交前仍需通过当前 Core profile 与 revision 检查。</p>
        {draftRecovery.issues.length > 0 && <div className="recovery-issues" role="alert">{draftRecovery.issues.map((issue) => <div key={issue}><strong>需要处理</strong><span>{issue}</span></div>)}</div>}
        {draftRecovery.drafts.length > 0 ? <div className="recovery-list" aria-label="可恢复草稿">{draftRecovery.drafts.map((draft) => <button key={draft.nodeId} className={draft.nodeId === selected ? "active" : ""} onClick={() => void openNode(draft.nodeId, { refresh: true })}><div><strong>{draft.name}</strong><span>{draft.length} 字节 · {new Date(draft.updatedAtUnixMs).toLocaleString()}</span></div><em>{draft.stale ? "磁盘已变化" : "可直接恢复"}</em></button>)}</div> : draftRecovery.issues.length === 0 && <p className="empty-properties">当前没有设备恢复草稿。</p>}
        {currentRecovery && liveDocument && <div className="recovery-detail">
          <div className="recovery-detail-title"><strong>{currentRecovery.name}</strong><span className={currentRecovery.stale ? "stale" : "current"}>{currentRecovery.stale ? "基于旧 revision，需要选择" : "基于当前 revision"}</span></div>
          {currentRecoverySource !== undefined && <div className="recovery-compare"><div><span>磁盘版本 · {liveDocument.revision.slice(0, 12)}</span><pre>{liveDocument.source}</pre></div><div><span>设备草稿 · {currentRecovery.baseRevision.slice(0, 12)}</span><pre>{currentRecoverySource}</pre></div></div>}
          <div className="dialog-actions"><button onClick={() => void discardPersistentDraft(currentRecovery.nodeId, true)}>使用磁盘版本</button>{currentRecovery.stale ? <button className="primary" onClick={recoverPersistentDraft}>恢复草稿继续编辑</button> : <button className="primary" onClick={() => setDialog(null)}>继续编辑草稿</button>}</div>
        </div>}
        {!currentRecovery && <div className="dialog-actions"><button className="primary" onClick={() => setDialog(null)}>关闭</button></div>}
      </section></div>}

      {dialog === "rename" && nodeActionTarget?.kind === "node" && nodeActionTarget.action === "rename" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="rename-title">
        <span className="eyebrow">重命名</span><h2 id="rename-title">重命名当前节点</h2><p>只更改固定 UUID 节点“{nodeActionTargetNode.name}”的名称；父节点不会改变。</p>
        <label>新名称<input aria-label="节点新名称" value={moveName} onChange={(event) => setMoveName(event.target.value)} /></label>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={!liveWorkspace || !moveName.trim() || moveName === nodeActionTargetNode.name} onClick={() => void previewWorkspaceAction({ action: "rename", nodeId: nodeActionTarget.nodeId, name: moveName.trim() }, nodeActionTarget)}>预览重命名</button></div>
      </section></div>}

      {dialog === "move" && nodeActionTarget?.kind === "node" && nodeActionTarget.action === "move" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card" role="dialog" aria-modal="true" aria-labelledby="move-title">
        <span className="eyebrow">移动节点分支</span><h2 id="move-title">移动整个节点分支</h2><p>完整分支包括同名规范文档、所有后代、节点资源和批注 sidecar。名称保持“{nodeActionTargetNode.name}”。</p>
        <label>目标父节点<select aria-label="移动目标父节点" value={moveParent} onChange={(event) => setMoveParent(event.target.value)}>{moveTargets.map((node) => <option key={node.id} value={node.id}>{node.path || node.name}</option>)}</select></label>
        <p className="safety-note">Core 会验证目标父节点是否合法；操作范围始终是上述完整节点分支，并保留永久节点 UUID。</p>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={!liveWorkspace || !moveParent || moveParent === nodeActionTargetNode.parentId} onClick={() => void previewWorkspaceAction({ action: "move", nodeId: nodeActionTarget.nodeId, parentId: moveParent, name: nodeActionTargetNode.name }, nodeActionTarget)}>预览移动整个分支</button></div>
      </section></div>}

      {dialog === "copy" && nodeActionTarget?.kind === "node" && nodeActionTarget.action === "copy" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card" role="dialog" aria-modal="true" aria-labelledby="copy-title">
        <span className="eyebrow">复制节点分支</span><h2 id="copy-title">复制整个节点分支</h2><p>完整复制同名规范文档、所有后代、节点资源和批注 sidecar；副本中的每个节点都会获得新的永久 UUID。</p>
        <label>目标父节点<select aria-label="复制目标父节点" value={moveParent} onChange={(event) => setMoveParent(event.target.value)}>{moveTargets.map((node) => <option key={node.id} value={node.id}>{node.path || node.name}</option>)}</select></label>
        <label>副本名称<input aria-label="节点副本名称" value={moveName} onChange={(event) => setMoveName(event.target.value)} /></label>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="primary" disabled={!liveWorkspace || !moveParent || !moveName.trim()} onClick={() => void previewWorkspaceAction({ action: "copy", nodeId: nodeActionTarget.nodeId, parentId: moveParent, name: moveName.trim() }, nodeActionTarget)}>预览复制整个分支</button></div>
      </section></div>}

      {dialog === "node_trash" && nodeActionTarget?.kind === "node" && nodeActionTarget.action === "trash_node" && nodeActionTargetNode && <div className="modal-backdrop" role="presentation"><section className="dialog-card trash-dialog" role="dialog" aria-modal="true" aria-labelledby="node-trash-title">
        <span className="eyebrow">移入废纸篓</span><h2 id="node-trash-title">将整个节点分支移入废纸篓</h2><p>“{nodeActionTargetNode.name}”及其同名规范文档、全部后代、节点资源和批注 sidecar 会原名进入一个可恢复条目；后代不会另建条目。</p>
        <div className="core-connection-note"><span>条目数量</span><strong>1 个节点分支条目</strong></div>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="danger" disabled={!liveWorkspace || safeMode || liveWorkspace.trashReconciliation?.required || liveWorkspace.trashLegacyMigrationRequired} onClick={() => void previewNodeTrash(nodeActionTarget)}>预览移入废纸篓</button></div>
      </section></div>}

      {dialog === "resource_trash" && (nodeActionTarget?.kind === "node" || nodeActionTarget?.kind === "resource") && <div className="modal-backdrop" role="presentation"><section className="dialog-card trash-dialog" role="dialog" aria-modal="true" aria-labelledby="resource-trash-title">
        <span className="eyebrow">节点资源</span><h2 id="resource-trash-title">将节点拥有的资源移入废纸篓</h2><p>每个文件会成为可独立恢复的条目；一次批量操作的条目共享 operationId，且不会越过 unmanaged 或 ignored 边界。</p>
        <label>资源文件名（每行一个）<textarea aria-label="废纸篓资源文件名" value={trashResourceNames} onChange={(event) => setTrashResourceNames(event.target.value)} placeholder={"figure.png\nattachment.pdf"} /></label>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>取消</button><button className="danger" disabled={!liveWorkspace || safeMode || !trashResourceNames.trim() || liveWorkspace.trashReconciliation?.required || liveWorkspace.trashLegacyMigrationRequired} onClick={() => void previewTrashResources()}>预览资源条目</button></div>
      </section></div>}

      {dialog === "trash_item" && selectedTrashItem && <div className="modal-backdrop" role="presentation"><section className="dialog-card trash-dialog" role="dialog" aria-modal="true" aria-labelledby="trash-item-title">
        <span className="eyebrow">废纸篓条目</span><h2 id="trash-item-title" dir="auto">{selectedTrashItem.manifest.originalName}</h2>
        <p>恢复依据是条目 ID、永久节点/owner UUID 与 payload 摘要；显示名称和路径不作为恢复依据。</p>
        <div className="core-connection-note"><span>条目</span><strong>{selectedTrashItem.manifest.kind === "node" ? "完整节点分支" : "独立资源"} · {trashPayloadByteLength(selectedTrashItem.manifest).toLocaleString()} 字节</strong></div>
        {selectedTrashItem.restore.originResolution === "unknown" && <p className="safety-note">旧条目没有可信来源；默认继续留在废纸篓，必须显式选择现有目标。</p>}
        {selectedTrashItem.restore.blockedReason && <p className="safety-note" role="alert">{selectedTrashItem.restore.blockedReason}</p>}
        <div className="dialog-actions action-cluster"><button onClick={() => setDialog(null)}>关闭</button><button className="danger" disabled={selectedTrashItem.restore.originResolution === "reconciliation_required"} onClick={() => openTrashPermanentDelete(selectedTrashItem)}>{WORKSPACE_ACTION_REGISTRY.permanently_delete_item.label}</button><button className="primary" disabled={selectedTrashItem.restore.originResolution === "reconciliation_required"} onClick={() => openTrashRestore(selectedTrashItem)}>{WORKSPACE_ACTION_REGISTRY.restore_item.label}</button></div>
      </section></div>}

      {dialog === "trash_restore" && selectedTrashItem && nodeActionTarget?.kind === "trash_item" && nodeActionTarget.action === "restore_item" && <div className="modal-backdrop" role="presentation"><section className="dialog-card trash-dialog" role="dialog" aria-modal="true" aria-labelledby="trash-restore-title">
        <span className="eyebrow">恢复废纸篓条目</span><h2 id="trash-restore-title" dir="auto">恢复“{selectedTrashItem.manifest.originalName}”</h2>
        <label>恢复方式<select aria-label="废纸篓恢复方式" value={trashRestoreMode} onChange={(event) => setTrashRestoreMode(event.target.value as typeof trashRestoreMode)}>
          <option value="original" disabled={!selectedTrashItem.restore.originalAvailable}>恢复到原位置{selectedTrashItem.restore.originalAvailable ? "" : "（不可用）"}</option>
          <option value="with_ancestors" disabled={!selectedTrashItem.restore.withAncestorsAvailable}>先恢复父链再恢复此项{selectedTrashItem.restore.withAncestorsAvailable ? "" : "（不可用）"}</option>
          <option value="existing_target">选择现有目标</option>
        </select></label>
        {trashRestoreMode === "existing_target" && <><label>现有目标节点<select aria-label="废纸篓恢复目标" value={trashRestoreTarget} onChange={(event) => setTrashRestoreTarget(event.target.value)}><option value="">请选择现有目标…</option>{nodes.map((node) => <option key={node.id} value={node.id}>{node.path || node.name}</option>)}</select></label><label>恢复名称<input aria-label="废纸篓恢复名称" value={trashRestoreName} onChange={(event) => setTrashRestoreName(event.target.value)} /></label></>}
        {selectedTrashItem.restore.originResolution === "unknown" && <p className="safety-note">旧条目没有可信来源；默认继续留在废纸篓，必须选择现有目标，系统不会猜测或自动创建父节点。</p>}
        <div className="dialog-actions"><button onClick={() => setDialog("trash_item")}>返回</button><button className="primary" disabled={safeMode || selectedTrashItem.restore.originResolution === "reconciliation_required" || (trashRestoreMode === "existing_target" && (!trashRestoreTarget || !trashRestoreName.trim()))} onClick={() => void previewTrashRestore()}>{safeMode ? "安全模式：操作已暂停" : "预览恢复条目"}</button></div>
      </section></div>}

      {dialog === "trash_permanent" && selectedTrashItem && nodeActionTarget?.kind === "trash_item" && nodeActionTarget.action === "permanently_delete_item" && <div className="modal-backdrop" role="presentation"><section className="dialog-card trash-dialog permanent-delete-dialog" role="alertdialog" aria-modal="true" aria-labelledby="trash-permanent-title">
        <span className="eyebrow">高权限操作</span><h2 id="trash-permanent-title">永久删除废纸篓条目</h2><p>这不是恢复或清空废纸篓的附带选项。系统会先固定并展示精确条目 ID、摘要和字节，再要求二次确认。</p>
        <div className="core-connection-note"><span>将核对</span><strong>{selectedTrashItem.manifest.trashItemId} · {trashPayloadByteLength(selectedTrashItem.manifest).toLocaleString()} 字节</strong></div>
        <div className="dialog-actions"><button onClick={() => setDialog("trash_item")}>返回</button><button className="danger" disabled={safeMode || selectedTrashItem.restore.originResolution === "reconciliation_required"} onClick={() => void previewTrashPermanentDelete()}>{safeMode ? "安全模式：操作已暂停" : "预览永久删除证据"}</button></div>
      </section></div>}

      {dialog === "core" && <div className="modal-backdrop" role="presentation"><section className="dialog-card compact" role="dialog" aria-modal="true" aria-labelledby="core-title">
        <span className="eyebrow">LOCAL CORE</span><h2 id="core-title">{liveDocument ? "已打开本机工作区" : desktopMode ? "打开工作区" : "连接真实文档"}</h2>
        <label className="setting-row"><span>界面主题</span><select aria-label="界面主题" value={theme} onChange={(event) => setTheme(event.target.value as ThemeMode)}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option><option value="contrast">高对比度</option></select></label>
        {liveDocument && liveWorkspace ? <><p>{desktopMode ? "桌面端只访问你通过系统选择器授权的工作区。" : "浏览器没有目录权限；当前工作区由本机 Rust Core 授权。"} 文档和结构写入都经过版本与事务检查。</p><div className="core-connection-note"><span>状态</span><strong>{liveWorkspace.nodes.length} 个节点 · 工作区版本 {liveWorkspace.revision.slice(0, 12)}</strong></div><label className="setting-row"><span>紧邻标题 + 正文</span><button disabled={safeMode} onClick={() => void previewWorkspaceAction({ action: "presentation", value: liveWorkspace.presentation.adjacentHeadingBody === "run_in" ? "separate" : "run_in" })}>{liveWorkspace.presentation.adjacentHeadingBody === "run_in" ? "混排（开启）" : "分开（默认）"}</button></label>{desktopMode && <div className="desktop-diagnostics"><div><span>Safe Mode</span><strong>{safeMode ? "已启用：只读工作区 + 设备草稿" : "未启用：Core 提交可用"}</strong></div><div className="diagnostic-actions"><button onClick={() => void refreshDiagnostics()}>刷新诊断</button><button className={safeMode ? "safe-active" : ""} onClick={() => void toggleSafeMode()}>{safeMode ? "退出安全模式" : "进入安全模式"}</button></div>{diagnostics && <ul><li>工作区：{diagnostics.workspaceValid ? "有效" : "需要处理"} · {diagnostics.nodeCount} 个节点</li><li>恢复：{diagnostics.recoveryDraftCount} 份草稿 · {diagnostics.recoveryIssueCount} 个问题</li><li>索引：{diagnostics.index}</li><li>诊断已隐藏绝对路径和文档正文</li></ul>}</div>}</> : desktopMode ? <><p>{coreState === "connecting" ? "正在恢复上次的工作区…" : restoreError || "选择一个有效的 X/X.adoc Weftext 工作区。打开后，文缕会记住它并在下次启动时恢复。"}</p><div className="core-connection-note"><span>安全边界</span><strong>系统文件夹选择器 · Rust Core · 无任意文件访问</strong></div></> : <><p>{coreState === "connecting" ? "正在连接命令行中选择的工作区…" : coreState === "error" ? coreError : "在命令行启动工作区原型连接，然后打开它输出的完整链接。访问令牌只留在浏览器地址片段中。"}</p><div className="core-connection-note"><span>安全边界</span><strong>仅本机 · 工作区事务 · 必须预览 · 版本检查</strong></div></>}
        {derivedIndexWarning && liveDocument && <div className="core-connection-note" role="alert"><span>搜索降级</span><strong>工作区可继续使用；派生索引需要重建</strong></div>}
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>关闭</button>{desktopMode && liveWorkspace && <button onClick={() => setDialog("backup")}>备份与恢复</button>}{desktopMode && <button className="primary desktop-open-button" onClick={() => void chooseDesktopWorkspace()}>{liveDocument ? "切换工作区" : "选择文件夹"}</button>}</div>
      </section></div>}

      {dialog === "backup" && desktopMode && liveWorkspace && <div className="modal-backdrop intake-modal-backdrop" role="presentation"><BackupSurface
        enabled={typeof window.weftextDesktop?.chooseBackupDirectory === "function"}
        safeMode={safeMode}
        blockedReason={backupBlockedReason}
        sourceNodeId={selected}
        destinationParentId={liveWorkspace.rootNodeId}
        request={sharedCoreRequest}
        chooseDirectory={async (kind) => window.weftextDesktop?.chooseBackupDirectory?.(kind) ?? null}
        onWorkspaceChanged={async () => { await openNode(selected, { refresh: true }); }}
        onClose={() => setDialog(null)}
      /></div>}

      {dialog === "structure" && structuralPlan && <div className="modal-backdrop" role="presentation"><section className="dialog-card" role="dialog" aria-modal="true" aria-labelledby="structure-title">
        <span className="eyebrow">操作确认</span><h2 id="structure-title">{structuralPreviewLabel(structuralContext)}预览</h2><p>这是 Core 固定的确切计划。确认时会检查同一个工作区版本、目标 UUID 和草稿范围，不会重新生成另一套身份。</p>
        {structuralContext?.kind === "node_metadata" && <p className="metadata-plan-summary"><strong>节点元数据：</strong>{structuralContext.summary}</p>}
        {structuralPlan.scopeSummary && <section className="scope-summary" aria-label="完整操作范围">
          <header><strong dir="auto">{structuralPlan.scopeSummary.rootNode.displayName}</strong><code>{structuralPlan.scopeSummary.rootNode.nodeId}</code><span>{structuralPlan.scopeSummary.identityPolicy === "rekey" ? "副本重新分配永久身份" : "保留永久身份"}</span></header>
          <div className="scope-summary-grid"><span>后代节点<strong>{structuralPlan.scopeSummary.descendantNodeCount}</strong></span><span>节点资源<strong>{structuralPlan.scopeSummary.resourceCount}</strong></span><span>批注文件<strong>{structuralPlan.scopeSummary.annotationSidecarCount}</strong></span><span>总字节<strong>{structuralPlan.scopeSummary.byteTotal.toLocaleString()}</strong></span><span>受影响文档<strong>{structuralPlan.scopeSummary.affectedDocumentNodeIds.length}</strong></span><span>链接或文档改写<strong>{structuralPlan.scopeSummary.rewrittenDocumentNodeIds.length}</strong></span><span>废纸篓条目<strong>{structuralPlan.scopeSummary.trashItemCount}</strong></span></div>
          {structuralPlan.scopeSummary.operationId && <p>批量操作编号 <code>{structuralPlan.scopeSummary.operationId}</code></p>}
        </section>}
        {!structuralPlan.scopeSummary && Boolean(structuralPlan.trashItemChanges?.length) && <div className="core-connection-note"><span>废纸篓条目</span><strong>{structuralPlan.trashItemChanges?.length ?? 0} 个独立可恢复条目</strong></div>}
        {structuralPlan.identityMap.length > 0 && <details className="identity-map"><summary>副本永久身份映射（{structuralPlan.identityMap.length}）</summary>{structuralPlan.identityMap.map((entry) => <div key={entry.sourceNodeId}><code>{entry.sourceNodeId}</code><i>→</i><code>{entry.destinationNodeId}</code></div>)}</details>}
        <div className="transaction-list">{structuralPlan.trashItemChanges?.length ? structuralPlan.trashItemChanges.map((change) => <div key={`${change.disposition}-${change.manifest.trashItemId}`}><span>{trashDispositionLabel(change.disposition)}</span><i>→</i><strong dir="auto">{change.manifest.originalName}</strong><small>{change.manifest.trashItemId} · {trashPayloadSha256(change.manifest)} · {trashPayloadByteLength(change.manifest).toLocaleString()} 字节{change.destinationName ? ` · 目标 ${change.destinationName}` : ""}</small></div>) : structuralPlan.pathChanges.length ? structuralPlan.pathChanges.map((change) => <div key={`${change.nodeId}-${change.newPath}`}><span>{change.oldPath ?? "新节点"}</span><i>→</i><strong>{change.newPath}</strong></div>) : <div><span>工作区设置</span><i>→</i><strong>{structuralPlan.documentChanges[0]?.path ?? "无普通文档路径变化"}</strong></div>}</div>
        <ul className="impact-list"><li><span>✓</span>基准工作区版本 {structuralPlan.baseRevision.slice(0, 12)}</li><li><span>✓</span>{structuralPlan.documentChanges.length} 个文档需要精确改写</li><li><span>✓</span>{structuralPlan.generatedNodeIds.length} 个新节点身份</li><li><span>✓</span>提交前复核 {structuralPlan.draftSensitiveNodeIds.length} 个草稿敏感节点</li></ul>
        {structuralContext?.kind === "trash" && structuralContext.purpose === "permanent_delete" && <label className="permanent-delete-confirmation"><input type="checkbox" checked={permanentDeleteConfirmed} onChange={(event) => setPermanentDeleteConfirmed(event.target.checked)} />我确认永久删除上列精确条目 ID、摘要和字节；废纸篓不是备份，此操作不可恢复。</label>}
        <div className="dialog-actions"><button onClick={() => { setStructuralPlan(null); setStructuralContext(null); setStructuralDraftScope(null); setPermanentDeleteConfirmed(false); setDialog(null); }}>取消</button><button className="primary" disabled={safeMode || (structuralContext?.kind === "trash" && structuralContext.purpose === "permanent_delete" && !permanentDeleteConfirmed)} onClick={() => void commitWorkspaceAction()}>{safeMode ? "安全模式：提交已暂停" : structuralContext?.kind === "trash" && structuralContext.purpose === "permanent_delete" ? "确认永久删除" : "确认提交事务"}</button></div>
      </section></div>}

      {dialog === "save" && savePlan && <div className="modal-backdrop" role="presentation"><section className="dialog-card" role="dialog" aria-modal="true" aria-labelledby="save-title">
        <span className="eyebrow">CORE SAVE PREVIEW</span><h2 id="save-title">{saveTarget === "split" ? "保存第二栏文档预览" : "保存文档预览"}</h2><p>Core 已对当前 revision 生成确定性计划。确认后仍会在原子替换前再次检查 revision 与节点身份。</p>
        <div className="move-preview"><div><span>当前</span><code>{savePlan.baseRevision.slice(0, 12)}</code><small>{savePlan.oldLength} 字节</small></div><i>→</i><div><span>提交后</span><code>{savePlan.nextRevision.slice(0, 12)}</code><small>{savePlan.newLength} 字节</small></div></div>
        <ul className="impact-list"><li><span>✓</span>节点 UUID 必须保持不变</li><li><span>✓</span>精确 UTF-8 源码，不重排 frontmatter</li><li><span>✓</span>陈旧 revision 将进入冲突状态</li></ul>
        <div className="dialog-actions"><button onClick={() => setDialog(null)}>返回编辑</button><button className="primary" disabled={!savePlan.changed || safeMode} onClick={() => void commitSave()}>{safeMode ? "安全模式：提交已暂停" : savePlan.changed ? "确认提交" : "没有变化"}</button></div>
      </section></div>}

      {toast && <div className="toast"><span>✓</span>{toast}</div>}
    </main>
  );
}

export default function Home() {
  return <WeftextApp demo={DEMO_WORKSPACE} />;
}
