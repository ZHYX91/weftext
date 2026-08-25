export type WorkspaceActionKind =
  | "create"
  | "rename"
  | "move"
  | "copy"
  | "trash_node"
  | "chrono"
  | "restore_item"
  | "trash_resource"
  | "permanently_delete_item";

export type WorkspaceActionField =
  | "name"
  | "parentNodeId"
  | "date"
  | "period"
  | "restoreMode"
  | "resourceNames";

export type NodeRowSurface = "hierarchy" | "contents" | "search" | "chrono";

export type WorkspaceActionInvocation =
  | Readonly<{ source: "editor_command"; focusedNodeId: string }>
  | Readonly<{ source: "explicit_node_row"; surface: NodeRowSurface; nodeId: string }>
  | Readonly<{ source: "trash_item_row"; trashItemId: string }>
  | Readonly<{ source: "resource_row"; ownerNodeId: string; resourceName: string }>;

type NodeTargetAction = "create" | "rename" | "move" | "copy" | "trash_node" | "chrono" | "trash_resource";

export type FrozenWorkspaceActionTarget =
  | Readonly<{
      kind: "node";
      action: NodeTargetAction;
      nodeId: string;
      source: "editor_command" | "explicit_node_row";
      surface: NodeRowSurface | null;
    }>
  | Readonly<{
      kind: "trash_item";
      action: "restore_item" | "permanently_delete_item";
      trashItemId: string;
      source: "trash_item_row";
    }>
  | Readonly<{
      kind: "resource";
      action: "trash_resource";
      ownerNodeId: string;
      resourceName: string;
      source: "resource_row";
    }>;

export type WorkspaceActionDefinition = Readonly<{
  action: WorkspaceActionKind;
  label: string;
  targetLabel: string;
  fields: readonly WorkspaceActionField[];
}>;

export const WORKSPACE_ACTION_REGISTRY: Readonly<Record<WorkspaceActionKind, WorkspaceActionDefinition>> = Object.freeze({
  create: Object.freeze({ action: "create", label: "新建子节点", targetLabel: "父节点", fields: Object.freeze(["name"] as const) }),
  rename: Object.freeze({ action: "rename", label: "重命名当前节点", targetLabel: "节点", fields: Object.freeze(["name"] as const) }),
  move: Object.freeze({ action: "move", label: "移动整个节点分支", targetLabel: "分支根节点", fields: Object.freeze(["parentNodeId"] as const) }),
  copy: Object.freeze({ action: "copy", label: "复制整个节点分支", targetLabel: "分支根节点", fields: Object.freeze(["parentNodeId", "name"] as const) }),
  trash_node: Object.freeze({ action: "trash_node", label: "将整个节点分支移入废纸篓", targetLabel: "分支根节点", fields: Object.freeze([] as const) }),
  chrono: Object.freeze({ action: "chrono", label: "创建时间节点", targetLabel: "时间节点根", fields: Object.freeze(["date", "period"] as const) }),
  restore_item: Object.freeze({ action: "restore_item", label: "恢复废纸篓条目", targetLabel: "废纸篓条目", fields: Object.freeze(["restoreMode"] as const) }),
  trash_resource: Object.freeze({ action: "trash_resource", label: "将节点资源移入废纸篓", targetLabel: "资源所属节点", fields: Object.freeze(["resourceNames"] as const) }),
  permanently_delete_item: Object.freeze({ action: "permanently_delete_item", label: "永久删除废纸篓条目", targetLabel: "废纸篓条目", fields: Object.freeze([] as const) }),
});

function requireId(value: string, label: string) {
  const normalized = value.trim();
  if (!normalized) throw new Error(`${label}缺少明确 ID`);
  return normalized;
}

/** Resolves and freezes the complete action target at invocation time. */
export function resolveWorkspaceActionTarget(
  action: WorkspaceActionKind,
  invocation: WorkspaceActionInvocation,
): FrozenWorkspaceActionTarget {
  if (action === "restore_item" || action === "permanently_delete_item") {
    if (invocation.source !== "trash_item_row") throw new Error("废纸篓操作必须来自明确条目");
    return Object.freeze({ kind: "trash_item", action, trashItemId: requireId(invocation.trashItemId, "废纸篓条目"), source: invocation.source });
  }
  if (action === "trash_resource" && invocation.source === "resource_row") {
    return Object.freeze({
      kind: "resource",
      action,
      ownerNodeId: requireId(invocation.ownerNodeId, "资源所属节点"),
      resourceName: requireId(invocation.resourceName, "资源"),
      source: invocation.source,
    });
  }
  if (invocation.source !== "editor_command" && invocation.source !== "explicit_node_row") {
    throw new Error("节点操作必须来自当前编辑栏或明确节点行");
  }
  const nodeId = invocation.source === "editor_command"
    ? requireId(invocation.focusedNodeId, "当前编辑栏")
    : requireId(invocation.nodeId, "操作行");
  return Object.freeze({
    kind: "node",
    action,
    nodeId,
    source: invocation.source,
    surface: invocation.source === "explicit_node_row" ? invocation.surface : null,
  });
}

/** Caller passes Core's exact draft-sensitive set; this module never derives action scope. */
export type CoreReviewedDraftScope = Readonly<{ draftSensitiveNodeIds: readonly string[] }>;
export type FrozenReviewedDraftScope = Readonly<{ draftSensitiveNodeIds: readonly string[] }>;

export function freezeCoreReviewedDraftScope(scope: CoreReviewedDraftScope): FrozenReviewedDraftScope {
  const normalized = scope.draftSensitiveNodeIds.map((id) => requireId(id, "Core draft-sensitive scope"));
  if (new Set(normalized).size !== normalized.length) throw new Error("Core draft-sensitive scope contains duplicate node IDs");
  normalized.sort();
  return Object.freeze({ draftSensitiveNodeIds: Object.freeze(normalized) });
}

export function conflictingDirtyNodeIds(scope: FrozenReviewedDraftScope, dirtyNodeIds: ReadonlySet<string>) {
  return Object.freeze(scope.draftSensitiveNodeIds.filter((nodeId) => dirtyNodeIds.has(nodeId)));
}
