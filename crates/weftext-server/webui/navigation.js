export const INITIAL_NAVIGATION_WINDOW = 240;

const storagePrefix = "weftext.server.explorer.v1";

export function defaultNavigationState() {
  return {
    version: 1,
    activity: "explorer",
    mode: "hierarchy",
    collapsedNodeIds: [],
    filter: "",
    width: 288,
    scrollTop: 0,
    browseLocator: null,
    hierarchyLimit: INITIAL_NAVIGATION_WINDOW,
    contentsLimit: INITIAL_NAVIGATION_WINDOW,
    metrics: [],
  };
}

export function readNavigationState(workspaceScope) {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(`${storagePrefix}.${workspaceScope}`) ?? "null");
    if (!parsed || parsed.version !== 1) return defaultNavigationState();
    return {
      ...defaultNavigationState(),
      activity: ["explorer", "search", "chrono", "trash"].includes(parsed.activity) ? parsed.activity : "explorer",
      mode: parsed.mode === "contents" ? "contents" : "hierarchy",
      collapsedNodeIds: Array.isArray(parsed.collapsedNodeIds) ? parsed.collapsedNodeIds.filter((id) => typeof id === "string") : [],
      filter: typeof parsed.filter === "string" ? parsed.filter : "",
      width: Math.max(220, Math.min(480, Number(parsed.width) || 288)),
      scrollTop: Math.max(0, Number(parsed.scrollTop) || 0),
      browseLocator: typeof parsed.browseLocator === "string" ? parsed.browseLocator : null,
    };
  } catch {
    return defaultNavigationState();
  }
}

export function writeNavigationState(workspaceScope, state) {
  try {
    window.localStorage.setItem(`${storagePrefix}.${workspaceScope}`, JSON.stringify({
      version: 1,
      activity: state.activity,
      mode: state.mode,
      collapsedNodeIds: [...state.collapsedNodeIds],
      filter: state.filter,
      width: state.width,
      scrollTop: state.scrollTop,
      browseLocator: state.browseLocator,
    }));
  } catch {
    // Device presentation state is best effort and never Server authority.
  }
}

export function workspaceNavigation(workspace) {
  if (workspace?.navigation?.version === 1) return workspace.navigation;
  const children = new Map();
  for (const node of workspace?.nodes ?? []) if (node.parentId) children.set(node.parentId, (children.get(node.parentId) ?? 0) + 1);
  return {
    version: 1,
    rootNodeId: workspace.rootNodeId,
    hierarchy: (workspace?.nodes ?? []).map((node) => ({
      nodeId: node.id,
      name: node.name,
      parentNodeId: node.parentId,
      locator: node.locator,
      depth: node.locator ? node.locator.split("/").length : 0,
      childCount: children.get(node.id) ?? 0,
      displayIcon: node.displayIcon,
    })),
    contents: (workspace?.content ?? []).map((item) => ({ ...item })),
  };
}

export function visibleHierarchy(projection, collapsedNodeIds, filter = "") {
  const collapsed = new Set(collapsedNodeIds);
  const hiddenDepths = [];
  const needle = filter.trim().toLocaleLowerCase();
  const matches = needle ? new Set(projection.hierarchy.filter((node) => node.name.toLocaleLowerCase().includes(needle)).map((node) => node.nodeId)) : null;
  if (matches) {
    for (const node of projection.hierarchy) {
      if (!matches.has(node.nodeId)) continue;
      let parent = node.parentNodeId;
      while (parent) {
        matches.add(parent);
        parent = projection.hierarchy.find((candidate) => candidate.nodeId === parent)?.parentNodeId ?? null;
      }
    }
  }
  return projection.hierarchy.filter((node) => {
    while (hiddenDepths.length && hiddenDepths.at(-1) >= node.depth) hiddenDepths.pop();
    const hidden = hiddenDepths.length > 0;
    if (collapsed.has(node.nodeId)) hiddenDepths.push(node.depth);
    return !hidden && (!matches || matches.has(node.nodeId));
  });
}

export function directContents(projection, locator, filter = "") {
  const needle = filter.trim().toLocaleLowerCase();
  return projection.contents.filter((item) => item.parentLocator === locator && (!needle || item.name.toLocaleLowerCase().includes(needle)));
}

export function validateBrowseLocator(projection, locator) {
  if (locator === null) return null;
  return projection.contents.some((item) => item.kind === "unmanaged_directory" && item.locator === locator) ? locator : null;
}

export function locationBreadcrumbs(projection, locator) {
  const segments = locator ? locator.split("/") : [];
  return ["", ...segments.map((_, index) => segments.slice(0, index + 1).join("/"))].map((value) => {
    const node = projection.hierarchy.find((item) => item.locator === value);
    const directory = projection.contents.find((item) => item.kind === "unmanaged_directory" && item.locator === value);
    return { locator: value, name: node?.name ?? directory?.name ?? value.split("/").at(-1) ?? "工作区", nodeId: node?.nodeId ?? null, unmanaged: Boolean(directory) };
  });
}

export function incrementalItems(items, limit) {
  const safeLimit = Math.max(INITIAL_NAVIGATION_WINDOW, limit);
  return { items: items.slice(0, safeLimit), remaining: Math.max(0, items.length - safeLimit) };
}

export function measureInteraction(operation, startedAt, renderedItems, totalItems) {
  return { operation, durationMs: Math.max(0, performance.now() - startedAt), renderedItems, totalItems, excludesCoreScan: true };
}
