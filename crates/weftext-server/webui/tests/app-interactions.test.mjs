import assert from "node:assert/strict";
import test from "node:test";

class FakeElement {
  constructor() {
    this.children = [];
    this.dataset = {};
    this.style = {};
    this.listeners = new Map();
    this.value = "";
    this.textContent = "";
    this.innerHTML = "";
    this.disabled = false;
    this.readOnly = false;
    this.tabIndex = 0;
    this.selectionStart = 0;
    this.selectionEnd = 0;
    this.focused = false;
  }
  addEventListener(name, listener) {
    const listeners = this.listeners.get(name) ?? [];
    listeners.push(listener);
    this.listeners.set(name, listeners);
  }
  emit(name, event = {}) {
    for (const listener of this.listeners.get(name) ?? []) listener(event);
  }
  append(...children) {
    this.children.push(...children);
  }
  replaceChildren(...children) {
    this.children = children;
  }
  setAttribute(name, value) {
    this[name] = value;
  }
  querySelectorAll(selector) {
    const descendants = this.children.flatMap((child) => child instanceof FakeElement ? [child, ...child.querySelectorAll(selector)] : []);
    if (selector === "[data-hierarchy-node]") return descendants.filter((child) => child.dataset.hierarchyNode);
    return [];
  }
  focus() { this.focused = true; }
  click() {
    this.clickCount = (this.clickCount ?? 0) + 1;
    this.emit("click", { preventDefault() {} });
  }
}

const ownerCapabilities = {
  readVisibleContent: true,
  editDocuments: true,
  mutateStructure: true,
  writeAnnotations: true,
  permanentlyDelete: true,
  manageMembers: true,
  manageWorkspace: true,
};

const roleCapabilities = {
  owner: ownerCapabilities,
  admin: { ...ownerCapabilities, permanentlyDelete: false, manageWorkspace: false },
  editor: { ...ownerCapabilities, permanentlyDelete: false, manageMembers: false, manageWorkspace: false },
  commenter: {
    readVisibleContent: true, editDocuments: false, mutateStructure: false, writeAnnotations: true,
    permanentlyDelete: false, manageMembers: false, manageWorkspace: false,
  },
  viewer: {
    readVisibleContent: true, editDocuments: false, mutateStructure: false, writeAnnotations: false,
    permanentlyDelete: false, manageMembers: false, manageWorkspace: false,
  },
};

function jsonResponse(payload, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(payload),
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

async function waitFor(predicate) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  assert.fail("interaction did not reach expected state");
}

test("actual thin-client interactions preserve drafts and ignore late mutations", async () => {
  const ids = [
    "status", "contentList", "title", "revision", "editor", "preview", "previewSummary",
    "searchInput", "searchResults", "previewButton", "commitButton", "searchForm", "serverStage",
    "workspaceView", "authPanel", "authTitle", "authStatus", "bootstrapField", "bootstrapInput",
    "loginField", "loginInput", "passwordInput", "authSubmit", "authModeButton", "authForm", "sessionLabel", "logoutButton",
    "permissionsButton", "permissionsPanel", "permissionSummary", "memberForm", "memberLogin", "memberPassword", "memberRole", "memberList",
    "nodeAclForm", "nodeAclMember", "nodeAclAccess", "nodeAclList",
    "explorerActivity", "searchActivity", "chronoActivity", "trashActivity", "navigationPanel", "explorerPanel", "searchPanel", "chronoPanel", "trashPanel",
    "hierarchyMode", "contentsMode", "navigationFilter", "explorerScroll", "hierarchyView", "contentsView",
    "contentsLocation", "contentsFollowing", "returnToNode", "contentsBreadcrumbs", "narrowNavigation", "widenNavigation",
    "navigationWidth", "navigationMetrics",
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
  ];
  const elements = Object.fromEntries(ids.map((id) => [id, new FakeElement()]));
  const storage = new Map();
  globalThis.document = {
    getElementById: (id) => elements[id],
    createElement: () => new FakeElement(),
    createTextNode: (text) => ({ textContent: text }),
  };
  globalThis.window = {
    localStorage: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
    },
    addEventListener() {},
  };
  globalThis.requestAnimationFrame = (callback) => { callback(); return 1; };
  globalThis.EventSource = class {
    constructor(path) {
      this.path = path;
      this.listeners = new Map();
      globalThis.EventSource.instance = this;
      globalThis.EventSource.instances ??= [];
      globalThis.EventSource.instances.push(this);
    }
    addEventListener(name, listener) {
      this.listeners.set(name, listener);
    }
    emit(name, data = "") {
      this.listeners.get(name)?.({ data });
    }
    close() {}
  };

  const revisionA = "a".repeat(64);
  const revisionB = "b".repeat(64);
  const nodeA = "550e8400-e29b-41d4-a716-446655440000";
  const nodeB = "11111111-1111-4111-8111-111111111111";
  const trashItemId = "22222222-2222-4222-8222-222222222222";
  const trashOperationId = "33333333-3333-4333-8333-333333333333";
  const trashPlanId = "44444444-4444-4444-8444-444444444444";
  const trashDigest = "d".repeat(64);
  const trashItem = {
    manifest: {
      schema: "weftext.trash-item/v1", trashItemId, operationId: trashOperationId,
      kind: "resource", trashedAt: "2026-08-24T12:00:00+08:00", originStatus: "unknown",
      originalOwnerNodeId: null, originalName: "figure.png", sha256: trashDigest, byteLength: 2048,
    },
    containedNodeIds: [],
    restore: {
      originResolution: "unknown", originalAvailable: false, withAncestorsAvailable: false,
      requiredAncestorItemIds: [], blockedReason: "legacy_origin_unknown",
    },
  };
  const metadata = (id, root) => ({
    schema: "weftext.node-metadata.v1", id, icon: null, resolvedIcon: null,
    aliases: id === nodeB ? ["节点乙"] : [], childSort: "name", childSortDirection: "ascending",
    siblingRank: null, adjacentHeadingBody: root ? "separate" : null, diagnostics: [],
  });
  const properties = { properties: [{ name: "status", value: "draft", kind: "descriptive", range: { start: 0, end: 1 }, nameRange: { start: 0, end: 1 }, valueRange: { start: 0, end: 1 } }], diagnostics: [], headerRange: { start: 0, end: 1 } };
  const nodes = [
    { id: nodeA, name: "A", parentId: null, locator: "", icon: null, displayIcon: { kind: "workspace_root" } },
    { id: nodeB, name: "B", parentId: nodeA, locator: "B", icon: null, displayIcon: { kind: "default_node" } },
  ];
  const documents = {
    [nodeA]: { nodeId: nodeA, name: "A", revision: revisionA, source: "A source", profile: { profile: "ascii_doc_v1" }, model: { blocks: [] }, metadata: metadata(nodeA, true), properties },
    [nodeB]: { nodeId: nodeB, name: "B", revision: revisionB, source: "B source", profile: { profile: "ascii_doc_v1" }, model: { blocks: [] }, metadata: metadata(nodeB, false), properties },
  };
  let pendingPreview = null;
  let pendingCommit = null;
  let pendingCommitRequest = null;
  let loginCalls = 0;
  let sessionExpired = false;
  let trashCommitted = false;
  let trashCommitCalls = 0;
  const trashRestoreRequests = [];
  const openCounts = { [nodeA]: 0, [nodeB]: 0 };
  globalThis.fetch = async (path, options = {}) => {
    if (path.endsWith("/health")) return jsonResponse({ stage: "canonical-asciidoc-owner-only" });
    if (path.endsWith("/admin/backup/capabilities")) return jsonResponse({
      schema: "weftext.server-backup-capabilities.v1", ownerOnly: true,
      fullWorkspaceAndControlPlanePair: true, exclusiveLease: true, apiQuiescence: true,
      alternateCleanRestore: true, restoreDrill: true, managedShape: "X/X.adoc",
      annotations: "node_local_weftext.annotations.json",
    });
    if (path.endsWith("/capabilities")) return jsonResponse({ loopbackOnly: true, deploymentReady: false, roleCapabilities });
    if (path.endsWith("/auth/session")) {
      return loginCalls > 0 && !sessionExpired
        ? jsonResponse({ authenticated: true, role: "owner", actorScope: "3".repeat(64), capabilities: ownerCapabilities })
        : jsonResponse({ error: { code: "authentication_required", message: "login" } }, 401);
    }
    if (path.endsWith("/auth/login")) {
      loginCalls += 1;
      return jsonResponse({ authenticated: true, role: "owner", actorScope: "3".repeat(64), capabilities: ownerCapabilities });
    }
    if (path.endsWith("/auth/logout")) return jsonResponse({ authenticated: false });
    if (path.endsWith("/admin/members") && (options.method ?? "GET") === "GET") {
      return jsonResponse([{ actorScope: "3".repeat(64), login: "owner", role: "owner", enabled: true }]);
    }
    if (path.endsWith("/admin/node-acl") && (options.method ?? "GET") === "GET") return jsonResponse([]);
    if (path.endsWith("/workspace")) return jsonResponse({
      workspaceScope: "1".repeat(64), workspaceRevision: "2".repeat(64), rootNodeId: nodeA, nodes,
      documentFormat: { generation: "ascii_doc_v1", canonicalExtension: "adoc", mediaType: "text/asciidoc" },
      content: [],
      navigation: {
        version: 1,
        rootNodeId: nodeA,
        hierarchy: [
          { nodeId: nodeA, name: "A", parentNodeId: null, locator: "", depth: 0, childCount: 1, displayIcon: { kind: "workspace_root" } },
          { nodeId: nodeB, name: "B", parentNodeId: nodeA, locator: "B", depth: 1, childCount: 0, displayIcon: { kind: "default_node" } },
        ],
        contents: [
          { kind: "managed_node", name: "A", locator: "", parentLocator: null, nodeId: nodeA, ownerNodeId: null, displayIcon: { kind: "workspace_root" } },
          { kind: "managed_node", name: "B", locator: "B", parentLocator: "", nodeId: nodeB, ownerNodeId: null, displayIcon: { kind: "default_node" } },
          { kind: "unmanaged_directory", name: "Files", locator: "Files", parentLocator: "", nodeId: null, ownerNodeId: null, displayIcon: { kind: "folder" } },
          { kind: "unmanaged_markdown", name: "plain.md", locator: "plain.md", parentLocator: "", nodeId: null, ownerNodeId: null, displayIcon: { kind: "markdown_file" } },
          { kind: "unmanaged_markdown", name: "inside.md", locator: "Files/inside.md", parentLocator: "Files", nodeId: null, ownerNodeId: null, displayIcon: { kind: "markdown_file" } },
        ],
      },
    });
    if (path.endsWith("/trash")) return jsonResponse({
      workspaceRevision: "2".repeat(64),
      state: "ready",
      legacyMigrationRequired: false,
      items: trashCommitted ? [] : [trashItem],
      reconciliation: { required: false, issueCount: 0 },
    });
    if (path.includes(`/trash/items/${trashItemId}/restore/preview`)) {
      trashRestoreRequests.push(JSON.parse(options.body));
      return jsonResponse({
        planId: trashPlanId, baseWorkspaceRevision: "2".repeat(64), action: "trash_restore",
        scopeSummary: null,
        identityMap: [],
        capturedTarget: { kind: "trash_item", trashItemId, resolvedBy: "explicit_row" },
        targetNodeIds: [nodeA],
        draftSensitiveNodeIds: [nodeA],
        draftGate: {
          requiredCleanNodeIds: [nodeA], blockingDirtyNodeIds: [], observationDigest: "e".repeat(64),
        },
        trashItemChanges: [{
          disposition: "restored", manifest: trashItem.manifest,
          destinationNodeId: nodeA, destinationName: "restored-figure.png",
        }],
      });
    }
    if (path.includes(`/trash/transactions/${trashPlanId}/commit`)) {
      trashCommitCalls += 1;
      trashCommitted = true;
      return jsonResponse({ committed: true, auditRecorded: true });
    }
    if (path.includes("/search?")) return jsonResponse({ results: [{ id: nodeB, name: "B", snippet: "match", icon: null }] });
    if (path.includes("/collaboration/documents/")) {
      const parts = path.split("/");
      const collaborationNodeId = parts[parts.indexOf("documents") + 1];
      const collaborationDocument = documents[collaborationNodeId];
      const collaborationPayload = {
        wireVersion: "weftext.collaboration.v1",
        nodeId: collaborationNodeId,
        actorId: "77777777-7777-4777-8777-777777777777",
        state: {
          wireVersion: "weftext.collaboration.v1",
          epoch: 1,
          version: 0,
          revision: collaborationDocument.revision,
          frozen: collaborationNodeId === nodeB,
          ...(collaborationNodeId === nodeB ? { reason: "external_edit" } : {}),
        },
        source: collaborationDocument.source,
        participants: [{
          actorId: "77777777-7777-4777-8777-777777777777",
          clientId: "88888888-8888-4888-8888-888888888888",
          role: "owner",
          cursor: 0,
          selectionStart: 0,
          selectionEnd: 0,
          expiresAt: 9999999999,
        }],
      };
      if (path.endsWith("/presence")) return jsonResponse(collaborationPayload);
      if (path.endsWith("/drafts")) {
        pendingCommitRequest = JSON.parse(options.body);
        pendingCommit = deferred();
        return pendingCommit.promise;
      }
      return jsonResponse(collaborationPayload);
    }
    if (path.endsWith("/preview")) {
      pendingPreview = deferred();
      return pendingPreview.promise;
    }
    if (options.method === "PUT") {
      pendingCommit = deferred();
      return pendingCommit.promise;
    }
    const nodeId = path.split("/").at(-1);
    openCounts[nodeId] += 1;
    return jsonResponse(documents[nodeId]);
  };

  await import(`../app.js?interaction=${Date.now()}`);
  await waitFor(() => elements.authPanel.hidden === false);
  elements.authModeButton.emit("click");
  assert.equal(elements.bootstrapField.hidden, false);
  assert.equal(elements.loginField.hidden, true);
  elements.authModeButton.emit("click");
  assert.equal(elements.bootstrapField.hidden, true);
  elements.loginInput.value = "owner";
  elements.passwordInput.value = "correct horse battery staple";
  elements.authForm.emit("submit", { preventDefault() {} });
  await waitFor(() => elements.editor.value === "A source");
  await waitFor(() => elements.collaborationParticipants.children.length === 1);
  assert.match(elements.collaborationState.textContent, /实时协作就绪/);
  const collaborationEvents = globalThis.EventSource.instances.find((eventSource) => eventSource.path.endsWith("/collaboration/events"));
  collaborationEvents.emit("resync-required", JSON.stringify({ reason: "lagged" }));
  assert.match(elements.collaborationState.textContent, /resync authoritative/);
  await waitFor(() => /实时协作就绪/.test(elements.collaborationState.textContent));
  assert.equal(loginCalls, 1);
  assert.equal(elements.nodeMetadataValues.children[1].textContent, nodeA);
  assert.equal(elements.documentPropertyValues.children[0].textContent, "status");
  assert.equal(elements.documentPropertyValues.children[1].textContent, "draft");
  elements.permissionsButton.emit("click");
  await waitFor(() => elements.memberList.children.length === 1);
  assert.equal(elements.permissionsPanel.hidden, false);
  assert.equal(elements.memberForm.hidden, false);
  assert.equal(elements.nodeAclForm.hidden, false);
  assert.equal(elements.nodeAclMember.children.length, 1);
  assert.match(elements.permissionSummary.textContent, /owner/);
  assert.equal(elements.contentList.children.length, 3);
  assert.match(elements.contentList.children[2].children[0].innerHTML, /plain\.md/);

  elements.trashActivity.emit("click");
  await waitFor(() => elements.trashItemList.children[0]?.["aria-label"] === "打开 Trash Item figure.png");
  elements.trashItemList.children[0].emit("click");
  assert.equal(elements.trashRestoreMode.value, "existing_target");
  assert.equal(elements.trashRestoreOriginalOption.disabled, true);
  assert.equal(elements.trashRestoreAncestorsOption.disabled, true);
  elements.trashRestorePreviewButton.emit("click");
  await waitFor(() => /必须显式选择现有目标/.test(elements.trashStatus.textContent));
  assert.equal(trashRestoreRequests.length, 0);
  elements.trashRestoreTarget.value = nodeA;
  elements.trashRestoreName.value = "restored-figure.png";
  elements.trashRestorePreviewButton.emit("click");
  await waitFor(() => elements.trashPlanPanel.hidden === false);
  assert.deepEqual(trashRestoreRequests, [{
    baseWorkspaceRevision: "2".repeat(64), mode: "existing_target",
    resolvedBy: "explicit_row", targetNodeId: nodeA, name: "restored-figure.png",
  }]);
  assert.equal(JSON.stringify(trashRestoreRequests).includes("_weftext.items"), false);
  assert.match(elements.trashPlanOutput.textContent, new RegExp(trashDigest));
  elements.trashPlanConfirm.checked = true;
  elements.trashPlanConfirm.emit("change");
  assert.equal(elements.trashPlanCommitButton.disabled, false);
  elements.trashPlanCommitButton.emit("click");
  await waitFor(() => trashCommitCalls === 1 && /authoritative inventory/.test(elements.trashStatus.textContent));
  elements.trashPlanCommitButton.emit("click");
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(trashCommitCalls, 1);
  const openCountAfterTrash = openCounts[nodeA];

  elements.contentsMode.emit("click");
  elements.contentList.children[1].children[0].emit("click");
  assert.equal(elements.title.textContent, "A");
  assert.equal(elements.contentsLocation.textContent, "正在浏览：Files");
  assert.match(elements.contentList.children[0].children[0].innerHTML, /inside\.md/);
  elements.returnToNode.emit("click");
  elements.hierarchyMode.emit("click");
  const events = globalThis.EventSource.instances.find((eventSource) => eventSource.path.endsWith("/changes"));
  events.emit("open");
  await waitFor(() => openCounts[nodeA] === openCountAfterTrash + 1);
  events.emit("open");
  await waitFor(() => openCounts[nodeA] === openCountAfterTrash + 2);
  events.emit("resync-required", JSON.stringify({ reason: "lagged", missedEvents: 4 }));
  await waitFor(() => openCounts[nodeA] === openCountAfterTrash + 3);

  elements.editor.value = "A exact dirty\r\n";
  elements.editor.emit("input");
  elements.hierarchyView.children[1].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "B source");
  await waitFor(() => /已冻结/.test(elements.collaborationState.textContent));
  assert.equal(elements.collaborationStatus.dataset.state, "frozen");
  elements.hierarchyView.children[0].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "A exact dirty\r\n");

  elements.searchInput.value = "B";
  elements.searchForm.emit("submit", { preventDefault() {} });
  await waitFor(() => elements.searchResults.children.length === 1);
  elements.searchResults.children[0].emit("click");
  await waitFor(() => elements.editor.value === "B source");
  elements.hierarchyView.children[0].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "A exact dirty\r\n");

  elements.previewButton.emit("click");
  await waitFor(() => pendingPreview !== null);
  elements.hierarchyView.children[1].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "B source");
  pendingPreview.resolve(jsonResponse({
    oldLength: 1,
    newLength: 2,
    changed: true,
    model: { blocks: [{ kind: "paragraph", text: "late preview" }] },
  }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(elements.title.textContent, "B");
  assert.equal(elements.editor.value, "B source");
  assert.notEqual(elements.previewSummary.textContent, "1 → 2 字节 · 有更改");

  elements.hierarchyView.children[0].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "A exact dirty\r\n");
  elements.commitButton.emit("click");
  await waitFor(() => pendingCommit !== null);
  elements.hierarchyView.children[1].children.at(-1).emit("click");
  await waitFor(() => elements.editor.value === "B source");
  pendingCommit.resolve(jsonResponse({
    wireVersion: "weftext.collaboration.v1",
    status: "accepted",
    nodeId: nodeA,
    actorId: "77777777-7777-4777-8777-777777777777",
    clientId: pendingCommitRequest.clientId,
    operationId: pendingCommitRequest.operationId,
    transactionId: pendingCommitRequest.operationId,
    requestBaseRevision: revisionA,
    requestBaseVersion: 0,
    appliedBaseRevision: revisionA,
    appliedBaseVersion: 0,
    resultRevision: "c".repeat(64),
    state: {
      wireVersion: "weftext.collaboration.v1",
      epoch: 1,
      version: 1,
      revision: "c".repeat(64),
      frozen: false,
    },
    transformed: false,
    operations: [],
    auditRecorded: true,
  }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(elements.title.textContent, "B");
  assert.equal(elements.revision.textContent, revisionB);
  assert.equal(elements.editor.value, "B source");

  elements.logoutButton.emit("click");
  await waitFor(() => elements.authPanel.hidden === false && elements.workspaceView.hidden === true);
  elements.loginInput.value = "owner";
  elements.passwordInput.value = "correct horse battery staple";
  elements.authForm.emit("submit", { preventDefault() {} });
  await waitFor(() => elements.workspaceView.hidden === false);
  sessionExpired = true;
  globalThis.EventSource.instances.at(-1).emit("error");
  await waitFor(() => elements.authPanel.hidden === false && elements.workspaceView.hidden === true);
  assert.match(elements.authStatus.textContent, /会话已失效/);
});

test("hosted annotation, task, and query surfaces guard IME, replay, stale responses, and hidden data", async () => {
  const ids = [
    "status", "contentList", "title", "revision", "editor", "preview", "previewSummary",
    "searchInput", "searchResults", "previewButton", "commitButton", "searchForm", "serverStage",
    "workspaceView", "authPanel", "authTitle", "authStatus", "bootstrapField", "bootstrapInput",
    "loginField", "loginInput", "passwordInput", "authSubmit", "authModeButton", "authForm", "sessionLabel", "logoutButton",
    "permissionsButton", "permissionsPanel", "permissionSummary", "memberForm", "memberLogin", "memberPassword", "memberRole", "memberList",
    "nodeAclForm", "nodeAclMember", "nodeAclAccess", "nodeAclList",
    "explorerActivity", "searchActivity", "chronoActivity", "trashActivity", "navigationPanel", "explorerPanel", "searchPanel", "chronoPanel", "trashPanel",
    "hierarchyMode", "contentsMode", "navigationFilter", "explorerScroll", "hierarchyView", "contentsView",
    "contentsLocation", "contentsFollowing", "returnToNode", "contentsBreadcrumbs", "narrowNavigation", "widenNavigation",
    "navigationWidth", "navigationMetrics", "nodeMetadataValues", "nodeMetadataAliases", "nodeMetadataDiagnostics", "documentPropertyValues",
    "inspectorTabs", "propertiesTab", "annotationsTab", "tasksTab", "queryTab", "propertiesPanel",
    "annotationsPanel", "annotationStatus", "annotationCreateForm", "annotationKind", "annotationTarget",
    "annotationBody", "annotationSuggestedSource", "annotationLabels", "annotationMark", "annotationTheme",
    "annotationCreateButton", "annotationList", "tasksPanel", "taskStatus", "taskList", "taskPlanPanel",
    "taskPlanId", "taskPlanIntent", "taskPlanGenerated", "taskPlanSource", "taskPlanConfirm",
    "taskPlanChanges", "taskPlanCancelButton", "taskPlanCommitButton", "queryPanel",
    "queryForm", "querySource", "queryBlockIndex", "queryToday", "queryRunButton", "queryStatus",
    "queryDiagnostics", "queryGroups", "queryTableContainer", "queryCsvButton",
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
  ];
  const elements = Object.fromEntries(ids.map((id) => [id, new FakeElement()]));
  const createdAnchors = [];
  globalThis.document = {
    getElementById: (id) => elements[id],
    createElement: (tagName) => {
      const element = new FakeElement();
      element.tagName = tagName;
      if (tagName === "a") createdAnchors.push(element);
      return element;
    },
    createTextNode: (text) => ({ textContent: text }),
  };
  globalThis.window = {
    localStorage: { getItem: () => null, setItem() {} },
    addEventListener() {},
  };
  globalThis.requestAnimationFrame = (callback) => { callback(); return 1; };
  globalThis.EventSource = class {
    addEventListener() {}
    close() {}
  };

  const rootId = "550e8400-e29b-41d4-a716-446655440000";
  const otherId = "11111111-1111-4111-8111-111111111111";
  const hiddenId = "99999999-9999-4999-8999-999999999999";
  const taskId = "22222222-2222-4222-8222-222222222222";
  const annotationId = "33333333-3333-4333-8333-333333333333";
  const messageId = "44444444-4444-4444-8444-444444444444";
  const firstPlanId = "55555555-5555-4555-8555-555555555555";
  const stalePlanId = "66666666-6666-4666-8666-666666666666";
  let workspaceRevision = "1".repeat(64);
  let rootRevision = "a".repeat(64);
  let rootSource = "= 根\n\n你🙂 task\n";
  let annotationPosts = 0;
  let annotationGets = 0;
  let taskPreviewCalls = 0;
  let taskCommitCalls = 0;
  let queryCalls = 0;
  let inventoryCalls = 0;
  let rootDocumentReads = 0;
  let sessionRole = "owner";
  const requestedPaths = [];
  const staleTaskPreview = deferred();
  const firstQuery = deferred();
  const taskCommit = deferred();

  const metadata = (id, root) => ({
    schema: "weftext.node-metadata.v1", id, icon: null, resolvedIcon: null, aliases: [],
    childSort: "name", childSortDirection: "ascending", siblingRank: null,
    adjacentHeadingBody: root ? "separate" : null, diagnostics: [],
  });
  const documentPayload = (id) => ({
    nodeId: id,
    name: id === rootId ? "根" : "可见节点",
    revision: id === rootId ? rootRevision : "b".repeat(64),
    source: id === rootId ? rootSource : "= 可见节点\n",
    profile: { profile: "ascii_doc_v1" },
    model: { blocks: [] },
    metadata: metadata(id, id === rootId),
    properties: { properties: [], diagnostics: [], headerRange: { start: 0, end: 0 } },
  });
  const workspacePayload = () => ({
    workspaceScope: "7".repeat(64),
    workspaceRevision,
    rootNodeId: rootId,
    documentFormat: { generation: "ascii_doc_v1", canonicalExtension: "adoc", mediaType: "text/asciidoc" },
    nodes: [
      { id: rootId, name: "根", parentId: null, locator: "", icon: null, displayIcon: { kind: "workspace_root" } },
      { id: otherId, name: "可见节点", parentId: rootId, locator: "Visible", icon: null, displayIcon: { kind: "default_node" } },
    ],
    content: [],
    navigation: {
      version: 1,
      rootNodeId: rootId,
      hierarchy: [
        { nodeId: rootId, name: "根", parentNodeId: null, locator: "", depth: 0, childCount: 1, displayIcon: { kind: "workspace_root" } },
        { nodeId: otherId, name: "可见节点", parentNodeId: rootId, locator: "Visible", depth: 1, childCount: 0, displayIcon: { kind: "default_node" } },
      ],
      contents: [
        { kind: "managed_node", name: "根", locator: "", parentLocator: null, nodeId: rootId, ownerNodeId: null, displayIcon: { kind: "workspace_root" } },
        { kind: "managed_node", name: "可见节点", locator: "Visible", parentLocator: "", nodeId: otherId, ownerNodeId: null, displayIcon: { kind: "default_node" } },
      ],
    },
  });
  const annotationPayload = (node = rootId) => ({
    nodeId: node,
    workspaceRevision,
    revision: node === rootId ? rootRevision : "b".repeat(64),
    store: {
      version: 3,
      document_id: node,
      annotations: node === rootId ? [{
        id: annotationId,
        kind: "suggestion_insert",
        target: { kind: "document" },
        suggested_source: "建议源码",
        labels: ["review"],
        thread: [{
          id: messageId,
          author_id: annotationId,
          author_name: "owner",
          body: { format: "weftext.asciidoc.inline.v1", source: "可见批注" },
          created_at: "2026-08-24T00:00:00Z",
          updated_at: "2026-08-24T00:00:00Z",
        }],
        state: "open",
        created_at: "2026-08-24T00:00:00Z",
        updated_at: "2026-08-24T00:00:00Z",
      }] : [],
    },
  });
  const taskPayload = (node = rootId) => ({
    nodeId: node,
    occurrences: node === rootId ? [{
      nodeId: rootId,
      revision: rootRevision,
      task: {
        state: "open",
        description: "可见任务",
        range: { start: 12, end: 22 },
        metadata: {
          id: taskId,
          priority: "high",
          phase: "todo",
          resolution: null,
          recurrence: null,
          repeatFrom: "due",
          dependencies: [],
        },
      },
    }] : [],
    diagnostics: [],
  });
  const taskPlan = (id) => ({
    planId: id,
    nodeId: rootId,
    baseWorkspaceRevision: workspaceRevision,
    authoring: {
      proposedSource: "= 根\n\n* [x] 可见任务\n",
      assignedId: null,
    },
    documentChanges: [{
      nodeId: rootId,
      path: "Root.adoc",
      baseRevision: rootRevision,
      nextRevision: "c".repeat(64),
      editCount: 1,
    }],
  });
  const queryPayload = (label) => ({
    valid: true,
    workspaceRevision,
    execution: {
      blockIndex: 0,
      analysis: { blocks: [], diagnostics: [] },
      result: {
        source: "nodes",
        columns: [{ outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }],
        rows: [{ identity: { kind: "node", nodeId: rootId, revision: rootRevision }, cells: [{ column: { outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }, value: { kind: "text", value: label } }] }],
        groups: [{ column: { outputName: "group_name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }, value: { kind: "text", value: "可见分组" }, rowCount: 1 }],
        totalBeforeLimit: 1,
        truncated: false,
      },
      csv: `name\r\n${label}\r\n`,
    },
  });

  globalThis.fetch = async (path, options = {}) => {
    requestedPaths.push(path);
    if (path.endsWith("/health")) return jsonResponse({ stage: "hosted-products" });
    if (path.endsWith("/capabilities")) return jsonResponse({ loopbackOnly: true, deploymentReady: false, roleCapabilities });
    if (path.endsWith("/auth/session") || path.endsWith("/auth/login")) return jsonResponse({
      authenticated: true,
      role: sessionRole,
      actorScope: "8".repeat(64),
      capabilities: roleCapabilities[sessionRole],
    });
    if (path.endsWith("/auth/logout")) return jsonResponse({ authenticated: false });
    if (path.endsWith("/workspace")) {
      inventoryCalls += 1;
      return jsonResponse(workspacePayload());
    }
    if (path.includes("/annotations/")) {
      const node = path.split("/").at(-1);
      if ((options.method ?? "GET") === "GET") {
        annotationGets += 1;
        return jsonResponse(annotationPayload(node));
      }
      annotationPosts += 1;
      workspaceRevision = "2".repeat(64);
      return jsonResponse({ ...annotationPayload(node), auditRecorded: true });
    }
    if (path.includes("/tasks/nodes/") && path.endsWith("/edit/preview")) {
      taskPreviewCalls += 1;
      return taskPreviewCalls === 1 ? jsonResponse(taskPlan(firstPlanId)) : staleTaskPreview.promise;
    }
    if (path.includes("/tasks/nodes/")) {
      const parts = path.split("/");
      return jsonResponse(taskPayload(parts.at(-1)));
    }
    if (path.includes("/tasks/transactions/")) {
      taskCommitCalls += 1;
      return taskCommitCalls === 1 ? taskCommit.promise : jsonResponse({
        error: {
          code: "stale_workspace_revision",
          message: "workspace changed after preview",
          conflict: { expectedRevision: "3".repeat(64), actualRevision: "4".repeat(64) },
        },
      }, 409);
    }
    if (path.endsWith("/queries/execute")) {
      queryCalls += 1;
      const source = JSON.parse(options.body).source;
      return source === "first" ? firstQuery.promise : jsonResponse(queryPayload("SECOND_VISIBLE"));
    }
    if (path.includes(`/documents/${rootId}`)) rootDocumentReads += 1;
    if (path.includes("/documents/")) return jsonResponse(documentPayload(path.split("/").at(-1)));
    throw new Error(`unexpected route ${path}`);
  };

  await import(`../app.js?products=${Date.now()}`);
  await waitFor(() => elements.editor.value === rootSource);
  assert.equal(elements.propertiesTab["aria-selected"], "true");

  let prevented = false;
  elements.inspectorTabs.emit("keydown", { key: "ArrowRight", preventDefault() { prevented = true; } });
  await waitFor(() => annotationGets === 1 && elements.annotationList.children.length === 1);
  assert.equal(prevented, true);
  assert.equal(elements.annotationsTab["aria-selected"], "true");
  assert.equal(elements.annotationsTab.focused, true);
  assert.equal(elements.annotationsPanel.hidden, false);
  assert.equal(elements.annotationList.children[0].children[2].textContent, "建议源码");

  elements.annotationKind.value = "suggestion_insert";
  elements.annotationKind.emit("change");
  assert.equal(elements.annotationTarget.value, "insertion_point");
  assert.equal(elements.annotationTarget.disabled, true);
  assert.equal(elements.annotationSuggestedSource.disabled, false);
  elements.annotationKind.value = "comment";
  elements.annotationKind.emit("change");
  elements.annotationTarget.value = "document";
  elements.annotationMark.value = "none";
  elements.annotationTheme.value = "yellow";
  elements.annotationBody.value = "中文输入法批注";
  elements.annotationBody.emit("keydown", { key: "Enter", ctrlKey: true, isComposing: true, keyCode: 229, preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(annotationPosts, 0);
  elements.annotationBody.emit("keydown", { key: "Enter", ctrlKey: true, isComposing: false, keyCode: 13, preventDefault() {} });
  await waitFor(() => annotationPosts === 1 && annotationGets >= 2 && inventoryCalls >= 2 && rootDocumentReads >= 2);
  assert.match(elements.status.textContent, /authoritative/);

  elements.tasksTab.emit("click");
  await waitFor(() => elements.taskList.children.length === 1);
  const firstTaskActions = elements.taskList.children[0].children[2];
  firstTaskActions.children[0].emit("click");
  await waitFor(() => elements.taskPlanPanel.hidden === false);
  assert.equal(elements.taskPlanId.textContent, firstPlanId);
  assert.match(elements.taskPlanIntent.textContent, /"route": "edit"/);
  assert.equal(elements.taskPlanSource.textContent, "= 根\n\n* [x] 可见任务\n");
  assert.equal(elements.taskPlanCommitButton.disabled, true);
  elements.taskPlanConfirm.checked = true;
  elements.taskPlanConfirm.emit("change");
  assert.equal(elements.taskPlanCommitButton.disabled, false);
  elements.taskPlanCommitButton.emit("click");
  elements.taskPlanCommitButton.emit("click");
  await waitFor(() => taskCommitCalls === 1);
  assert.equal(elements.taskPlanPanel.hidden, true);
  workspaceRevision = "3".repeat(64);
  rootRevision = "c".repeat(64);
  rootSource = "= 根\n\n已提交任务\n";
  taskCommit.resolve(jsonResponse({ committed: true }));
  await waitFor(() => elements.editor.value === rootSource && elements.taskList.children.length === 1);

  elements.taskList.children[0].children[2].children[0].emit("click");
  await waitFor(() => taskPreviewCalls === 2);
  elements.hierarchyView.children[1].children.at(-1).emit("click");
  await waitFor(() => elements.title.textContent === "可见节点");
  staleTaskPreview.resolve(jsonResponse(taskPlan(stalePlanId)));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(elements.taskPlanPanel.hidden, true);
  assert.notEqual(elements.taskPlanId.textContent, stalePlanId);

  elements.hierarchyView.children[0].children.at(-1).emit("click");
  await waitFor(() => elements.title.textContent === "根" && elements.taskList.children.length === 1);
  elements.taskList.children[0].children[2].children[0].emit("click");
  await waitFor(() => elements.taskPlanPanel.hidden === false && elements.taskPlanId.textContent === stalePlanId);
  elements.taskPlanConfirm.checked = true;
  elements.taskPlanConfirm.emit("change");
  elements.taskPlanCommitButton.emit("click");
  await waitFor(() => taskCommitCalls === 2 && /stale/.test(elements.taskStatus.textContent));
  assert.match(elements.taskStatus.textContent, /stale/);
  assert.equal(elements.status.dataset.tone, "conflict");

  elements.queryTab.emit("click");
  elements.queryBlockIndex.value = "0";
  elements.queryToday.value = "2026-08-24";
  elements.querySource.value = "first";
  elements.querySource.emit("keydown", { key: "Enter", ctrlKey: true, isComposing: true, keyCode: 229, preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(queryCalls, 0);
  elements.queryForm.emit("submit", { preventDefault() {} });
  await waitFor(() => queryCalls === 1);
  elements.querySource.value = "second";
  elements.queryForm.emit("submit", { preventDefault() {} });
  await waitFor(() => queryCalls === 2 && elements.queryStatus.textContent.startsWith("1/1"));
  firstQuery.resolve(jsonResponse(queryPayload("LATE_HIDDEN")));
  await new Promise((resolve) => setTimeout(resolve, 0));
  const table = elements.queryTableContainer.children[0];
  const resultButton = table.children[1].children[0].children[0].children[0];
  assert.equal(resultButton.textContent, "SECOND_VISIBLE");
  assert.equal(elements.queryGroups.children[0].textContent, "group_name: 可见分组: 1");
  elements.queryCsvButton.emit("click");
  assert.equal(createdAnchors.at(-1).download, "weftext-query.csv");
  assert.equal(createdAnchors.at(-1).clickCount, 1);

  const projectedText = (element) => [
    element?.textContent ?? "",
    element?.innerHTML ?? "",
    ...(element?.children ?? []).map(projectedText),
  ].flat(Infinity).join(" ");
  assert.doesNotMatch(projectedText(elements.workspaceView), new RegExp(hiddenId));
  assert.ok(requestedPaths.every((path) => !path.includes(hiddenId)));

  elements.logoutButton.emit("click");
  await waitFor(() => elements.authPanel.hidden === false && elements.workspaceView.hidden === true);
  sessionRole = "viewer";
  elements.loginInput.value = "viewer";
  elements.passwordInput.value = "viewer password";
  elements.authForm.emit("submit", { preventDefault() {} });
  await waitFor(() => elements.workspaceView.hidden === false && elements.editor.readOnly === true);
  assert.equal(elements.previewButton.disabled, true);
  assert.equal(elements.commitButton.disabled, true);
  assert.equal(elements.queryRunButton.disabled, false);

  const adminRequestsBefore = requestedPaths.filter((path) => path.includes("/admin/")).length;
  elements.permissionsButton.emit("click");
  await waitFor(() => /不向当前角色披露/.test(elements.permissionSummary.textContent));
  assert.equal(elements.memberForm.hidden, true);
  assert.equal(elements.nodeAclForm.hidden, true);
  assert.equal(requestedPaths.filter((path) => path.includes("/admin/")).length, adminRequestsBefore);

  const annotationReadsBefore = annotationGets;
  elements.annotationsTab.emit("click");
  await waitFor(() => annotationGets > annotationReadsBefore);
  assert.equal(elements.annotationCreateButton.disabled, true);
  assert.equal(elements.annotationBody.disabled, true);
  elements.tasksTab.emit("click");
  await waitFor(() => elements.taskList.children.length === 1);
  assert.equal(elements.taskList.children[0].children[2].children[0].disabled, true);
  assert.match(elements.taskStatus.textContent, /当前角色没有此写入能力/);
});
