export type ExplorerActivity = "explorer" | "search" | "chrono";
export type ExplorerMode = "hierarchy" | "contents";

export type NavigationIcon = {
  kind: "explicit_node" | "default_node" | "folder" | "markdown_file" | "file" | "workspace_root" | "trash";
  explicit?: { kind: "emoji" | "built_in"; value: string; glyph: string };
};

export type NavigationNode = {
  nodeId: string;
  name: string;
  parentNodeId: string | null;
  locator: string;
  depth: number;
  childCount: number;
  displayIcon: NavigationIcon;
};

export type NavigationContentItem = {
  kind: "managed_node" | "unmanaged_directory" | "unmanaged_markdown" | "resource";
  name: string;
  locator: string;
  parentLocator: string | null;
  nodeId: string | null;
  ownerNodeId: string | null;
  displayIcon: NavigationIcon;
};

export type SharedNavigationProjection = {
  version: 1;
  rootNodeId: string;
  hierarchy: NavigationNode[];
  contents: NavigationContentItem[];
};

export type ExplorerDeviceState = {
  version: 1;
  activity: ExplorerActivity;
  mode: ExplorerMode;
  collapsedNodeIds: string[];
  filter: string;
  width: number;
  scrollTop: number;
  browseLocator: string | null;
};

export type NavigationPerformanceSample = {
  operation: "initial_render" | "expand" | "mode_switch" | "incremental_refresh" | "keyboard_move";
  durationMs: number;
  renderedItems: number;
  totalItems: number;
};

const storageKey = "weftext.explorer.v1";
export const INITIAL_NAVIGATION_WINDOW = 240;

export const defaultExplorerState: ExplorerDeviceState = {
  version: 1,
  activity: "explorer",
  mode: "hierarchy",
  collapsedNodeIds: [],
  filter: "",
  width: 284,
  scrollTop: 0,
  browseLocator: null,
};

export function readExplorerState(workspaceId: string): ExplorerDeviceState {
  try {
    const records = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, Partial<ExplorerDeviceState>>;
    const record = records[workspaceId];
    if (!record || record.version !== 1) return defaultExplorerState;
    return {
      version: 1,
      activity: record.activity === "search" || record.activity === "chrono" ? record.activity : "explorer",
      mode: record.mode === "contents" ? "contents" : "hierarchy",
      collapsedNodeIds: Array.isArray(record.collapsedNodeIds) ? record.collapsedNodeIds.filter((id): id is string => typeof id === "string") : [],
      filter: typeof record.filter === "string" ? record.filter : "",
      width: Math.max(220, Math.min(480, Number(record.width) || defaultExplorerState.width)),
      scrollTop: Math.max(0, Number(record.scrollTop) || 0),
      browseLocator: typeof record.browseLocator === "string" ? record.browseLocator : null,
    };
  } catch {
    return defaultExplorerState;
  }
}

export function writeExplorerState(workspaceId: string, state: ExplorerDeviceState) {
  try {
    const records = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, ExplorerDeviceState>;
    window.localStorage.setItem(storageKey, JSON.stringify({ ...records, [workspaceId]: state }));
  } catch {
    // Explorer layout is best-effort device state, never workspace authority.
  }
}

type LegacyNode = {
  id: string;
  name: string;
  parentId?: string | null;
  path?: string;
  depth?: number;
  displayIcon?: NavigationIcon | null;
  icon?: NavigationIcon["explicit"] | null;
};
type LegacyContent = {
  kind: NavigationContentItem["kind"];
  name: string;
  path: string;
  parentPath: string | null;
  nodeId: string | null;
  ownerNodeId: string | null;
  displayIcon: NavigationIcon;
};

export function navigationProjection(
  rootNodeId: string,
  projection: SharedNavigationProjection | undefined,
  nodes: LegacyNode[],
  content: LegacyContent[] = [],
): SharedNavigationProjection {
  if (projection?.version === 1 && projection.rootNodeId === rootNodeId) return projection;
  const childCounts = new Map<string, number>();
  for (const node of nodes) if (node.parentId) childCounts.set(node.parentId, (childCounts.get(node.parentId) ?? 0) + 1);
  return {
    version: 1,
    rootNodeId,
    hierarchy: nodes.map((node) => ({
      nodeId: node.id,
      name: node.name,
      parentNodeId: node.parentId ?? null,
      locator: node.path ?? "",
      depth: node.path ? node.path.split("/").length : node.depth ?? 0,
      childCount: childCounts.get(node.id) ?? 0,
      displayIcon: node.displayIcon ?? (node.icon ? { kind: "explicit_node", explicit: node.icon } : { kind: node.id === rootNodeId ? "workspace_root" : "default_node" }),
    })),
    contents: content.map((item) => ({
      kind: item.kind,
      name: item.name,
      locator: item.path,
      parentLocator: item.parentPath,
      nodeId: item.nodeId,
      ownerNodeId: item.ownerNodeId,
      displayIcon: item.displayIcon,
    })),
  };
}

export function visibleHierarchy(
  projection: SharedNavigationProjection,
  collapsedNodeIds: ReadonlySet<string>,
  filter: string,
) {
  const hiddenDepths: number[] = [];
  const needle = filter.trim().toLocaleLowerCase();
  const matched = needle
    ? new Set(projection.hierarchy.filter((node) => node.name.toLocaleLowerCase().includes(needle)).map((node) => node.nodeId))
    : null;
  if (matched) {
    for (const node of projection.hierarchy) {
      if (!matched.has(node.nodeId)) continue;
      let parent = node.parentNodeId;
      while (parent) {
        matched.add(parent);
        parent = projection.hierarchy.find((candidate) => candidate.nodeId === parent)?.parentNodeId ?? null;
      }
    }
  }
  return projection.hierarchy.filter((node) => {
    while (hiddenDepths.length && hiddenDepths.at(-1)! >= node.depth) hiddenDepths.pop();
    const hidden = hiddenDepths.length > 0;
    if (collapsedNodeIds.has(node.nodeId)) hiddenDepths.push(node.depth);
    return !hidden && (!matched || matched.has(node.nodeId));
  });
}

export function directContents(projection: SharedNavigationProjection, locator: string, filter = "") {
  const needle = filter.trim().toLocaleLowerCase();
  return projection.contents.filter((item) => item.parentLocator === locator && (!needle || item.name.toLocaleLowerCase().includes(needle)));
}

export function validatedBrowseLocator(projection: SharedNavigationProjection, locator: string | null) {
  if (locator === null) return null;
  return projection.contents.some((item) => item.kind === "unmanaged_directory" && item.locator === locator)
    ? locator
    : null;
}

export function locationBreadcrumbs(projection: SharedNavigationProjection, locator: string) {
  const segments = locator ? locator.split("/") : [];
  const locators = ["", ...segments.map((_, index) => segments.slice(0, index + 1).join("/"))];
  return locators.map((value) => {
    const node = projection.hierarchy.find((item) => item.locator === value);
    const directory = projection.contents.find((item) => item.kind === "unmanaged_directory" && item.locator === value);
    return {
      locator: value,
      name: node?.name ?? directory?.name ?? value.split("/").at(-1) ?? "工作区",
      nodeId: node?.nodeId ?? null,
      unmanaged: Boolean(directory),
    };
  });
}

export function incrementalWindow<T>(items: readonly T[], limit: number) {
  const safeLimit = Math.max(INITIAL_NAVIGATION_WINDOW, limit);
  return { items: items.slice(0, safeLimit), remaining: Math.max(0, items.length - safeLimit) };
}

export function interactionMeasurement(
  operation: NavigationPerformanceSample["operation"],
  startedAt: number,
  renderedItems: number,
  totalItems: number,
): NavigationPerformanceSample {
  return {
    operation,
    durationMs: Math.max(0, performance.now() - startedAt),
    renderedItems,
    totalItems,
  };
}
