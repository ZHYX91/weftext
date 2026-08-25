import {
  ApiFailure,
  COLLABORATION_WIRE_VERSION,
  annotationTargetFromSelection,
  captureMutationContext,
  captureSurfaceContext,
  createDeviceDraft,
  createCollaborationClientId,
  createLatestRequestGate,
  deviceDraftStorageKey,
  isDeviceDraftForWorkspace,
  mutationContextMatches,
  previewLines,
  productSurfaceAccess,
  requireAnnotationReadPayload,
  requireCanonicalDocumentPayload,
  requireCanonicalWorkspacePayload,
  requireQueryExecutionPayload,
  requireRoleCapabilityMap,
  requireSessionCapabilities,
  requireTaskInspectionPayload,
  requireTaskPreviewPayload,
  safeguardOutgoingDraft,
  serverApi,
  surfaceContextMatches,
  taskPreviewConfirmation,
  taskTargetForOccurrence,
  trashPermanentDeleteConfirmation,
} from "./api.js";
import {
  INITIAL_NAVIGATION_WINDOW,
  defaultNavigationState,
  directContents,
  incrementalItems,
  locationBreadcrumbs,
  measureInteraction,
  readNavigationState,
  validateBrowseLocator,
  visibleHierarchy,
  workspaceNavigation,
  writeNavigationState,
} from "./navigation.js";

const state = {
  workspace: null,
  document: null,
  preview: null,
  session: null,
  serverCapabilities: null,
  inspectorTab: "properties",
  annotations: null,
  tasks: null,
  taskPlan: null,
  taskPlanConfirmed: false,
  taskPlanPending: false,
  backupPlan: null,
  restorePlan: null,
  restorePurpose: null,
  trashInventory: null,
  selectedTrashItemId: null,
  trashPlan: null,
  trashPlanPurpose: null,
  query: null,
  collaboration: null,
  collaborationNotice: null,
  collaborationClientId: null,
  collaborationUnsubscribe: null,
  unsubscribe: null,
  checkingSubscriptionSession: false,
  navigation: defaultNavigationState(),
};
const elements = {};
const navigationGate = createLatestRequestGate();
const mutationGate = createLatestRequestGate();
const annotationGate = createLatestRequestGate();
const taskGate = createLatestRequestGate();
const queryGate = createLatestRequestGate();
const trashGate = createLatestRequestGate();

function currentSurfaceAccess() {
  return state.session ? productSurfaceAccess(state.session) : {
    readAnnotations: false,
    writeAnnotations: false,
    readTasks: false,
    writeTasks: false,
    executeQueries: false,
    editDocuments: false,
    mutateStructure: false,
    permanentlyDelete: false,
    manageMembers: false,
    manageWorkspace: false,
  };
}

function hasDirtyDocument() {
  return Boolean(state.document && elements.editor.value !== state.document.source);
}

function currentWorkspaceScope() {
  return state.workspace?.workspaceScope ?? null;
}

function currentActorScope() {
  return state.session?.actorScope ?? null;
}

function nodeIcon(icon) {
  return icon?.glyph ? `<span class="node-icon" aria-hidden="true">${escapeHtml(icon.glyph)}</span>` : "";
}

function workspaceItemIcon(icon) {
  if (icon?.kind === "explicit_node") return nodeIcon(icon.explicit);
  const glyph = {
    default_node: "文",
    folder: "夹",
    markdown_file: "M",
    file: "档",
    workspace_root: "根",
    trash: "废",
  }[icon?.kind];
  return glyph ? `<span class="node-icon" aria-hidden="true">${glyph}</span>` : "";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function setStatus(message, tone = "normal") {
  elements.status.textContent = message;
  elements.status.dataset.tone = tone;
}

function currentProjection() {
  return state.workspace ? workspaceNavigation(state.workspace) : null;
}

function currentNavigationNode() {
  return currentProjection()?.hierarchy.find((node) => node.nodeId === state.document?.nodeId) ?? null;
}

function currentContentsLocator() {
  return state.navigation.browseLocator ?? currentNavigationNode()?.locator ?? "";
}

function persistNavigation() {
  if (currentWorkspaceScope()) writeNavigationState(currentWorkspaceScope(), state.navigation);
}

function recordNavigationMetric(operation, startedAt, renderedItems, totalItems) {
  state.navigation.metrics = [
    ...state.navigation.metrics.slice(-24),
    measureInteraction(operation, startedAt, renderedItems, totalItems),
  ];
  renderNavigationMetrics();
}

function renderNavigationMetrics() {
  elements.navigationMetrics.replaceChildren();
  for (const metric of state.navigation.metrics.slice(-5)) {
    const row = document.createElement("div");
    row.innerHTML = `<span>${escapeHtml(metric.operation)}</span><strong>${metric.durationMs.toFixed(2)} ms</strong><small>${metric.renderedItems}/${metric.totalItems} UI 项；不含 Core 扫描</small>`;
    elements.navigationMetrics.append(row);
  }
}

function chooseActivity(activity) {
  const startedAt = performance.now();
  state.navigation.activity = activity;
  renderNavigation();
  persistNavigation();
  if (activity === "search") elements.searchInput.focus();
  if (activity === "trash") void loadTrashInventory();
  requestAnimationFrame(() => recordNavigationMetric("mode_switch", startedAt, 1, 1));
}

function chooseExplorerMode(mode) {
  const startedAt = performance.now();
  state.navigation.activity = "explorer";
  state.navigation.mode = mode;
  state.navigation.hierarchyLimit = INITIAL_NAVIGATION_WINDOW;
  state.navigation.contentsLimit = INITIAL_NAVIGATION_WINDOW;
  renderNavigation();
  persistNavigation();
  const projection = currentProjection();
  const total = projection ? (mode === "hierarchy" ? visibleHierarchy(projection, state.navigation.collapsedNodeIds, state.navigation.filter).length : directContents(projection, currentContentsLocator(), state.navigation.filter).length) : 0;
  requestAnimationFrame(() => recordNavigationMetric("mode_switch", startedAt, Math.min(total, INITIAL_NAVIGATION_WINDOW), total));
}

function toggleHierarchyNode(nodeId) {
  const startedAt = performance.now();
  const collapsed = new Set(state.navigation.collapsedNodeIds);
  if (collapsed.has(nodeId)) collapsed.delete(nodeId); else collapsed.add(nodeId);
  state.navigation.collapsedNodeIds = [...collapsed];
  renderHierarchy();
  persistNavigation();
  const total = currentProjection() ? visibleHierarchy(currentProjection(), collapsed, state.navigation.filter).length : 0;
  requestAnimationFrame(() => recordNavigationMetric("expand", startedAt, Math.min(total, state.navigation.hierarchyLimit), total));
}

function focusHierarchyNode(nodeId) {
  [...elements.hierarchyView.querySelectorAll("[data-hierarchy-node]")]
    .find((element) => element.dataset.hierarchyNode === nodeId)?.focus();
}

function handleHierarchyKey(event, rows, index) {
  const node = rows[index];
  if (!node) return;
  const startedAt = performance.now();
  let handled = true;
  if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
    let target = index;
    if (event.key === "ArrowDown") target = Math.min(rows.length - 1, index + 1);
    if (event.key === "ArrowUp") target = Math.max(0, index - 1);
    if (event.key === "Home") target = 0;
    if (event.key === "End") target = rows.length - 1;
    if (target >= state.navigation.hierarchyLimit) {
      state.navigation.hierarchyLimit = target + INITIAL_NAVIGATION_WINDOW;
      renderHierarchy();
    }
    requestAnimationFrame(() => focusHierarchyNode(rows[target].nodeId));
  } else if (event.key === "ArrowRight" && node.childCount) {
    if (state.navigation.collapsedNodeIds.includes(node.nodeId)) toggleHierarchyNode(node.nodeId);
    else if (rows[index + 1]?.parentNodeId === node.nodeId) focusHierarchyNode(rows[index + 1].nodeId);
  } else if (event.key === "ArrowLeft") {
    if (node.childCount && !state.navigation.collapsedNodeIds.includes(node.nodeId)) toggleHierarchyNode(node.nodeId);
    else if (node.parentNodeId) focusHierarchyNode(node.parentNodeId);
  } else if (event.key === "Enter") {
    void openNode(node.nodeId, "hierarchy");
  } else {
    handled = false;
  }
  if (handled) {
    event.preventDefault();
    requestAnimationFrame(() => recordNavigationMetric("keyboard_move", startedAt, Math.min(rows.length, state.navigation.hierarchyLimit), rows.length));
  }
}

function renderHierarchy() {
  const projection = currentProjection();
  elements.hierarchyView.replaceChildren();
  if (!projection) return;
  const rows = visibleHierarchy(projection, state.navigation.collapsedNodeIds, state.navigation.filter);
  const rendered = incrementalItems(rows, state.navigation.hierarchyLimit);
  for (const [index, node] of rendered.items.entries()) {
    const row = document.createElement("div");
    row.className = `hierarchy-row${node.nodeId === state.document?.nodeId ? " active" : ""}`;
    row.setAttribute("role", "treeitem");
    row.setAttribute("aria-level", String(node.depth + 1));
    row.setAttribute("aria-selected", String(node.nodeId === state.document?.nodeId));
    row.style.paddingInlineStart = `${6 + node.depth * 15}px`;
    if (node.childCount) {
      const collapsed = state.navigation.collapsedNodeIds.includes(node.nodeId);
      row.setAttribute("aria-expanded", String(!collapsed));
      const disclosure = document.createElement("button");
      disclosure.className = "disclosure";
      disclosure.setAttribute("aria-label", `${collapsed ? "展开" : "折叠"} ${node.name}`);
      disclosure.innerHTML = `<span aria-hidden="true">${collapsed ? "›" : "⌄"}</span>`;
      disclosure.addEventListener("click", () => toggleHierarchyNode(node.nodeId));
      row.append(disclosure);
    } else {
      const placeholder = document.createElement("span");
      placeholder.className = "disclosure-placeholder";
      placeholder.setAttribute("aria-hidden", "true");
      row.append(placeholder);
    }
    const name = document.createElement("button");
    name.className = "hierarchy-name";
    name.dataset.hierarchyNode = node.nodeId;
    name.tabIndex = node.nodeId === state.document?.nodeId ? 0 : -1;
    name.setAttribute("aria-label", `打开节点 ${node.name}`);
    name.innerHTML = `${workspaceItemIcon(node.displayIcon)}<span dir="auto">${escapeHtml(node.name)}</span>`;
    name.addEventListener("click", () => void openNode(node.nodeId, "hierarchy"));
    name.addEventListener("keydown", (event) => handleHierarchyKey(event, rows, index));
    row.append(name);
    elements.hierarchyView.append(row);
  }
  if (!rows.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.setAttribute("role", "status");
    empty.textContent = "没有匹配的 managed nodes。";
    elements.hierarchyView.append(empty);
  }
  if (rendered.remaining) {
    const more = document.createElement("button");
    more.className = "load-more";
    more.textContent = `继续显示 ${Math.min(INITIAL_NAVIGATION_WINDOW, rendered.remaining)} 项`;
    more.addEventListener("click", () => {
      state.navigation.hierarchyLimit += INITIAL_NAVIGATION_WINDOW;
      renderHierarchy();
    });
    elements.hierarchyView.append(more);
  }
}

function browseContents(locator) {
  const startedAt = performance.now();
  state.navigation.browseLocator = locator;
  state.navigation.contentsLimit = INITIAL_NAVIGATION_WINDOW;
  renderContents();
  persistNavigation();
  const total = currentProjection() ? directContents(currentProjection(), locator, state.navigation.filter).length : 0;
  requestAnimationFrame(() => recordNavigationMetric("incremental_refresh", startedAt, Math.min(total, INITIAL_NAVIGATION_WINDOW), total));
}

function renderContents() {
  const projection = currentProjection();
  elements.contentList.replaceChildren();
  elements.contentsBreadcrumbs.replaceChildren();
  if (!projection) return;
  const locator = currentContentsLocator();
  const followed = currentNavigationNode();
  elements.contentsLocation.textContent = `正在浏览：${locator || "工作区根"}`;
  elements.contentsFollowing.textContent = `跟随当前编辑节点：${followed?.name ?? "未打开"}`;
  elements.returnToNode.hidden = state.navigation.browseLocator === null;
  elements.contentList.setAttribute("aria-label", `${locator || "工作区根"} 的直接子项`);
  for (const [index, crumb] of locationBreadcrumbs(projection, locator).entries()) {
    if (index) elements.contentsBreadcrumbs.append(document.createTextNode("/"));
    const button = document.createElement("button");
    button.textContent = crumb.name;
    if (crumb.locator === locator) button.setAttribute("aria-current", "location");
    button.addEventListener("click", () => {
      if (crumb.unmanaged) browseContents(crumb.locator);
      else if (crumb.nodeId) void openNode(crumb.nodeId, "breadcrumb");
      else browseContents("");
    });
    elements.contentsBreadcrumbs.append(button);
  }
  const rows = directContents(projection, locator, state.navigation.filter);
  const rendered = incrementalItems(rows, state.navigation.contentsLimit);
  const labels = { managed_node: "managed node", unmanaged_directory: "unmanaged directory", unmanaged_markdown: "unmanaged Markdown，只读", resource: "resource，只读" };
  for (const item of rendered.items) {
    const row = document.createElement("div");
    row.className = `contents-row ${item.kind}`;
    row.setAttribute("role", "listitem");
    const contents = `${workspaceItemIcon(item.displayIcon)}<span dir="auto">${escapeHtml(item.name)}</span><small>${labels[item.kind]}</small>`;
    if (item.kind === "managed_node" && item.nodeId) {
      const button = document.createElement("button");
      button.setAttribute("aria-label", `打开节点 ${item.name}`);
      button.innerHTML = contents;
      button.addEventListener("click", () => void openNode(item.nodeId, "contents"));
      row.append(button);
    } else if (item.kind === "unmanaged_directory") {
      const button = document.createElement("button");
      button.setAttribute("aria-label", `浏览文件夹 ${item.name}`);
      button.innerHTML = contents;
      button.addEventListener("click", () => browseContents(item.locator));
      row.append(button);
    } else {
      const inventory = document.createElement("div");
      inventory.setAttribute("aria-label", `${item.name}，${labels[item.kind]}`);
      inventory.innerHTML = contents;
      row.append(inventory);
    }
    elements.contentList.append(row);
  }
  if (!rows.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.setAttribute("role", "status");
    empty.textContent = "此位置没有 Core 可见的直接子项。";
    elements.contentList.append(empty);
  }
  if (rendered.remaining) {
    const more = document.createElement("button");
    more.className = "load-more";
    more.textContent = `继续显示 ${Math.min(INITIAL_NAVIGATION_WINDOW, rendered.remaining)} 项`;
    more.addEventListener("click", () => {
      state.navigation.contentsLimit += INITIAL_NAVIGATION_WINDOW;
      renderContents();
    });
    elements.contentList.append(more);
  }
}

function renderNavigation() {
  const activity = state.navigation.activity;
  for (const [name, element] of [["explorer", elements.explorerActivity], ["search", elements.searchActivity], ["chrono", elements.chronoActivity], ["trash", elements.trashActivity]]) element.setAttribute("aria-pressed", String(activity === name));
  elements.explorerPanel.hidden = activity !== "explorer";
  elements.searchPanel.hidden = activity !== "search";
  elements.chronoPanel.hidden = activity !== "chrono";
  elements.trashPanel.hidden = activity !== "trash";
  elements.navigationPanel.setAttribute("aria-label", `${activity === "explorer" ? "Explorer" : activity === "search" ? "Search" : activity === "chrono" ? "Chrono" : "Workspace Trash"} 侧栏`);
  elements.navigationPanel.style.width = `${state.navigation.width}px`;
  elements.navigationWidth.textContent = `${state.navigation.width}px`;
  elements.hierarchyMode.setAttribute("aria-selected", String(state.navigation.mode === "hierarchy"));
  elements.contentsMode.setAttribute("aria-selected", String(state.navigation.mode === "contents"));
  elements.hierarchyView.hidden = state.navigation.mode !== "hierarchy";
  elements.contentsView.hidden = state.navigation.mode !== "contents";
  elements.navigationFilter.value = state.navigation.filter;
  renderHierarchy();
  renderContents();
  renderNavigationMetrics();
  requestAnimationFrame(() => { elements.explorerScroll.scrollTop = state.navigation.scrollTop; });
}

function renderTree() {
  renderNavigation();
}

function renderContent() {
  renderNavigation();
}

function renderPreview(model) {
  elements.preview.replaceChildren();
  for (const block of previewLines(model)) {
    const item = document.createElement(block.kind === "heading" ? "h3" : "p");
    item.className = `preview-${block.kind}`;
    item.textContent = block.text;
    if (block.kind === "heading" && block.level) {
      item.setAttribute("role", "heading");
      item.setAttribute("aria-level", String(block.level));
    }
    if (block.kind === "quote" && block.quoteDepth) {
      item.style.marginInlineStart = `${Math.min(block.quoteDepth, 9)}rem`;
    }
    elements.preview.append(item);
  }
}

function renderDocumentMetadata(documentPayload) {
  elements.nodeMetadataValues.replaceChildren();
  elements.nodeMetadataAliases.replaceChildren();
  elements.nodeMetadataDiagnostics.replaceChildren();
  elements.documentPropertyValues.replaceChildren();
  const metadata = documentPayload.metadata;
  const fields = [
    ["节点 ID", metadata.id],
    ["图标 scalar", metadata.icon ?? "未设置"],
    ["直接子节点排序", `${metadata.childSort} / ${metadata.childSortDirection}`],
    ["同级 rank", metadata.siblingRank ?? "未设置"],
  ];
  if (metadata.adjacentHeadingBody !== null) fields.push(["紧邻标题 + 正文", metadata.adjacentHeadingBody]);
  for (const [name, value] of fields) {
    const term = document.createElement("dt");
    const description = document.createElement("dd");
    term.textContent = name;
    description.textContent = String(value);
    elements.nodeMetadataValues.append(term, description);
  }
  const aliasLabel = document.createElement("strong");
  aliasLabel.textContent = "别名";
  elements.nodeMetadataAliases.append(aliasLabel);
  if (metadata.aliases.length) {
    for (const alias of metadata.aliases) {
      const item = document.createElement("span");
      item.textContent = alias;
      elements.nodeMetadataAliases.append(item);
    }
  } else {
    elements.nodeMetadataAliases.append(document.createTextNode("未设置"));
  }
  for (const diagnostic of metadata.diagnostics) {
    const item = document.createElement("p");
    item.textContent = `${diagnostic.code}: ${diagnostic.message}`;
    elements.nodeMetadataDiagnostics.append(item);
  }
  for (const property of documentPayload.properties.properties) {
    const term = document.createElement("dt");
    const description = document.createElement("dd");
    term.textContent = property.name;
    description.textContent = property.value;
    elements.documentPropertyValues.append(term, description);
  }
  if (!documentPayload.properties.properties.length) {
    elements.documentPropertyValues.append(document.createTextNode("没有可投影的文档头属性"));
  }
}

const inspectorTabs = ["properties", "annotations", "tasks", "query", "permissions"];

function chooseInspectorTab(tab, { focus = false } = {}) {
  if (!inspectorTabs.includes(tab)) return;
  state.inspectorTab = tab;
  for (const name of inspectorTabs) {
    const button = elements[`${name === "permissions" ? "permissionsButton" : `${name}Tab`}`];
    const panel = elements[`${name}Panel`];
    const active = name === tab;
    button.setAttribute("aria-selected", String(active));
    button.tabIndex = active ? 0 : -1;
    panel.hidden = !active;
  }
  if (focus) elements[`${tab === "permissions" ? "permissionsButton" : `${tab}Tab`}`].focus();
  if (tab === "annotations") void loadAnnotations();
  if (tab === "tasks") void loadTasks();
  if (tab === "permissions") void loadPermissions();
}

function handleInspectorTabKey(event) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const current = inspectorTabs.indexOf(state.inspectorTab);
  let next = current;
  if (event.key === "ArrowLeft") next = (current - 1 + inspectorTabs.length) % inspectorTabs.length;
  if (event.key === "ArrowRight") next = (current + 1) % inspectorTabs.length;
  if (event.key === "Home") next = 0;
  if (event.key === "End") next = inspectorTabs.length - 1;
  chooseInspectorTab(inspectorTabs[next], { focus: true });
}

function surfaceButton(label, action, disabled = false) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.disabled = disabled;
  button.addEventListener("click", action);
  return button;
}

function typedSelect(label, values, current = "") {
  const select = document.createElement("select");
  select.setAttribute("aria-label", label);
  for (const [value, text] of values) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = text;
    option.selected = current === value;
    select.append(option);
  }
  return select;
}

function mutationUnavailableMessage(capability) {
  if (!capability) return "当前角色没有此写入能力；Server 仍提供 permission-filtered 只读投影。";
  if (hasDirtyDocument()) return "当前有未提交的设备草稿；先保存或放弃后再执行结构化操作。";
  return "";
}

function annotationMutationEnabled() {
  return currentSurfaceAccess().writeAnnotations && !hasDirtyDocument() && Boolean(state.annotations);
}

function setAnnotationComposerEnabled(enabled) {
  for (const name of [
    "annotationKind", "annotationTarget", "annotationBody", "annotationSuggestedSource",
    "annotationLabels", "annotationMark", "annotationTheme",
  ]) {
    if (elements[name]) elements[name].disabled = !enabled;
  }
  if (elements.annotationCreateButton) elements.annotationCreateButton.disabled = !enabled;
  if (!enabled || !elements.annotationKind) return;
  const kind = elements.annotationKind.value;
  if (kind === "suggestion_insert") elements.annotationTarget.value = "insertion_point";
  if (kind === "suggestion_delete") elements.annotationTarget.value = "text_range";
  elements.annotationTarget.disabled = ["suggestion_insert", "suggestion_delete"].includes(kind);
  elements.annotationSuggestedSource.disabled = kind !== "suggestion_insert";
}

async function loadAnnotations() {
  if (!state.document || !currentSurfaceAccess().readAnnotations) return;
  const context = captureSurfaceContext(state.workspace, state.document);
  const token = annotationGate.begin();
  elements.annotationStatus.textContent = "正在读取批注…";
  try {
    const payload = requireAnnotationReadPayload(
      await serverApi.annotations(context.nodeId),
      context.nodeId,
    );
    if (!annotationGate.isCurrent(token)
      || state.inspectorTab !== "annotations"
      || !surfaceContextMatches(context, state.workspace, state.document)) return;
    state.annotations = payload;
    renderAnnotations();
  } catch (error) {
    if (annotationGate.isCurrent(token) && state.inspectorTab === "annotations") {
      state.annotations = null;
      elements.annotationList.replaceChildren();
      elements.annotationStatus.textContent = error instanceof Error ? error.message : "批注读取失败";
    }
  }
}

function renderAnnotations() {
  elements.annotationList.replaceChildren();
  const annotations = state.annotations?.store?.annotations ?? [];
  const enabled = annotationMutationEnabled();
  setAnnotationComposerEnabled(enabled);
  elements.annotationStatus.textContent = mutationUnavailableMessage(currentSurfaceAccess().writeAnnotations)
    || `${annotations.length} 条批注 · sidecar v${state.annotations.store.version}`;
  for (const annotation of annotations) {
    const card = document.createElement("article");
    card.className = "surface-card annotation-card";
    card.setAttribute("role", "listitem");
    const heading = document.createElement("h4");
    heading.textContent = `${annotation.kind} · ${annotation.state}`;
    const meta = document.createElement("p");
    meta.textContent = `目标 ${annotation.target?.kind ?? "unknown"} · ${annotation.labels?.join(", ") || "无标签"}`;
    card.append(heading, meta);
    if (typeof annotation.suggested_source === "string") {
      const suggestion = document.createElement("pre");
      suggestion.className = "annotation-suggestion";
      suggestion.setAttribute("aria-label", `批注 ${annotation.id} 建议源码`);
      suggestion.textContent = annotation.suggested_source;
      card.append(suggestion);
    }

    const messages = document.createElement("div");
    messages.className = "annotation-thread";
    for (const message of annotation.thread ?? []) {
      const messageRow = document.createElement("section");
      const messageLabel = document.createElement("strong");
      messageLabel.textContent = message.author_name;
      const editor = document.createElement("textarea");
      editor.value = message.body?.source ?? "";
      editor.rows = 2;
      editor.disabled = !enabled;
      editor.setAttribute("aria-label", `编辑消息 ${message.id}`);
      const save = surfaceButton("保存消息", () => void commitAnnotationAction("edit_message", {
        annotationId: annotation.id,
        messageId: message.id,
        bodySource: editor.value,
      }), !enabled);
      messageRow.append(messageLabel, editor, save);
      messages.append(messageRow);
    }
    card.append(messages);

    const reply = document.createElement("textarea");
    reply.rows = 2;
    reply.placeholder = "回复（AsciiDoc inline）";
    reply.disabled = !enabled;
    reply.setAttribute("aria-label", `回复批注 ${annotation.id}`);
    const replyButton = surfaceButton("回复", () => void commitAnnotationAction("reply", {
      annotationId: annotation.id,
      bodySource: reply.value,
    }), !enabled);
    const labels = document.createElement("input");
    labels.value = (annotation.labels ?? []).join(", ");
    labels.disabled = !enabled;
    labels.setAttribute("aria-label", `批注 ${annotation.id} 标签`);
    const labelsButton = surfaceButton("更新标签", () => void commitAnnotationAction("set_labels", {
      annotationId: annotation.id,
      labels: labels.value.split(",").map((label) => label.trim()).filter(Boolean),
    }), !enabled);

    const mark = document.createElement("select");
    mark.setAttribute("aria-label", `批注 ${annotation.id} 外观`);
    for (const value of ["none", "highlight", "underline", "squiggle", "strike"]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = value;
      option.selected = (annotation.appearance?.mark ?? "none") === value;
      mark.append(option);
    }
    mark.disabled = !enabled;
    const theme = document.createElement("select");
    theme.setAttribute("aria-label", `批注 ${annotation.id} 主题`);
    for (const value of ["yellow", "red", "green", "blue", "purple", "pink", "gray"]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = value;
      option.selected = (annotation.appearance?.theme ?? "yellow") === value;
      theme.append(option);
    }
    theme.disabled = !enabled;
    const appearanceButton = surfaceButton("更新外观", () => void commitAnnotationAction("set_appearance", {
      annotationId: annotation.id,
      appearance: mark.value === "none" ? { mark: "none" } : { mark: mark.value, theme: theme.value },
    }), !enabled);
    const controls = document.createElement("div");
    controls.className = "surface-actions";
    controls.append(
      surfaceButton(annotation.state === "resolved" ? "重新打开" : "解决", () => void commitAnnotationAction(annotation.state === "resolved" ? "reopen" : "resolve", { annotationId: annotation.id }), !enabled),
      surfaceButton("重新锚定", () => void commitAnnotationAction("reanchor", { annotationId: annotation.id }), !enabled),
    );
    if (["suggestion_insert", "suggestion_delete"].includes(annotation.kind) && annotation.state === "open") {
      controls.append(
        surfaceButton("接受建议", () => void commitAnnotationAction("accept_suggestion", { annotationId: annotation.id }), !enabled || !currentSurfaceAccess().editDocuments),
        surfaceButton("拒绝建议", () => void commitAnnotationAction("reject_suggestion", { annotationId: annotation.id }), !enabled),
      );
    }
    card.append(reply, replyButton, labels, labelsButton, mark, theme, appearanceButton, controls);
    elements.annotationList.append(card);
  }
  if (!annotations.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.setAttribute("role", "status");
    empty.textContent = "当前节点没有批注。";
    elements.annotationList.append(empty);
  }
}

async function refreshAfterStructuredCommit(nodeId, gate, token, message) {
  const [workspace, documentPayload] = await Promise.all([
    serverApi.inventory(),
    serverApi.openDocument(nodeId),
  ]);
  if (!gate.isCurrent(token) || state.document?.nodeId !== nodeId) return;
  state.workspace = requireCanonicalWorkspacePayload(workspace);
  applyDocumentPayload(documentPayload, { restoreDraft: false });
  renderNavigation();
  setStatus(message);
}

async function commitAnnotationAction(action, fields) {
  if (!annotationMutationEnabled() || !state.document || !state.annotations) return;
  if (action === "accept_suggestion" && !currentSurfaceAccess().editDocuments) {
    elements.annotationStatus.textContent = "接受建议会修改文档；当前角色只有批注写入能力。";
    return;
  }
  const context = captureSurfaceContext(state.workspace, state.document, action);
  const token = annotationGate.begin();
  const request = {
    baseWorkspaceRevision: state.annotations.workspaceRevision,
    baseRevision: state.annotations.revision,
    action,
    nodeId: context.nodeId,
    ...fields,
    timestamp: new Date().toISOString(),
  };
  elements.annotationStatus.textContent = "正在提交批注事务…";
  try {
    await serverApi.commitAnnotation(context.nodeId, request);
    if (!annotationGate.isCurrent(token)
      || !surfaceContextMatches(context, state.workspace, state.document, action)) return;
    if (action === "create") {
      elements.annotationBody.value = "";
      elements.annotationSuggestedSource.value = "";
    }
    await refreshAfterStructuredCommit(context.nodeId, annotationGate, token, "批注事务已提交并重新读取 authoritative 文档");
  } catch (error) {
    if (annotationGate.isCurrent(token)
      && surfaceContextMatches(context, state.workspace, state.document, action)) {
      elements.annotationStatus.textContent = error instanceof Error ? error.message : "批注事务失败";
      showFailure(error);
    }
  }
}

function submitAnnotationCreate(event) {
  event.preventDefault();
  if (!state.document) return;
  const kind = elements.annotationKind.value;
  if (kind === "comment" && !elements.annotationBody.value.trim()) {
    elements.annotationStatus.textContent = "评论必须包含 AsciiDoc inline 正文。";
    return;
  }
  if (kind === "mark" && elements.annotationMark.value === "none") {
    elements.annotationStatus.textContent = "标记批注必须选择一种外观。";
    return;
  }
  if (kind === "suggestion_insert" && !elements.annotationSuggestedSource.value) {
    elements.annotationStatus.textContent = "插入建议必须包含建议源码。";
    return;
  }
  let target;
  try {
    target = annotationTargetFromSelection(
      elements.annotationTarget.value,
      state.document.source,
      elements.editor.selectionStart,
      elements.editor.selectionEnd,
    );
  } catch (error) {
    elements.annotationStatus.textContent = error instanceof Error ? error.message : "批注目标无效";
    return;
  }
  if (kind === "suggestion_delete" && target.start === target.end) {
    elements.annotationStatus.textContent = "删除建议必须选择非空源码范围。";
    return;
  }
  const mark = elements.annotationMark.value;
  const fields = {
    kind,
    target,
    labels: elements.annotationLabels.value.split(",").map((label) => label.trim()).filter(Boolean),
  };
  if (elements.annotationBody.value) fields.bodySource = elements.annotationBody.value;
  if (kind === "suggestion_insert") fields.suggestedSource = elements.annotationSuggestedSource.value;
  if (mark !== "none") fields.appearance = { mark, theme: elements.annotationTheme.value };
  void commitAnnotationAction("create", fields);
}

function taskMutationEnabled() {
  return currentSurfaceAccess().writeTasks && !hasDirtyDocument() && Boolean(state.tasks);
}

async function loadTasks() {
  if (!state.document || !currentSurfaceAccess().readTasks) return;
  const context = captureSurfaceContext(state.workspace, state.document);
  const token = taskGate.begin();
  elements.taskStatus.textContent = "正在读取任务…";
  try {
    const payload = requireTaskInspectionPayload(
      await serverApi.inspectTasks(context.nodeId),
      context.nodeId,
    );
    if (!taskGate.isCurrent(token)
      || state.inspectorTab !== "tasks"
      || !surfaceContextMatches(context, state.workspace, state.document)) return;
    state.tasks = payload;
    renderTasks();
  } catch (error) {
    if (taskGate.isCurrent(token) && state.inspectorTab === "tasks") {
      state.tasks = null;
      elements.taskList.replaceChildren();
      elements.taskStatus.textContent = error instanceof Error ? error.message : "任务读取失败";
    }
  }
}

function renderTasks() {
  elements.taskList.replaceChildren();
  const occurrences = state.tasks?.occurrences ?? [];
  const enabled = taskMutationEnabled();
  elements.taskStatus.textContent = mutationUnavailableMessage(currentSurfaceAccess().writeTasks)
    || `${occurrences.length} 个任务 · ${state.tasks.diagnostics.length} 条诊断`;
  for (const occurrence of occurrences) {
    const task = occurrence.task;
    const occurrenceEnabled = enabled && task.valid !== false;
    const card = document.createElement("article");
    card.className = "surface-card task-card";
    card.setAttribute("role", "listitem");
    const heading = document.createElement("h4");
    heading.textContent = task.description;
    const meta = document.createElement("p");
    meta.textContent = `${task.state} · ${task.metadata?.priority ?? "normal"} · ${task.metadata?.phase ?? "无 phase"}`;
    const actions = document.createElement("div");
    actions.className = "surface-actions";
    if (!task.metadata?.recurrence) {
      actions.append(surfaceButton("切换完成状态", () => void previewTask("edit", occurrence, { kind: "toggle" }), !occurrenceEnabled));
    }

    const priority = typedSelect(`任务 ${task.description} 优先级`, [
      ["", "清除显式优先级"], ["lowest", "lowest"], ["low", "low"], ["normal", "normal"],
      ["medium", "medium"], ["high", "high"], ["highest", "highest"],
    ], task.metadata?.priority ?? "");
    priority.disabled = !occurrenceEnabled;
    actions.append(priority, surfaceButton("预览优先级", () => void previewTask("edit", occurrence, {
      kind: "set_priority",
      priority: priority.value || null,
    }), !occurrenceEnabled));

    const phase = typedSelect(`任务 ${task.description} phase`, [
      ["", "清除 phase"], ["todo", "todo"], ["in-progress", "in-progress"], ["on-hold", "on-hold"],
    ], task.metadata?.phase ?? "");
    phase.disabled = !occurrenceEnabled;
    actions.append(phase, surfaceButton("预览 phase", () => void previewTask("edit", occurrence, {
      kind: "set_phase",
      phase: phase.value || null,
    }), !occurrenceEnabled));

    const resolution = typedSelect(`任务 ${task.description} resolution`, [
      ["", "清除 resolution"], ["completed", "completed"], ["cancelled", "cancelled"],
    ], task.metadata?.resolution ?? "");
    resolution.disabled = !occurrenceEnabled;
    actions.append(resolution, surfaceButton("预览 resolution", () => void previewTask("edit", occurrence, {
      kind: "set_resolution",
      resolution: resolution.value || null,
    }), !occurrenceEnabled));

    const dateField = typedSelect(`任务 ${task.description} 日期字段`, [
      ["created", "created"], ["start", "start"], ["scheduled", "scheduled"], ["due", "due"], ["closed", "closed"],
    ]);
    const dateKind = typedSelect(`任务 ${task.description} 日期类型`, [["date", "date"], ["instant", "instant"]]);
    const dateValue = document.createElement("input");
    dateField.disabled = !occurrenceEnabled;
    dateKind.disabled = !occurrenceEnabled;
    dateValue.disabled = !occurrenceEnabled;
    dateValue.placeholder = "YYYY-MM-DD 或 RFC 3339；留空清除";
    dateValue.setAttribute("aria-label", `任务 ${task.description} 日期值`);
    actions.append(dateField, dateKind, dateValue, surfaceButton("预览日期", () => void previewTask("edit", occurrence, {
      kind: "set_date",
      field: dateField.value,
      value: dateValue.value.trim() ? { kind: dateKind.value, value: dateValue.value.trim() } : null,
    }), !occurrenceEnabled));

    const recurrenceRule = document.createElement("input");
    recurrenceRule.value = task.metadata?.recurrence?.source ?? "";
    recurrenceRule.disabled = !occurrenceEnabled;
    recurrenceRule.placeholder = "RRULE（留空清除）";
    recurrenceRule.setAttribute("aria-label", `任务 ${task.description} recurrence rule`);
    const repeatFrom = typedSelect(`任务 ${task.description} repeat from`, [
      ["", "清除 repeat-from"], ["due", "due"], ["scheduled", "scheduled"], ["completion", "completion"],
    ], task.metadata?.repeatFrom ?? "");
    repeatFrom.disabled = !occurrenceEnabled;
    actions.append(recurrenceRule, repeatFrom, surfaceButton("预览 recurrence", () => void previewTask("edit", occurrence, {
      kind: "set_recurrence",
      rrule: recurrenceRule.value.trim() || null,
      repeat_from: repeatFrom.value || null,
    }), !occurrenceEnabled));

    if (task.metadata?.recurrence) {
      actions.append(surfaceButton("预览完成本次重复任务", () => void previewTask("recurrence", occurrence, {
        completedAt: { kind: "instant", value: new Date().toISOString() },
        utcOffsetMinutes: -new Date().getTimezoneOffset(),
      }), !occurrenceEnabled));
    }
    const dependencies = document.createElement("input");
    dependencies.value = (task.metadata?.dependencies ?? []).join(", ");
    dependencies.disabled = !occurrenceEnabled;
    dependencies.placeholder = "依赖 UUID，逗号分隔";
    dependencies.setAttribute("aria-label", `任务 ${task.description} 依赖`);
    actions.append(dependencies, surfaceButton("预览依赖替换", () => void previewTask("dependencies", occurrence,
      dependencies.value.split(",").map((id) => id.trim()).filter(Boolean)), !occurrenceEnabled));
    card.append(heading, meta, actions);
    elements.taskList.append(card);
  }
  if (!occurrences.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.setAttribute("role", "status");
    empty.textContent = "当前节点没有任务 occurrence。";
    elements.taskList.append(empty);
  }
  for (const diagnostic of state.tasks?.diagnostics ?? []) {
    const item = document.createElement("p");
    item.className = "surface-error";
    item.textContent = `${diagnostic.code}: ${diagnostic.message}`;
    elements.taskList.append(item);
  }
  renderTaskPlan();
}

async function previewTask(kind, occurrence, typedValue) {
  if (!taskMutationEnabled() || !state.document) return;
  let target;
  try {
    target = taskTargetForOccurrence(occurrence);
  } catch (error) {
    elements.taskStatus.textContent = error instanceof Error ? error.message : "任务目标无效";
    return;
  }
  const context = captureSurfaceContext(state.workspace, state.document, `${kind}:${JSON.stringify(typedValue)}`);
  const request = {
    baseWorkspaceRevision: context.workspaceRevision,
    baseRevision: context.revision,
    target,
  };
  const token = taskGate.begin();
  state.taskPlan = null;
  state.taskPlanConfirmed = false;
  renderTaskPlan();
  elements.taskStatus.textContent = "正在生成固定任务事务预览…";
  try {
    let payload;
    if (kind === "edit") payload = await serverApi.previewTaskEdit(context.nodeId, { ...request, intent: typedValue });
    if (kind === "recurrence") payload = await serverApi.previewTaskRecurrence(context.nodeId, { ...request, context: typedValue });
    if (kind === "dependencies") payload = await serverApi.previewTaskDependencies(context.nodeId, { ...request, dependencies: typedValue });
    payload = requireTaskPreviewPayload(payload, context.nodeId);
    if (!taskGate.isCurrent(token)
      || state.inspectorTab !== "tasks"
      || !surfaceContextMatches(context, state.workspace, state.document, context.input)) return;
    const confirmation = taskPreviewConfirmation(payload, context.nodeId);
    state.taskPlan = { payload, context, kind, typedValue, confirmation };
    elements.taskStatus.textContent = "固定预览已缓存；请核对 typed intent、生成身份与 exact proposed source。";
    renderTaskPlan();
  } catch (error) {
    if (taskGate.isCurrent(token) && surfaceContextMatches(context, state.workspace, state.document, context.input)) {
      elements.taskStatus.textContent = error instanceof ApiFailure && error.code === "authorization_denied"
        ? "当前角色或继承节点 ACL 拒绝任务写入；未生成事务预览。"
        : error instanceof ApiFailure && ["stale_revision", "stale_workspace_revision"].includes(error.code)
          ? "任务基线已经 stale；请等待 authoritative refresh 后重新生成预览。"
          : error instanceof Error ? error.message : "任务预览失败";
      showFailure(error);
    }
  }
}

function renderTaskPlan() {
  const pending = state.taskPlan;
  elements.taskPlanPanel.hidden = !pending;
  elements.taskPlanChanges.replaceChildren();
  if (!pending) return;
  elements.taskPlanId.textContent = pending.payload.planId;
  elements.taskPlanIntent.textContent = JSON.stringify({
    route: pending.kind,
    value: pending.typedValue,
  }, null, 2);
  elements.taskPlanGenerated.textContent = pending.confirmation.generatedTaskIds.length
    ? `此计划将生成任务身份：${pending.confirmation.generatedTaskIds.join(", ")}`
    : "此计划不生成新的任务身份。";
  elements.taskPlanSource.textContent = pending.confirmation.proposedSource;
  for (const change of pending.payload.documentChanges) {
    const item = document.createElement("p");
    item.textContent = `${change.path} · ${change.editCount} 个精确 edit · ${String(change.baseRevision).slice(0, 12)} → ${String(change.nextRevision).slice(0, 12)}`;
    elements.taskPlanChanges.append(item);
  }
  elements.taskPlanConfirm.checked = state.taskPlanConfirmed;
  elements.taskPlanConfirm.disabled = state.taskPlanPending || !taskMutationEnabled();
  elements.taskPlanCommitButton.disabled = state.taskPlanPending
    || !state.taskPlanConfirmed
    || !taskMutationEnabled();
  elements.taskPlanCancelButton.disabled = state.taskPlanPending;
}

async function commitTaskPlan() {
  const pending = state.taskPlan;
  if (!pending || state.taskPlanPending || !state.taskPlanConfirmed || !taskMutationEnabled()) return;
  if (!surfaceContextMatches(pending.context, state.workspace, state.document, pending.context.input)) {
    state.taskPlan = null;
    state.taskPlanConfirmed = false;
    renderTaskPlan();
    elements.taskStatus.textContent = "节点或 revision 已变化；任务预览已失效。";
    return;
  }
  const token = taskGate.begin();
  state.taskPlan = null;
  state.taskPlanConfirmed = false;
  state.taskPlanPending = true;
  renderTaskPlan();
  elements.taskStatus.textContent = "正在单次提交固定 planId…";
  try {
    await serverApi.commitTask(pending.payload.planId);
    state.taskPlanPending = false;
    if (!taskGate.isCurrent(token) || state.document?.nodeId !== pending.context.nodeId) return;
    await refreshAfterStructuredCommit(pending.context.nodeId, taskGate, token, "任务事务已提交并重新读取 authoritative 文档");
  } catch (error) {
    state.taskPlanPending = false;
    if (taskGate.isCurrent(token) && state.document?.nodeId === pending.context.nodeId) {
      elements.taskStatus.textContent = error instanceof ApiFailure
        && ["stale_revision", "stale_workspace_revision", "task_transaction_rejected"].includes(error.code)
        ? "任务提交被 stale/不可安全重放检查拒绝；预览已消费，正在保留 authoritative 文档，请刷新后重新生成。"
        : `${error instanceof Error ? error.message : "任务提交失败"}；预览已消费，请重新生成。`;
      showFailure(error);
    }
  }
}

function formatQueryValue(value) {
  if (!value || value.kind === "null") return "";
  if (value.value === null || value.value === undefined) return String(value.kind);
  if (typeof value.value === "object") return JSON.stringify(value.value);
  return String(value.value);
}

function localInstant(date = new Date()) {
  const pad = (value, width = 2) => String(value).padStart(width, "0");
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes < 0 ? "-" : "+";
  const absoluteOffset = Math.abs(offsetMinutes);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
    + `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`
    + `${sign}${pad(Math.floor(absoluteOffset / 60))}:${pad(absoluteOffset % 60)}`;
}

function queryIdentityNode(identity) {
  if (!identity || typeof identity !== "object") return null;
  if (identity.kind === "node" || identity.kind === "heading") {
    return { nodeId: identity.nodeId, revision: identity.revision };
  }
  if (identity.kind !== "task" || !identity.evidence) return null;
  return identity.evidence.kind === "checklist"
    ? { nodeId: identity.evidence.ownerNodeId, revision: identity.evidence.revision }
    : { nodeId: identity.evidence.nodeId, revision: identity.evidence.revision };
}

function renderQuery(payload) {
  elements.queryDiagnostics.replaceChildren();
  elements.queryGroups.replaceChildren();
  elements.queryTableContainer.replaceChildren();
  for (const diagnostic of payload.execution.analysis.diagnostics) {
    const item = document.createElement("p");
    item.textContent = `${diagnostic.code}: ${diagnostic.message}`;
    elements.queryDiagnostics.append(item);
  }
  const result = payload.execution.result;
  state.query = payload;
  elements.queryCsvButton.disabled = typeof payload.execution.csv !== "string";
  if (!result) {
    elements.queryStatus.textContent = `Core 未执行所选块；${payload.execution.analysis.diagnostics.length} 条诊断。`;
    return;
  }
  elements.queryStatus.textContent = `${result.rows.length}/${result.totalBeforeLimit} 行${result.truncated ? " · 已按查询 limit 截断" : ""}`;
  for (const group of result.groups) {
    const item = document.createElement("span");
    item.textContent = `${group.column.outputName}: ${formatQueryValue(group.value) || "(空)"}: ${group.rowCount}`;
    elements.queryGroups.append(item);
  }
  const table = document.createElement("table");
  table.setAttribute("aria-label", "Core 查询结果");
  const head = document.createElement("thead");
  const headRow = document.createElement("tr");
  for (const column of result.columns) {
    const cell = document.createElement("th");
    cell.scope = "col";
    cell.textContent = column.outputName;
    cell.title = column.path;
    headRow.append(cell);
  }
  head.append(headRow);
  const body = document.createElement("tbody");
  for (const row of result.rows) {
    const tableRow = document.createElement("tr");
    const owner = queryIdentityNode(row.identity);
    for (const [index, cell] of row.cells.entries()) {
      const tableCell = document.createElement("td");
      const value = formatQueryValue(cell.value);
      const visibleIdentity = owner?.nodeId
        && state.workspace?.nodes?.some((node) => node.id === owner.nodeId);
      if (index === 0 && visibleIdentity) {
        const open = surfaceButton(value || "打开节点", () => void openNode(owner.nodeId, "query"));
        open.setAttribute("aria-label", `打开查询结果节点 ${value || owner.nodeId}`);
        tableCell.append(open);
      } else {
        tableCell.textContent = value;
      }
      tableRow.append(tableCell);
    }
    body.append(tableRow);
  }
  table.append(head, body);
  elements.queryTableContainer.append(table);
}

async function runQuery(event) {
  event?.preventDefault();
  if (!currentSurfaceAccess().executeQueries || !state.document) return;
  const source = elements.querySource.value;
  const blockIndex = Number(elements.queryBlockIndex.value);
  if (!Number.isSafeInteger(blockIndex) || blockIndex < 0) {
    elements.queryStatus.textContent = "查询块序号必须是非负整数。";
    return;
  }
  if (!/^\d{4}-\d{2}-\d{2}$/.test(elements.queryToday.value)) {
    elements.queryStatus.textContent = "请选择有效的查询求值日期。";
    return;
  }
  const [year, month, day] = elements.queryToday.value.split("-").map(Number);
  const input = `${source}\u0000${blockIndex}\u0000${elements.queryToday.value}`;
  const context = captureSurfaceContext(state.workspace, state.document, input);
  const token = queryGate.begin();
  elements.queryStatus.textContent = "正在执行 permission-filtered Core 查询…";
  try {
    const payload = requireQueryExecutionPayload(await serverApi.executeQuery(source, blockIndex, {
      today: { year, month, day },
      now: localInstant(),
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      locale: navigator.language,
      binding: { nodeId: context.nodeId, heading: null },
    }));
    if (!queryGate.isCurrent(token)
      || state.inspectorTab !== "query"
      || !surfaceContextMatches(context, state.workspace, state.document, input)
      || elements.querySource.value !== source) return;
    renderQuery(payload);
  } catch (error) {
    if (queryGate.isCurrent(token)
      && surfaceContextMatches(context, state.workspace, state.document, input)
      && elements.querySource.value === source) {
      elements.queryStatus.textContent = error instanceof Error ? error.message : "查询失败";
      showFailure(error);
    }
  }
}

function downloadQueryCsv() {
  const csv = state.query?.execution?.csv;
  if (typeof csv !== "string") return;
  const locator = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
  const anchor = document.createElement("a");
  anchor.href = locator;
  anchor.download = "weftext-query.csv";
  anchor.click();
  URL.revokeObjectURL(locator);
}

function readDeviceDrafts() {
  try {
    const storageKey = deviceDraftStorageKey(currentWorkspaceScope(), currentActorScope());
    if (!storageKey) return {};
    const value = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}");
    return value && typeof value === "object" ? value : {};
  } catch {
    return {};
  }
}

function saveDeviceDraft(draft) {
  try {
    const workspaceScope = currentWorkspaceScope();
    const actorScope = currentActorScope();
    const storageKey = deviceDraftStorageKey(workspaceScope, actorScope);
    if (!storageKey || !isDeviceDraftForWorkspace(draft, workspaceScope, actorScope)) return false;
    const drafts = readDeviceDrafts();
    window.localStorage.setItem(storageKey, JSON.stringify({ ...drafts, [draft.nodeId]: draft }));
    return true;
  } catch {
    return false;
  }
}

function storedDeviceDraft(nodeId) {
  const draft = readDeviceDrafts()[nodeId] ?? null;
  return isDeviceDraftForWorkspace(draft, currentWorkspaceScope(), currentActorScope(), nodeId) ? draft : null;
}

function removeDeviceDraft(nodeId, predicate = () => true) {
  try {
    const workspaceScope = currentWorkspaceScope();
    const actorScope = currentActorScope();
    const storageKey = deviceDraftStorageKey(workspaceScope, actorScope);
    if (!storageKey) return;
    const drafts = readDeviceDrafts();
    if (!isDeviceDraftForWorkspace(drafts[nodeId], workspaceScope, actorScope, nodeId) || !predicate(drafts[nodeId])) return;
    delete drafts[nodeId];
    window.localStorage.setItem(storageKey, JSON.stringify(drafts));
  } catch {
    // A failed cleanup cannot turn a committed Core operation into failure.
  }
}

function preserveOutgoingDraft(nodeId, origin) {
  return safeguardOutgoingDraft({
    origin,
    currentDocument: state.document,
    workspaceScope: currentWorkspaceScope(),
    actorScope: currentActorScope(),
    targetNodeId: nodeId,
    source: elements.editor.value,
    saveDraft: saveDeviceDraft,
  });
}

function resetNodeSurfaces() {
  annotationGate.invalidate();
  taskGate.invalidate();
  queryGate.invalidate();
  state.annotations = null;
  state.tasks = null;
  state.taskPlan = null;
  state.taskPlanConfirmed = false;
  state.taskPlanPending = false;
  state.query = null;
  setAnnotationComposerEnabled(false);
  elements.annotationList?.replaceChildren();
  elements.taskList?.replaceChildren();
  elements.taskPlanChanges?.replaceChildren();
  if (elements.taskPlanIntent) elements.taskPlanIntent.textContent = "";
  if (elements.taskPlanGenerated) elements.taskPlanGenerated.textContent = "";
  if (elements.taskPlanSource) elements.taskPlanSource.textContent = "";
  if (elements.taskPlanConfirm) elements.taskPlanConfirm.checked = false;
  if (elements.taskPlanPanel) elements.taskPlanPanel.hidden = true;
  elements.queryDiagnostics?.replaceChildren();
  elements.queryGroups?.replaceChildren();
  elements.queryTableContainer?.replaceChildren();
  if (elements.queryCsvButton) elements.queryCsvButton.disabled = true;
}

function renderCollaboration() {
  if (!elements.collaborationStatus || !elements.collaborationParticipants) return;
  elements.collaborationParticipants.replaceChildren();
  const collaboration = state.collaboration;
  if (!collaboration || collaboration.nodeId !== state.document?.nodeId) {
    elements.collaborationStatus.dataset.state = "resync";
    elements.collaborationState.textContent = state.collaborationNotice
      ?? "实时协作：正在读取 Server authoritative 快照（resync）";
    elements.collaborationRevision.textContent = "";
    return;
  }
  const documentState = collaboration.state;
  elements.collaborationStatus.dataset.state = documentState.frozen ? "frozen" : "ready";
  elements.collaborationState.textContent = documentState.frozen
    ? `实时协作已冻结：${documentState.reason ?? "需要重同步"}`
    : `实时协作就绪 · epoch ${documentState.epoch} · version ${documentState.version}`;
  elements.collaborationRevision.textContent = documentState.revision;
  for (const participant of collaboration.participants ?? []) {
    const item = document.createElement("span");
    item.setAttribute("role", "listitem");
    item.textContent = `${participant.role} · ${participant.actorId.slice(0, 8)} · 光标 ${participant.cursor}`;
    elements.collaborationParticipants.append(item);
  }
  if (!(collaboration.participants ?? []).length) {
    const empty = document.createElement("span");
    empty.setAttribute("role", "listitem");
    empty.textContent = "暂无活动参与者";
    elements.collaborationParticipants.append(empty);
  }
}

async function refreshCollaboration(nodeId = state.document?.nodeId) {
  if (!nodeId || nodeId !== state.document?.nodeId) return;
  try {
    const snapshot = await serverApi.collaborationSnapshot(nodeId);
    if (nodeId !== state.document?.nodeId) return;
    state.collaboration = snapshot;
    state.collaborationNotice = null;
    renderCollaboration();
    await publishCollaborationPresence();
  } catch (error) {
    if (nodeId !== state.document?.nodeId) return;
    state.collaboration = null;
    state.collaborationNotice = "实时协作快照不可用；界面保持 resync，未推测服务器状态";
    renderCollaboration();
    showFailure(error);
  }
}

async function publishCollaborationPresence() {
  if (!state.document || !state.collaboration || !state.collaborationClientId
    || elements.editor.value !== state.document.source) return;
  const source = elements.editor.value;
  const cursor = new TextEncoder().encode(source.slice(0, elements.editor.selectionStart)).length;
  const selectionStart = new TextEncoder().encode(source.slice(0, elements.editor.selectionStart)).length;
  const selectionEnd = new TextEncoder().encode(source.slice(0, elements.editor.selectionEnd)).length;
  try {
    const result = await serverApi.updateCollaborationPresence(state.document.nodeId, {
      wireVersion: COLLABORATION_WIRE_VERSION,
      clientId: state.collaborationClientId,
      epoch: state.collaboration.state.epoch,
      revision: state.collaboration.state.revision,
      cursor,
      selectionStart,
      selectionEnd,
    });
    if (result.nodeId !== state.document?.nodeId) return;
    state.collaboration.participants = result.participants;
    renderCollaboration();
  } catch (error) {
    if (error instanceof ApiFailure && error.code === "authentication_required") showFailure(error);
  }
}

function applyDocumentPayload(documentPayload, { restoreDraft = true } = {}) {
  requireCanonicalDocumentPayload(documentPayload);
  resetNodeSurfaces();
  state.document = documentPayload;
  state.preview = documentPayload.model;
  let source = documentPayload.source;
  let status = "已通过 Server API 打开文档";
  let tone = "normal";
  const saved = restoreDraft ? storedDeviceDraft(documentPayload.nodeId) : null;
  if (saved?.baseRevision === documentPayload.revision && saved.source !== documentPayload.source) {
    source = saved.source;
    status = "已恢复与当前 revision 匹配的设备草稿";
    tone = "draft";
  } else if (saved && saved.baseRevision !== documentPayload.revision) {
    status = "设备草稿基于旧 revision，已保留但未覆盖服务器内容";
    tone = "conflict";
  }
  elements.title.textContent = documentPayload.name;
  elements.revision.textContent = documentPayload.revision;
  elements.editor.value = source;
  renderPreview(documentPayload.model);
  renderDocumentMetadata(documentPayload);
  renderNavigation();
  setStatus(status, tone);
  if (state.inspectorTab === "annotations") void loadAnnotations();
  if (state.inspectorTab === "tasks") void loadTasks();
  if (elements.collaborationStatus) void refreshCollaboration(documentPayload.nodeId);
}

async function openNode(nodeId, origin = "direct") {
  if (state.document?.nodeId === nodeId) {
    state.navigation.browseLocator = null;
    renderContents();
    persistNavigation();
    return;
  }
  if (!preserveOutgoingDraft(nodeId, origin)) return;
  mutationGate.invalidate();
  const navigationToken = navigationGate.begin();
  try {
    const documentPayload = await serverApi.openDocument(nodeId);
    if (!navigationGate.isCurrent(navigationToken)) return;
    state.navigation.browseLocator = null;
    applyDocumentPayload(documentPayload);
    persistNavigation();
  } catch (error) {
    if (navigationGate.isCurrent(navigationToken)) showFailure(error);
  }
}

async function refreshWorkspace() {
  const workspace = requireCanonicalWorkspacePayload(await serverApi.inventory());
  if (!deviceDraftStorageKey(workspace.workspaceScope, currentActorScope())) throw new Error("服务器工作区或身份作用域声明异常");
  const changedScope = state.workspace?.workspaceScope !== workspace.workspaceScope;
  state.workspace = workspace;
  if (changedScope) state.navigation = readNavigationState(workspace.workspaceScope);
  const projection = currentProjection();
  const validated = validateBrowseLocator(projection, state.navigation.browseLocator);
  if (state.navigation.browseLocator !== null && validated === null) {
    state.navigation.browseLocator = null;
    setStatus("先前浏览位置已不可用；Explorer 已回到当前编辑节点");
  }
  const startedAt = performance.now();
  renderNavigation();
  const total = visibleHierarchy(projection, state.navigation.collapsedNodeIds, state.navigation.filter).length;
  requestAnimationFrame(() => recordNavigationMetric(changedScope ? "initial_render" : "incremental_refresh", startedAt, Math.min(total, state.navigation.hierarchyLimit), total));
  if (!state.document && state.workspace.rootNodeId) await openNode(state.workspace.rootNodeId);
}

function selectedTrashItem() {
  return state.trashInventory?.items.find((item) => item.manifest.trashItemId === state.selectedTrashItemId) ?? null;
}

function setTrashStatus(message, error = false) {
  elements.trashStatus.textContent = message;
  elements.trashStatus.classList?.toggle("surface-error", error);
}

function renderTrashPlan() {
  elements.trashPlanPanel.hidden = !state.trashPlan;
  elements.trashPlanOutput.textContent = state.trashPlan ? JSON.stringify(state.trashPlan, null, 2) : "";
  elements.trashPlanConfirmationText.textContent = state.trashPlanPurpose === "permanent_delete"
    ? "我已核对 planId、精确 item ID、payload 摘要与总字节；永久删除不可恢复，Trash 不是备份。"
    : "我已核对 planId、item/operation ID、payload 摘要、字节数与恢复目标。";
  elements.trashPlanCommitButton.disabled = !state.trashPlan
    || !elements.trashPlanConfirm.checked
    || hasDirtyDocument()
    || Object.keys(readDeviceDrafts()).length > 0;
}

function renderTrashInventory() {
  elements.trashItemList.replaceChildren();
  elements.trashRestoreTarget.replaceChildren();
  const blankTarget = document.createElement("option");
  blankTarget.value = "";
  blankTarget.textContent = "请选择现有目标…";
  elements.trashRestoreTarget.append(blankTarget);
  for (const node of state.workspace?.nodes ?? []) {
    const option = document.createElement("option");
    option.value = node.id;
    option.textContent = node.locator || node.name;
    elements.trashRestoreTarget.append(option);
  }
  const reconciliation = state.trashInventory?.reconciliation;
  const legacyMigrationRequired = Boolean(state.trashInventory?.legacyMigrationRequired);
  elements.trashMigrationPreviewButton.hidden = !legacyMigrationRequired;
  elements.trashMigrationPreviewButton.disabled = !legacyMigrationRequired
    || !currentSurfaceAccess().mutateStructure;
  if (legacyMigrationRequired) {
    const warning = document.createElement("div");
    warning.className = "trash-reconciliation";
    warning.setAttribute("role", "alert");
    warning.textContent = "检测到旧 Trash 格式。Server 必须先在已配置的工作区外位置创建精确快照；迁移后的条目为 origin unknown，不会猜测父节点或 owner。";
    elements.trashItemList.append(warning);
  }
  if (reconciliation?.required) {
    const warning = document.createElement("div");
    warning.className = "trash-reconciliation";
    warning.setAttribute("role", "alert");
    warning.textContent = `${reconciliation.issueCount} 项 Trash 证据不完整或冲突；仅可只读查看，恢复与删除均已暂停。`;
    elements.trashItemList.append(warning);
  }
  for (const item of state.trashInventory?.items ?? []) {
    const manifest = item.manifest;
    const bytes = manifest.payloadByteLength ?? manifest.byteLength;
    const button = document.createElement("button");
    button.className = "trash-item-row";
    button.setAttribute("role", "listitem");
    button.setAttribute("aria-label", `打开 Trash Item ${manifest.originalName}`);
    const origin = {
      active: "来源可用", in_trash: "来源也在 Trash", missing: "来源缺失",
      unknown: "来源未知", reconciliation_required: "需要协调",
    }[item.restore.originResolution];
    button.innerHTML = `<span aria-hidden="true">${manifest.kind === "node" ? "文" : "档"}</span><div><strong dir="auto">${escapeHtml(manifest.originalName)}</strong><small>${manifest.kind === "node" ? "节点子树" : "独立资源"} · ${bytes.toLocaleString()} 字节</small><small>${origin} · ${escapeHtml(new Date(manifest.trashedAt).toLocaleString())}</small></div>`;
    button.addEventListener("click", () => {
      state.selectedTrashItemId = manifest.trashItemId;
      elements.trashRestoreMode.value = item.restore.originalAvailable
        ? "original"
        : item.restore.withAncestorsAvailable ? "with_ancestors" : "existing_target";
      elements.trashRestoreOriginalOption.disabled = !item.restore.originalAvailable;
      elements.trashRestoreAncestorsOption.disabled = !item.restore.withAncestorsAvailable;
      elements.trashRestoreTarget.value = "";
      elements.trashRestoreName.value = manifest.originalName;
      elements.trashItemName.textContent = manifest.originalName;
      elements.trashItemEvidence.textContent = `${manifest.trashItemId} · ${manifest.operationId} · ${bytes.toLocaleString()} 字节 · ${origin}`;
      elements.trashItemPanel.hidden = false;
      const blocked = reconciliation?.required || item.restore.originResolution === "reconciliation_required";
      elements.trashRestorePreviewButton.disabled = blocked || !currentSurfaceAccess().mutateStructure;
      elements.trashPurgePreviewButton.disabled = blocked || !currentSurfaceAccess().permanentlyDelete;
    });
    elements.trashItemList.append(button);
  }
  if (!(state.trashInventory?.items ?? []).length && !reconciliation?.required && !legacyMigrationRequired) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.setAttribute("role", "status");
    empty.textContent = "Workspace Trash 为空。";
    elements.trashItemList.append(empty);
  }
  elements.trashCurrentNodePreviewButton.disabled = !currentSurfaceAccess().mutateStructure
    || !state.document || state.document.nodeId === state.workspace?.rootNodeId
    || reconciliation?.required || legacyMigrationRequired;
  elements.trashResourcesPreviewButton.disabled = !currentSurfaceAccess().mutateStructure
    || !state.document || reconciliation?.required || legacyMigrationRequired;
  renderTrashPlan();
}

async function loadTrashInventory() {
  if (!state.session || !state.workspace) return;
  const token = trashGate.begin();
  setTrashStatus("正在读取 permission-filtered Core Trash inventory…");
  try {
    const inventory = await serverApi.trashInventory();
    if (!trashGate.isCurrent(token)) return;
    state.trashInventory = inventory;
    if (!inventory.items.some((item) => item.manifest.trashItemId === state.selectedTrashItemId)) {
      state.selectedTrashItemId = null;
      elements.trashItemPanel.hidden = true;
    }
    renderTrashInventory();
    setTrashStatus(inventory.reconciliation.required
      ? "Trash reconciliation required；所有 mutation 已暂停。"
      : inventory.legacyMigrationRequired
        ? "旧 Trash 格式等待显式迁移；来源未知条目不会自动恢复。"
        : `${inventory.items.length} 个可见 Trash Items。Trash 是同步删除状态，不是备份。`, inventory.reconciliation.required);
  } catch (error) {
    if (!trashGate.isCurrent(token)) return;
    state.trashInventory = null;
    renderTrashInventory();
    setTrashStatus(error instanceof Error ? error.message : "Trash inventory 不可用", true);
  }
}

function dirtyBrowserDraftNodeIds() {
  const nodeIds = new Set(Object.keys(readDeviceDrafts()));
  if (hasDirtyDocument() && state.document?.nodeId) nodeIds.add(state.document.nodeId);
  return nodeIds;
}

function requireCleanTrashMutation(plan = null, allowLegacyMigration = false) {
  if (plan) {
    const dirtyNodeIds = dirtyBrowserDraftNodeIds();
    const conflicts = plan.draftSensitiveNodeIds.filter((nodeId) => dirtyNodeIds.has(nodeId));
    if (conflicts.length > 0) {
      throw new Error(`操作范围命中 ${conflicts.length} 个未提交的设备草稿；请提交或明确放弃后重新预览。`);
    }
  }
  if (state.trashInventory?.reconciliation.required) {
    throw new Error("Trash reconciliation required；当前只能只读查看诊断。");
  }
  if (state.trashInventory?.legacyMigrationRequired && !allowLegacyMigration) {
    throw new Error("旧 Trash 格式必须先完成外部快照支持的显式迁移；当前只能只读查看。");
  }
}

function stageTrashPlan(plan, purpose) {
  requireCleanTrashMutation(plan, purpose === "migration");
  state.trashPlan = plan;
  state.trashPlanPurpose = purpose;
  elements.trashPlanConfirm.checked = false;
  renderTrashPlan();
  setTrashStatus("Core 已固定单次、session-bound Trash 计划；尚未写入工作区。");
}

async function previewCurrentNodeTrash() {
  if (!state.document || !state.workspace || !currentSurfaceAccess().mutateStructure) return;
  try {
    requireCleanTrashMutation();
    stageTrashPlan(await serverApi.previewTrashNode(state.document.nodeId, {
      baseWorkspaceRevision: state.workspace.workspaceRevision,
      trashedAt: new Date().toISOString(),
      resolvedBy: "focused_pane",
    }), "node_trash");
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "节点 Trash 预览失败", true);
  }
}

async function previewResourceTrash() {
  if (!state.document || !state.workspace || !currentSurfaceAccess().mutateStructure) return;
  try {
    requireCleanTrashMutation();
    const names = elements.trashResourceNames.value.split(/\r?\n/u).map((name) => name.trim()).filter(Boolean);
    if (!names.length || new Set(names).size !== names.length) throw new Error("请输入一个或多个不重复的节点资源名，每行一个。");
    stageTrashPlan(await serverApi.previewTrashResources({
      baseWorkspaceRevision: state.workspace.workspaceRevision,
      trashedAt: new Date().toISOString(),
      resources: names.map((name) => ({ ownerNodeId: state.document.nodeId, name })),
      resolvedBy: "caller_explicit",
    }), "resource_trash");
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "资源 Trash 预览失败", true);
  }
}

async function previewLegacyTrashMigration() {
  if (!state.workspace || !state.trashInventory?.legacyMigrationRequired
    || !currentSurfaceAccess().mutateStructure) return;
  try {
    requireCleanTrashMutation(null, true);
    stageTrashPlan(await serverApi.previewTrashLegacyMigration({
      baseWorkspaceRevision: state.workspace.workspaceRevision,
      trashedAt: new Date().toISOString(),
    }), "migration");
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "旧 Trash 显式迁移预览失败", true);
  }
}

async function previewTrashRestore() {
  const item = selectedTrashItem();
  if (!item || !state.workspace || !currentSurfaceAccess().mutateStructure) return;
  try {
    requireCleanTrashMutation();
    const mode = elements.trashRestoreMode.value;
    const existing = mode === "existing_target";
    if (existing && (!elements.trashRestoreTarget.value || !elements.trashRestoreName.value.trim())) {
      throw new Error("来源未知或目标冲突时，必须显式选择现有目标并确认名称。");
    }
    stageTrashPlan(await serverApi.previewTrashRestore(item.manifest.trashItemId, {
      baseWorkspaceRevision: state.workspace.workspaceRevision,
      mode,
      resolvedBy: "explicit_row",
      ...(existing ? { targetNodeId: elements.trashRestoreTarget.value, name: elements.trashRestoreName.value.trim() } : {}),
    }), "restore");
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "Trash 恢复预览失败", true);
  }
}

async function previewTrashPermanentDelete() {
  const item = selectedTrashItem();
  if (!item || !state.workspace || !currentSurfaceAccess().permanentlyDelete) return;
  try {
    requireCleanTrashMutation();
    stageTrashPlan(await serverApi.previewTrashPermanentDelete({
      baseWorkspaceRevision: state.workspace.workspaceRevision,
      resolvedBy: "explicit_row",
      items: trashPermanentDeleteConfirmation([item]),
    }), "permanent_delete");
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "永久删除预览失败", true);
  }
}

async function commitTrashPlan() {
  if (!state.trashPlan || !elements.trashPlanConfirm.checked) return;
  try {
    requireCleanTrashMutation(state.trashPlan, state.trashPlanPurpose === "migration");
  } catch (error) {
    state.trashPlan = null;
    state.trashPlanPurpose = null;
    elements.trashPlanConfirm.checked = false;
    setTrashStatus(error.message, true);
    renderTrashPlan();
    return;
  }
  const plan = state.trashPlan;
  state.trashPlan = null;
  state.trashPlanPurpose = null;
  elements.trashPlanConfirm.checked = false;
  renderTrashPlan();
  try {
    const result = await serverApi.commitTrash(plan.planId);
    await refreshWorkspace();
    if (state.document && !state.workspace.nodes.some((node) => node.id === state.document.nodeId)) {
      state.document = null;
      await openNode(state.workspace.rootNodeId, "trash-commit");
    } else if (state.document) {
      applyDocumentPayload(await serverApi.openDocument(state.document.nodeId), { restoreDraft: false });
    }
    await loadTrashInventory();
    setTrashStatus(result.auditRecorded === false
      ? "Trash 事务已提交，但审计 receipt 未确认；请联系管理员。"
      : "Trash 事务已提交并从 Core authoritative inventory 刷新。", result.auditRecorded === false);
  } catch (error) {
    setTrashStatus(error instanceof Error ? error.message : "Trash 提交结果不确定", true);
  }
}

async function previewDraft() {
  if (!state.document || !currentSurfaceAccess().editDocuments) return;
  const context = captureMutationContext(state.document, elements.editor.value);
  const requestToken = mutationGate.begin();
  try {
    const result = await serverApi.preview(
      context.nodeId,
      context.revision,
      context.source,
    );
    if (!mutationGate.isCurrent(requestToken) || !mutationContextMatches(context, state.document, elements.editor.value)) return;
    state.preview = result.model;
    renderPreview(result.model);
    elements.previewSummary.textContent = `${result.oldLength} → ${result.newLength} 字节 · ${result.changed ? "有更改" : "无更改"}`;
    setStatus("Core 预览已生成；工作区尚未写入");
  } catch (error) {
    if (mutationGate.isCurrent(requestToken) && mutationContextMatches(context, state.document, elements.editor.value)) showFailure(error);
  }
}

async function commitDraft() {
  if (!state.document || !currentSurfaceAccess().editDocuments) return;
  const context = captureMutationContext(state.document, elements.editor.value);
  const requestToken = mutationGate.begin();
  try {
    if (!elements.collaborationStatus) {
      const result = await serverApi.commit(context.nodeId, context.revision, context.source);
      removeDeviceDraft(context.nodeId, (draft) => draft.baseRevision === context.revision && draft.source === context.source);
      if (!mutationGate.isCurrent(requestToken) || !mutationContextMatches(context, state.document, elements.editor.value)) return;
      state.document.revision = result.revision;
      state.document.source = context.source;
      elements.revision.textContent = result.revision;
      setStatus(result.changed ? "已通过 Core revision 检查提交" : "文档没有变化");
      await refreshWorkspace();
      return;
    }
    if (!state.collaboration || state.collaboration.nodeId !== context.nodeId) {
      await refreshCollaboration(context.nodeId);
    }
    if (!state.collaboration || state.collaboration.nodeId !== context.nodeId) {
      throw new Error("未取得当前文档的实时协作快照");
    }
    const operationId = createCollaborationClientId();
    const result = await serverApi.commitCollaborationDraft(context.nodeId, {
      wireVersion: COLLABORATION_WIRE_VERSION,
      clientId: state.collaborationClientId,
      operationId,
      epoch: state.collaboration.state.epoch,
      baseVersion: state.collaboration.state.version,
      baseRevision: context.revision,
      source: context.source,
    });
    if (!mutationGate.isCurrent(requestToken) || !mutationContextMatches(context, state.document, elements.editor.value)) return;
    state.collaboration.state = result.state;
    renderCollaboration();
    if (!["accepted", "replayed"].includes(result.status)) {
      saveDeviceDraft(createDeviceDraft(state.document, context.source, currentWorkspaceScope(), currentActorScope()));
      setStatus(`协作提交未覆盖服务器内容：${result.errorCode ?? result.status}`, "conflict");
      return;
    }
    removeDeviceDraft(context.nodeId, (draft) => draft.baseRevision === context.revision && draft.source === context.source);
    const canonical = await serverApi.openDocument(context.nodeId);
    if (!mutationGate.isCurrent(requestToken) || state.document?.nodeId !== context.nodeId) return;
    applyDocumentPayload(canonical, { restoreDraft: false });
    setStatus(result.status === "replayed" ? "协作操作已安全重放" : "已通过 Server 线性化协议与 Core 提交");
    await refreshWorkspace();
  } catch (error) {
    if (mutationGate.isCurrent(requestToken) && mutationContextMatches(context, state.document, elements.editor.value)) showFailure(error);
  }
}

async function runSearch(event) {
  event.preventDefault();
  const query = elements.searchInput.value.trim();
  if (!query) return;
  try {
    const payload = await serverApi.search(query);
    elements.searchResults.replaceChildren();
    for (const result of payload.results) {
      const button = document.createElement("button");
      button.innerHTML = `<strong>${nodeIcon(result.icon)}${escapeHtml(result.name)}</strong><span>${escapeHtml(result.snippet)}</span>`;
      button.addEventListener("click", () => void openNode(result.id, "search"));
      elements.searchResults.append(button);
    }
    if (!payload.results.length) elements.searchResults.textContent = "没有结果";
  } catch (error) {
    showFailure(error);
  }
}

async function reconcileCurrentRevision(reason) {
  if (!state.document) return;
  const context = captureMutationContext(state.document, elements.editor.value);
  try {
    const latest = await serverApi.openDocument(context.nodeId);
    if (!mutationContextMatches(context, state.document, elements.editor.value)) return;
    if (latest.revision === context.revision) {
      if (reason === "reconnected" || reason === "lagged") setStatus("已重新核对当前文档 revision");
      return;
    }
    if (context.source === state.document.source) {
      applyDocumentPayload(latest, { restoreDraft: false });
      setStatus("已在变更订阅重同步后载入新 revision");
    } else {
      saveDeviceDraft(createDeviceDraft(state.document, context.source, currentWorkspaceScope(), currentActorScope()));
      setStatus("服务器 revision 已变化；设备草稿已保留，请先解决冲突", "conflict");
    }
  } catch (error) {
    if (mutationContextMatches(context, state.document, elements.editor.value)) showFailure(error);
  }
}

function showFailure(error) {
  if (error instanceof ApiFailure && error.code === "authentication_required") {
    showAuth(false, "会话已失效，请重新登录");
    return;
  }
  if (error instanceof ApiFailure && ["stale_revision", "stale_workspace_revision"].includes(error.code)) {
    setStatus(`冲突：服务器当前 revision ${error.conflict?.actualRevision ?? "未知"}`, "conflict");
    return;
  }
  if (error instanceof ApiFailure && error.code === "authorization_denied") {
    setStatus("当前角色或继承节点 ACL 拒绝此操作；Server 未披露受限内容", "error");
    return;
  }
  setStatus(error instanceof Error ? error.message : "请求失败", "error");
}

function clearProtectedView() {
  state.unsubscribe?.();
  state.collaborationUnsubscribe?.();
  state.unsubscribe = null;
  state.collaborationUnsubscribe = null;
  state.collaboration = null;
  state.collaborationNotice = null;
  state.workspace = null;
  state.document = null;
  state.preview = null;
  state.backupPlan = null;
  state.restorePlan = null;
  state.restorePurpose = null;
  state.trashInventory = null;
  state.selectedTrashItemId = null;
  state.trashPlan = null;
  state.trashPlanPurpose = null;
  state.navigation = defaultNavigationState();
  navigationGate.invalidate();
  mutationGate.invalidate();
  resetNodeSurfaces();
  elements.hierarchyView.replaceChildren();
  elements.contentList.replaceChildren();
  elements.searchResults.replaceChildren();
  elements.preview.replaceChildren();
  elements.nodeMetadataValues?.replaceChildren();
  elements.nodeMetadataAliases?.replaceChildren();
  elements.nodeMetadataDiagnostics?.replaceChildren();
  elements.documentPropertyValues?.replaceChildren();
  elements.memberList?.replaceChildren();
  elements.nodeAclList?.replaceChildren();
  elements.trashItemList?.replaceChildren();
  if (elements.trashItemPanel) elements.trashItemPanel.hidden = true;
  if (elements.trashPlanPanel) elements.trashPlanPanel.hidden = true;
  if (elements.permissionsPanel) elements.permissionsPanel.hidden = true;
  if (elements.backupPanel) elements.backupPanel.hidden = true;
  elements.editor.value = "";
  elements.title.textContent = "未登录";
  elements.revision.textContent = "";
  renderCollaboration();
}

function showAuth(bootstrapMode, message = "请验证本地成员身份") {
  clearProtectedView();
  state.session = null;
  elements.workspaceView.hidden = true;
  elements.authPanel.hidden = false;
  elements.bootstrapField.hidden = !bootstrapMode;
  elements.loginField.hidden = bootstrapMode;
  elements.authTitle.textContent = bootstrapMode ? "初始化本机 Owner" : "成员登录";
  elements.authSubmit.textContent = bootstrapMode ? "一次性初始化并登录" : "登录";
  elements.authModeButton.textContent = bootstrapMode ? "返回 Owner 登录" : "首次使用？切换到 Owner 初始化";
  elements.authStatus.textContent = message;
  elements.sessionLabel.textContent = "未登录";
  elements.logoutButton.hidden = true;
}

async function enterSession(session) {
  applySessionCapabilities(session);
  if (elements.collaborationStatus) state.collaborationClientId ??= createCollaborationClientId();
  elements.authPanel.hidden = true;
  elements.workspaceView.hidden = false;
  elements.logoutButton.hidden = false;
  await refreshWorkspace();
  state.unsubscribe?.();
  state.unsubscribe = serverApi.subscribe({
    onChange: (change) => {
      if (change.nodeId === state.document?.nodeId && change.revision !== state.document.revision) {
        void reconcileCurrentRevision("change");
      }
    },
    onResync: ({ reason }) => void reconcileCurrentRevision(reason),
    onDisconnect: () => void verifySubscriptionSession(),
  });
  state.collaborationUnsubscribe?.();
  if (elements.collaborationStatus) state.collaborationUnsubscribe = serverApi.subscribeCollaboration({
    onEvent: (event) => {
      if (event.nodeId !== state.document?.nodeId) return;
      if (state.collaboration) {
        if (event.eventType !== "presence") {
          state.collaboration.state = {
            wireVersion: event.wireVersion,
            epoch: event.epoch,
            version: event.version,
            revision: event.revision,
            frozen: Boolean(event.reason) || ["conflict", "external-edit"].includes(event.eventType),
            ...(event.reason ? { reason: event.reason } : {}),
          };
        }
        if (event.participants) state.collaboration.participants = event.participants;
        renderCollaboration();
      }
      if (["operation-committed", "external-edit", "conflict", "resynced"].includes(event.eventType)) {
        void refreshCollaboration(event.nodeId);
      }
    },
    onResync: () => {
      state.collaboration = null;
      state.collaborationNotice = "实时协作事件滞后；正在 resync authoritative 快照";
      renderCollaboration();
      void refreshCollaboration();
    },
    onDisconnect: () => {
      state.collaboration = null;
      state.collaborationNotice = "实时协作连接已断开；正在验证会话并等待 resync";
      renderCollaboration();
      void verifySubscriptionSession();
    },
  });
}

function renderMemberRoleChoices() {
  const roles = state.session?.role === "owner"
    ? ["owner", "admin", "editor", "commenter", "viewer"]
    : ["admin", "editor", "commenter", "viewer"];
  const previous = elements.memberRole.value;
  elements.memberRole.replaceChildren();
  for (const role of roles) {
    const option = document.createElement("option");
    option.value = role;
    option.textContent = role[0].toUpperCase() + role.slice(1);
    option.selected = role === previous;
    elements.memberRole.append(option);
  }
}

function applySessionCapabilities(session) {
  state.session = requireSessionCapabilities(session, state.serverCapabilities?.roleCapabilities ?? null);
  const access = currentSurfaceAccess();
  elements.sessionLabel.textContent = session.role[0].toUpperCase() + session.role.slice(1);
  elements.editor.readOnly = !access.editDocuments;
  elements.previewButton.disabled = !access.editDocuments;
  elements.commitButton.disabled = !access.editDocuments;
  elements.annotationCreateButton.disabled = !access.writeAnnotations;
  elements.queryRunButton.disabled = !access.executeQueries;
  renderMemberRoleChoices();
  if (state.annotations) renderAnnotations();
  else setAnnotationComposerEnabled(false);
  if (state.tasks) renderTasks();
  if (state.trashInventory) renderTrashInventory();
}

function renderBackupPlans() {
  elements.backupPlanPanel.hidden = !state.backupPlan;
  elements.backupPlanOutput.textContent = state.backupPlan
    ? JSON.stringify(state.backupPlan, null, 2)
    : "";
  elements.backupCommitButton.disabled = !state.backupPlan || !elements.backupPlanConfirm.checked;
  elements.restorePlanPanel.hidden = !state.restorePlan;
  elements.restorePlanOutput.textContent = state.restorePlan
    ? JSON.stringify(state.restorePlan, null, 2)
    : "";
  const confirmed = Boolean(state.restorePlan && elements.restorePlanConfirm.checked);
  elements.restoreCommitButton.hidden = state.restorePurpose !== "restore";
  elements.drillCommitButton.hidden = state.restorePurpose !== "drill";
  elements.restoreCommitButton.disabled = !confirmed || state.restorePurpose !== "restore";
  elements.drillCommitButton.disabled = !confirmed || state.restorePurpose !== "drill";
}

function setBackupStatus(message, error = false) {
  elements.backupStatus.textContent = message;
  elements.backupStatus.classList?.toggle("surface-error", error);
}

function requireSavedBrowserSourceSet() {
  if (hasDirtyDocument() || Object.keys(readDeviceDrafts()).length > 0) {
    throw new Error("当前浏览器仍有设备草稿；请先提交或明确放弃后再预览 Server 备份");
  }
}

async function loadBackupCapabilities() {
  elements.backupPanel.hidden = !currentSurfaceAccess().manageWorkspace;
  if (elements.backupPanel.hidden) return;
  try {
    const capability = await serverApi.backupCapabilities();
    setBackupStatus(
      capability.apiQuiescence
        ? "Owner 控制已就绪：workspace + control plane 成对、排他 lease、API 静默、create-new 恢复。"
        : "备份安全能力不完整",
      !capability.apiQuiescence,
    );
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "无法读取备份能力", true);
  }
}

async function previewServerBackup(event) {
  event.preventDefault();
  if (!currentSurfaceAccess().manageWorkspace) return;
  try {
    requireSavedBrowserSourceSet();
    const result = await serverApi.previewServerBackup(elements.backupParent.value.trim());
    state.backupPlan = result.plan;
    elements.backupPlanConfirm.checked = false;
    renderBackupPlans();
    setBackupStatus("备份预览已固定；提交只接受此 planDigest。请核对两组目标与总字节数。");
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "备份预览失败", true);
  }
}

async function commitServerBackup() {
  if (!state.backupPlan || !elements.backupPlanConfirm.checked) return;
  elements.backupCommitButton.disabled = true;
  try {
    requireSavedBrowserSourceSet();
    const result = await serverApi.commitServerBackup(state.backupPlan.planDigest);
    elements.backupWorkspaceSnapshot.value = result.receipt.workspaceSnapshotDirectory;
    elements.backupControlSnapshot.value = result.receipt.controlPlaneSnapshotDirectory;
    state.backupPlan = null;
    elements.backupPlanConfirm.checked = false;
    renderBackupPlans();
    setBackupStatus("成对备份已提交、重开校验并写入 Owner 审计 receipt。");
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "备份提交失败", true);
    renderBackupPlans();
  }
}

function snapshotPairFields() {
  return {
    workspaceSnapshotDirectory: elements.backupWorkspaceSnapshot.value.trim(),
    controlPlaneSnapshotDirectory: elements.backupControlSnapshot.value.trim(),
  };
}

function restoreTargetFields() {
  return {
    restoredWorkspaceRoot: elements.restoredWorkspaceRoot.value.trim(),
    restoredControlPlaneRoot: elements.restoredControlRoot.value.trim(),
  };
}

async function verifyServerBackupPair(event) {
  event.preventDefault();
  try {
    const pair = snapshotPairFields();
    const result = await serverApi.verifyServerBackup(
      pair.workspaceSnapshotDirectory,
      pair.controlPlaneSnapshotDirectory,
    );
    setBackupStatus(result.verification.exactPair
      ? "workspace + control plane 快照精确配对校验通过。"
      : "快照未形成精确对", !result.verification.exactPair);
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "成对校验失败", true);
  }
}

async function previewServerRestore(purpose) {
  if (!currentSurfaceAccess().manageWorkspace) return;
  try {
    const pair = snapshotPairFields();
    const targets = restoreTargetFields();
    const result = purpose === "drill"
      ? await serverApi.previewServerRestoreDrill({
        ...pair,
        drillWorkspaceRoot: targets.restoredWorkspaceRoot,
        drillControlPlaneRoot: targets.restoredControlPlaneRoot,
      })
      : await serverApi.previewServerRestore({ ...pair, ...targets });
    state.restorePlan = result.plan;
    state.restorePurpose = purpose;
    elements.restorePlanConfirm.checked = false;
    renderBackupPlans();
    setBackupStatus(purpose === "drill"
      ? "恢复演练预览已固定；成功后保留 clean-instance 精确对并记录审计 receipt。"
      : "alternate restore 预览已固定；目标必须保持不存在且彼此不相交。");
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "恢复预览失败", true);
  }
}

async function commitServerRestore(purpose) {
  if (!state.restorePlan || state.restorePurpose !== purpose || !elements.restorePlanConfirm.checked) return;
  renderBackupPlans();
  try {
    const result = purpose === "drill"
      ? await serverApi.commitServerRestoreDrill(state.restorePlan.planDigest)
      : await serverApi.commitServerRestore(state.restorePlan.planDigest);
    state.restorePlan = null;
    state.restorePurpose = null;
    elements.restorePlanConfirm.checked = false;
    renderBackupPlans();
    setBackupStatus(purpose === "drill"
      ? "恢复演练已完成：clean open、bytewise exact pair 与审计 receipt 均已确认。"
      : "alternate restore 已完成：两个 create-new 目标均已重开并精确校验。");
    if (!result.auditRecorded) setBackupStatus("恢复完成但审计 receipt 未确认", true);
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "恢复提交失败", true);
    renderBackupPlans();
  }
}

async function verifyServerRestoreTargets() {
  try {
    const result = await serverApi.verifyServerRestore({
      ...snapshotPairFields(),
      ...restoreTargetFields(),
    });
    setBackupStatus(result.verification.exactPair
      ? "已恢复 workspace + control plane 精确对校验通过。"
      : "已恢复目标未形成精确对", !result.verification.exactPair);
  } catch (error) {
    setBackupStatus(error instanceof Error ? error.message : "恢复目标校验失败", true);
  }
}

async function loadPermissions() {
  const role = state.session?.role ?? "viewer";
  const canManage = currentSurfaceAccess().manageMembers;
  elements.permissionSummary.textContent = `当前角色：${role}。${canManage ? "可管理成员与节点授权。" : "此处为只读能力摘要；成员与 ACL 数据不向当前角色披露。"}`;
  elements.memberForm.hidden = !canManage;
  elements.nodeAclForm.hidden = !canManage || !state.document?.nodeId;
  elements.memberList.replaceChildren();
  elements.nodeAclList.replaceChildren();
  await loadBackupCapabilities();
  if (!canManage) return;
  try {
    const [members, nodeAcl] = await Promise.all([serverApi.members(), serverApi.nodeAcl()]);
    elements.nodeAclMember.replaceChildren();
    for (const member of members) {
      const memberOption = document.createElement("option");
      memberOption.value = member.actorScope;
      memberOption.textContent = `${member.login} (${member.role})`;
      elements.nodeAclMember.append(memberOption);
      const row = document.createElement("div");
      row.setAttribute("role", "listitem");
      const label = document.createElement("span");
      label.textContent = member.login;
      const roleSelect = document.createElement("select");
      roleSelect.setAttribute("aria-label", `${member.login} 角色`);
      for (const roleName of ["owner", "admin", "editor", "commenter", "viewer"]) {
        const option = document.createElement("option");
        option.value = roleName;
        option.textContent = roleName;
        option.selected = member.role === roleName;
        option.disabled = state.session.role !== "owner" && roleName === "owner";
        roleSelect.append(option);
      }
      const enabled = document.createElement("input");
      enabled.type = "checkbox";
      enabled.checked = member.enabled;
      enabled.setAttribute("aria-label", `${member.login} 已启用`);
      const save = document.createElement("button");
      save.textContent = "保存";
      const protectedOwner = state.session.role !== "owner" && member.role === "owner";
      roleSelect.disabled = protectedOwner;
      enabled.disabled = protectedOwner;
      save.disabled = protectedOwner;
      save.addEventListener("click", async () => {
        if (protectedOwner) return;
        save.disabled = true;
        try {
          await serverApi.updateMember(member.actorScope, roleSelect.value, enabled.checked);
          elements.permissionSummary.textContent = `已更新 ${member.login}`;
        } catch (error) {
          elements.permissionSummary.textContent = error instanceof Error ? error.message : "成员更新失败";
        } finally {
          save.disabled = false;
        }
      });
      row.append(label, roleSelect, enabled, save);
      elements.memberList.append(row);
    }
    for (const entry of nodeAcl.filter((entry) => entry.nodeId === state.document?.nodeId)) {
      const member = members.find((candidate) => candidate.actorScope === entry.actorScope);
      const row = document.createElement("div");
      row.setAttribute("role", "listitem");
      row.textContent = `${member?.login ?? entry.actorScope}: ${entry.access}`;
      elements.nodeAclList.append(row);
    }
  } catch (error) {
    elements.permissionSummary.textContent = error instanceof Error ? error.message : "权限读取失败";
  }
}

async function setCurrentNodeAcl(event) {
  event.preventDefault();
  if (!state.document?.nodeId || !currentSurfaceAccess().manageMembers) return;
  try {
    await serverApi.setNodeAcl(
      elements.nodeAclMember.value,
      state.document.nodeId,
      elements.nodeAclAccess.value || null,
    );
    await loadPermissions();
  } catch (error) {
    elements.permissionSummary.textContent = error instanceof Error ? error.message : "节点授权更新失败";
  }
}

async function createMemberFromForm(event) {
  event.preventDefault();
  if (!currentSurfaceAccess().manageMembers) return;
  if (state.session.role !== "owner" && elements.memberRole.value === "owner") return;
  try {
    await serverApi.createMember(
      elements.memberLogin.value.trim(),
      elements.memberPassword.value,
      elements.memberRole.value,
    );
    elements.memberPassword.value = "";
    await loadPermissions();
  } catch (error) {
    elements.permissionSummary.textContent = error instanceof Error ? error.message : "成员创建失败";
  }
}

async function verifySubscriptionSession() {
  if (!state.session || state.checkingSubscriptionSession) return;
  state.checkingSubscriptionSession = true;
  try {
    applySessionCapabilities(await serverApi.session());
    if (state.inspectorTab === "permissions") await loadPermissions();
  } catch (error) {
    showFailure(error);
  } finally {
    state.checkingSubscriptionSession = false;
  }
}

async function submitAuthentication(event) {
  event.preventDefault();
  elements.authStatus.textContent = "正在验证…";
  try {
    const session = elements.bootstrapField.hidden
      ? await serverApi.login(elements.loginInput.value.trim(), elements.passwordInput.value)
      : await serverApi.bootstrap(elements.bootstrapInput.value.trim(), elements.passwordInput.value);
    elements.passwordInput.value = "";
    elements.bootstrapInput.value = "";
    await enterSession(session);
  } catch (error) {
    elements.authStatus.textContent = error instanceof Error ? error.message : "身份验证失败";
  }
}

async function logout() {
  if (state.document) {
    const draft = createDeviceDraft(
      state.document,
      elements.editor.value,
      currentWorkspaceScope(),
      currentActorScope(),
    );
    if (draft && !saveDeviceDraft(draft)) {
      setStatus("设备草稿保存失败，已阻止注销", "error");
      return;
    }
  }
  try {
    await serverApi.logout();
  } catch (error) {
    if (!(error instanceof ApiFailure && error.code === "authentication_required")) {
      showFailure(error);
      return;
    }
  }
  showAuth(false, "已注销；设备草稿仍按原 Owner 隔离保存");
}

async function start() {
  for (const id of [
    "status", "contentList", "title", "revision", "editor", "preview", "previewSummary",
    "searchInput", "searchResults", "workspaceView", "authPanel", "authTitle", "authStatus",
    "bootstrapField", "bootstrapInput", "loginField", "loginInput", "passwordInput", "authSubmit",
    "authModeButton", "sessionLabel", "logoutButton", "explorerActivity", "searchActivity", "chronoActivity", "trashActivity",
    "navigationPanel", "explorerPanel", "searchPanel", "chronoPanel", "trashPanel", "hierarchyMode", "contentsMode",
    "navigationFilter", "explorerScroll", "hierarchyView", "contentsView", "contentsLocation",
    "contentsFollowing", "returnToNode", "contentsBreadcrumbs", "narrowNavigation", "widenNavigation",
    "navigationWidth", "navigationMetrics", "previewButton", "commitButton", "permissionsButton",
    "permissionsPanel", "permissionSummary", "memberForm", "memberLogin", "memberPassword",
    "memberRole", "memberList", "nodeAclForm", "nodeAclMember", "nodeAclAccess", "nodeAclList",
    "nodeMetadataValues", "nodeMetadataAliases", "nodeMetadataDiagnostics", "documentPropertyValues",
    "inspectorTabs", "propertiesTab", "annotationsTab", "tasksTab", "queryTab", "propertiesPanel",
    "annotationsPanel", "annotationStatus", "annotationCreateForm", "annotationKind", "annotationTarget",
    "annotationBody", "annotationSuggestedSource", "annotationLabels", "annotationMark", "annotationTheme",
    "annotationCreateButton", "annotationList", "tasksPanel", "taskStatus", "taskList", "taskPlanPanel",
    "taskPlanId", "taskPlanIntent", "taskPlanGenerated", "taskPlanSource", "taskPlanConfirm",
    "taskPlanChanges", "taskPlanCancelButton", "taskPlanCommitButton", "queryPanel",
    "queryForm", "querySource", "queryBlockIndex", "queryToday", "queryRunButton", "queryStatus",
    "queryDiagnostics", "queryGroups", "queryTableContainer", "queryCsvButton",
    "collaborationStatus", "collaborationState", "collaborationRevision", "collaborationParticipants",
    "backupPanel", "backupStatus", "backupForm", "backupParent", "backupPreviewButton",
    "backupPlanPanel", "backupPlanOutput", "backupPlanConfirm", "backupCommitButton",
    "backupVerifyForm", "backupWorkspaceSnapshot", "backupControlSnapshot", "backupVerifyButton",
    "restoreForm", "restoredWorkspaceRoot", "restoredControlRoot", "restorePreviewButton",
    "drillPreviewButton", "restoreVerifyButton", "restorePlanPanel", "restorePlanOutput",
    "restorePlanConfirm", "restoreCommitButton", "drillCommitButton",
    "trashRefreshButton", "trashStatus", "trashMigrationPreviewButton", "trashCurrentNodePreviewButton", "trashResourceNames",
    "trashResourcesPreviewButton", "trashItemList", "trashItemPanel", "trashItemName",
    "trashItemEvidence", "trashRestoreMode", "trashRestoreOriginalOption", "trashRestoreAncestorsOption", "trashRestoreTarget", "trashRestoreName",
    "trashRestorePreviewButton", "trashPurgePreviewButton", "trashPlanPanel", "trashPlanOutput",
    "trashPlanConfirm", "trashPlanConfirmationText", "trashPlanCancelButton", "trashPlanCommitButton",
  ]) {
    elements[id] = document.getElementById(id);
  }
  elements.previewButton.addEventListener("click", () => void previewDraft());
  elements.commitButton.addEventListener("click", () => void commitDraft());
  elements.editor.addEventListener("select", () => void publishCollaborationPresence());
  document.getElementById("searchForm").addEventListener("submit", (event) => void runSearch(event));
  document.getElementById("authForm").addEventListener("submit", (event) => void submitAuthentication(event));
  elements.authModeButton.addEventListener("click", () => showAuth(elements.bootstrapField.hidden));
  elements.logoutButton.addEventListener("click", () => void logout());
  elements.propertiesTab.addEventListener("click", () => chooseInspectorTab("properties"));
  elements.annotationsTab.addEventListener("click", () => chooseInspectorTab("annotations"));
  elements.tasksTab.addEventListener("click", () => chooseInspectorTab("tasks"));
  elements.queryTab.addEventListener("click", () => chooseInspectorTab("query"));
  elements.permissionsButton.addEventListener("click", () => chooseInspectorTab("permissions"));
  elements.inspectorTabs.addEventListener("keydown", handleInspectorTabKey);
  elements.memberForm.addEventListener("submit", (event) => void createMemberFromForm(event));
  elements.nodeAclForm.addEventListener("submit", (event) => void setCurrentNodeAcl(event));
  elements.backupForm.addEventListener("submit", (event) => void previewServerBackup(event));
  elements.backupPlanConfirm.addEventListener("change", renderBackupPlans);
  elements.backupCommitButton.addEventListener("click", () => void commitServerBackup());
  elements.backupVerifyForm.addEventListener("submit", (event) => void verifyServerBackupPair(event));
  elements.restorePreviewButton.addEventListener("click", () => void previewServerRestore("restore"));
  elements.drillPreviewButton.addEventListener("click", () => void previewServerRestore("drill"));
  elements.restorePlanConfirm.addEventListener("change", renderBackupPlans);
  elements.restoreCommitButton.addEventListener("click", () => void commitServerRestore("restore"));
  elements.drillCommitButton.addEventListener("click", () => void commitServerRestore("drill"));
  elements.restoreVerifyButton.addEventListener("click", () => void verifyServerRestoreTargets());
  elements.annotationCreateForm.addEventListener("submit", submitAnnotationCreate);
  elements.annotationKind.addEventListener("change", () => setAnnotationComposerEnabled(annotationMutationEnabled()));
  elements.annotationBody.addEventListener("keydown", (event) => {
    if (event.isComposing || event.keyCode === 229) return;
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") submitAnnotationCreate(event);
  });
  elements.taskPlanCommitButton.addEventListener("click", () => void commitTaskPlan());
  elements.taskPlanConfirm.addEventListener("change", () => {
    state.taskPlanConfirmed = Boolean(elements.taskPlanConfirm.checked);
    renderTaskPlan();
  });
  elements.taskPlanCancelButton.addEventListener("click", () => {
    if (state.taskPlanPending) return;
    state.taskPlan = null;
    state.taskPlanConfirmed = false;
    renderTaskPlan();
    elements.taskStatus.textContent = "任务事务预览已取消。";
  });
  elements.queryForm.addEventListener("submit", (event) => void runQuery(event));
  elements.querySource.addEventListener("keydown", (event) => {
    if (event.isComposing || event.keyCode === 229) return;
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") void runQuery(event);
  });
  elements.queryCsvButton.addEventListener("click", downloadQueryCsv);
  elements.explorerActivity.addEventListener("click", () => chooseActivity("explorer"));
  elements.searchActivity.addEventListener("click", () => chooseActivity("search"));
  elements.chronoActivity.addEventListener("click", () => chooseActivity("chrono"));
  elements.trashActivity.addEventListener("click", () => chooseActivity("trash"));
  elements.trashRefreshButton.addEventListener("click", () => void loadTrashInventory());
  elements.trashMigrationPreviewButton.addEventListener("click", () => void previewLegacyTrashMigration());
  elements.trashCurrentNodePreviewButton.addEventListener("click", () => void previewCurrentNodeTrash());
  elements.trashResourcesPreviewButton.addEventListener("click", () => void previewResourceTrash());
  elements.trashRestorePreviewButton.addEventListener("click", () => void previewTrashRestore());
  elements.trashPurgePreviewButton.addEventListener("click", () => void previewTrashPermanentDelete());
  elements.trashPlanConfirm.addEventListener("change", renderTrashPlan);
  elements.trashPlanCancelButton.addEventListener("click", () => {
    state.trashPlan = null;
    state.trashPlanPurpose = null;
    elements.trashPlanConfirm.checked = false;
    renderTrashPlan();
    setTrashStatus("Trash 事务预览已取消；工作区未写入。");
  });
  elements.trashPlanCommitButton.addEventListener("click", () => void commitTrashPlan());
  elements.hierarchyMode.addEventListener("click", () => chooseExplorerMode("hierarchy"));
  elements.contentsMode.addEventListener("click", () => chooseExplorerMode("contents"));
  elements.navigationFilter.addEventListener("input", () => {
    state.navigation.filter = elements.navigationFilter.value;
    state.navigation.hierarchyLimit = INITIAL_NAVIGATION_WINDOW;
    state.navigation.contentsLimit = INITIAL_NAVIGATION_WINDOW;
    renderNavigation();
    persistNavigation();
  });
  elements.explorerScroll.addEventListener("scroll", () => {
    state.navigation.scrollTop = elements.explorerScroll.scrollTop;
    persistNavigation();
  });
  elements.returnToNode.addEventListener("click", () => {
    state.navigation.browseLocator = null;
    renderContents();
    persistNavigation();
    elements.explorerScroll.focus();
  });
  elements.narrowNavigation.addEventListener("click", () => {
    state.navigation.width = Math.max(220, state.navigation.width - 24);
    renderNavigation();
    persistNavigation();
  });
  elements.widenNavigation.addEventListener("click", () => {
    state.navigation.width = Math.min(480, state.navigation.width + 24);
    renderNavigation();
    persistNavigation();
  });
  window.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      chooseActivity("search");
    } else if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "e") {
      event.preventDefault();
      chooseExplorerMode(state.navigation.mode === "hierarchy" ? "contents" : "hierarchy");
    } else if (event.altKey && event.key === "Home") {
      event.preventDefault();
      state.navigation.browseLocator = null;
      chooseExplorerMode("contents");
    }
  });
  elements.editor.addEventListener("input", () => {
    if (!state.document) return;
    const draft = createDeviceDraft(state.document, elements.editor.value, currentWorkspaceScope(), currentActorScope());
    if (draft) {
      const saved = saveDeviceDraft(draft);
      setStatus(saved ? "设备草稿已保存；尚未提交" : "设备草稿保存失败；替换文档的导航将取消", saved ? "draft" : "error");
    } else {
      removeDeviceDraft(state.document.nodeId, (saved) => saved.baseRevision === state.document.revision);
      setStatus("当前内容与服务器 revision 一致");
    }
    if (state.inspectorTab === "annotations" && state.annotations) renderAnnotations();
    if (state.inspectorTab === "tasks" && state.tasks) renderTasks();
    if (state.navigation.activity === "trash") renderTrashInventory();
  });
  const localToday = new Date();
  elements.queryToday.value = [
    localToday.getFullYear(),
    String(localToday.getMonth() + 1).padStart(2, "0"),
    String(localToday.getDate()).padStart(2, "0"),
  ].join("-");
  chooseInspectorTab("properties");
  try {
    const [health, capabilities] = await Promise.all([serverApi.health(), serverApi.capabilities()]);
    if (!capabilities.loopbackOnly || capabilities.deploymentReady) throw new Error("服务器安全能力声明异常");
    requireRoleCapabilityMap(capabilities.roleCapabilities);
    state.serverCapabilities = capabilities;
    document.getElementById("serverStage").textContent = health.stage;
    try {
      await enterSession(await serverApi.session());
    } catch (error) {
      if (error instanceof ApiFailure && error.code === "authentication_required") {
        showAuth(false);
      } else {
        throw error;
      }
    }
  } catch (error) {
    showFailure(error);
  }
}

if (typeof document !== "undefined") void start();
