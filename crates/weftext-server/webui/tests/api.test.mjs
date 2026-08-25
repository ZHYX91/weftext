import assert from "node:assert/strict";
import test from "node:test";

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
  requireCanonicalNodeMetadata,
  requireCanonicalWorkspacePayload,
  requireCollaborationOperation,
  requireCollaborationSnapshot,
  requireQueryExecutionPayload,
  requireRoleCapabilityMap,
  requireSessionCapabilities,
  requireTaskInspectionPayload,
  requireTaskPreviewPayload,
  requireTrashInventoryPayload,
  requireTrashPlanPayload,
  safeguardOutgoingDraft,
  serverApi,
  surfaceContextMatches,
  taskPreviewConfirmation,
  taskTargetForOccurrence,
  trashPermanentDeleteConfirmation,
} from "../api.js";

const workspaceScopeA = "1".repeat(64);
const workspaceScopeB = "2".repeat(64);
const actorScopeA = "3".repeat(64);
const actorScopeB = "4".repeat(64);
const nodeId = "550e8400-e29b-41d4-a716-446655440000";
const planId = "11111111-1111-4111-8111-111111111111";

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

test("managed payload guards accept only canonical AsciiDoc authority", () => {
  const workspace = {
    documentFormat: {
      generation: "ascii_doc_v1",
      canonicalExtension: "adoc",
      mediaType: "text/asciidoc",
    },
  };
  const metadata = {
    schema: "weftext.node-metadata.v1",
    id: "550e8400-e29b-41d4-a716-446655440000",
    icon: "weftext:future-token",
    resolvedIcon: null,
    aliases: ["文缕"],
    childSort: "name",
    childSortDirection: "ascending",
    siblingRank: null,
    adjacentHeadingBody: "separate",
    diagnostics: [],
  };
  const documentPayload = {
    nodeId: metadata.id,
    profile: { profile: "ascii_doc_v1" },
    metadata,
    properties: { properties: [], diagnostics: [], headerRange: { start: 0, end: 0 } },
  };
  assert.equal(requireCanonicalWorkspacePayload(workspace), workspace);
  assert.equal(requireCanonicalDocumentPayload(documentPayload), documentPayload);
  assert.equal(requireCanonicalNodeMetadata(metadata), metadata);
  assert.throws(() => requireCanonicalNodeMetadata({ ...metadata, icon: ["weftext:book"] }), /节点元数据投影/);
  assert.throws(() => requireCanonicalNodeMetadata({ ...metadata, recipe: [] }), /节点元数据投影/);
  assert.throws(
    () => requireCanonicalWorkspacePayload({
      documentFormat: {
        generation: "markdown_v1",
        canonicalExtension: "md",
        mediaType: "text/markdown",
      },
    }),
    /非 canonical AsciiDoc 工作区/,
  );
  assert.throws(
    () => requireCanonicalDocumentPayload({ profile: { profile: "markdown_v1" } }),
    /非 canonical AsciiDoc 托管文档/,
  );
});

test("five server roles drive product visibility and mutation affordances fail-closed", () => {
  assert.equal(requireRoleCapabilityMap(roleCapabilities), roleCapabilities);
  const access = Object.fromEntries(Object.entries(roleCapabilities).map(([role, capabilities]) => [
    role,
    productSurfaceAccess(requireSessionCapabilities({ authenticated: true, role, capabilities })),
  ]));
  for (const role of ["owner", "admin", "editor"]) {
    assert.deepEqual(
      [access[role].readAnnotations, access[role].writeAnnotations, access[role].readTasks, access[role].writeTasks, access[role].executeQueries, access[role].editDocuments],
      [true, true, true, true, true, true],
    );
  }
  assert.deepEqual(
    [access.commenter.readAnnotations, access.commenter.writeAnnotations, access.commenter.readTasks, access.commenter.writeTasks, access.commenter.executeQueries, access.commenter.editDocuments],
    [true, true, true, false, true, false],
  );
  assert.deepEqual(
    [access.viewer.readAnnotations, access.viewer.writeAnnotations, access.viewer.readTasks, access.viewer.writeTasks, access.viewer.executeQueries, access.viewer.editDocuments],
    [true, false, true, false, true, false],
  );
  assert.equal(access.admin.manageMembers, true);
  assert.equal(access.editor.manageMembers, false);
  assert.throws(() => requireRoleCapabilityMap({ owner: ownerCapabilities }), /五角色能力矩阵/);
  assert.throws(
    () => requireSessionCapabilities({ authenticated: true, role: "viewer", capabilities: { ...roleCapabilities.viewer, writeAnnotations: "no" } }),
    /角色能力契约/,
  );
  assert.throws(
    () => requireSessionCapabilities({ authenticated: true, role: "unknown", capabilities: ownerCapabilities }),
    /认证会话/,
  );
  assert.throws(
    () => requireSessionCapabilities(
      { authenticated: true, role: "viewer", capabilities: roleCapabilities.viewer },
      { ...roleCapabilities, viewer: { ...roleCapabilities.viewer, writeAnnotations: true } },
    ),
    /能力矩阵不一致/,
  );
});

test("structured product clients use only typed versioned routes and CSRF-protected mutations", async () => {
  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    return jsonResponse({});
  };
  await serverApi.annotations(nodeId);
  await serverApi.commitAnnotation(nodeId, { action: "resolve" });
  await serverApi.inspectTasks(nodeId);
  await serverApi.previewTaskEdit(nodeId, { intent: { kind: "toggle" } });
  await serverApi.previewTaskRecurrence(nodeId, { context: { completedAt: { kind: "date", value: "2026-08-24" } } });
  await serverApi.previewTaskDependencies(nodeId, { dependencies: [] });
  await serverApi.commitTask(planId);
  await serverApi.executeQuery("query source", 0, { today: { year: 2026, month: 8, day: 24 } });
  assert.deepEqual(calls.map(({ path }) => path), [
    `/api/v1/annotations/${nodeId}`,
    `/api/v1/annotations/${nodeId}`,
    `/api/v1/tasks/nodes/${nodeId}`,
    `/api/v1/tasks/nodes/${nodeId}/edit/preview`,
    `/api/v1/tasks/nodes/${nodeId}/recurrence/preview`,
    `/api/v1/tasks/nodes/${nodeId}/dependencies/preview`,
    `/api/v1/tasks/transactions/${planId}/commit`,
    "/api/v1/queries/execute",
  ]);
  assert.deepEqual(calls.map(({ options }) => options.method ?? "GET"), ["GET", "POST", "GET", "POST", "POST", "POST", "POST", "POST"]);
  for (const index of [1, 3, 4, 5, 6, 7]) {
    assert.equal(calls[index].options.headers["x-weftext-csrf"], "same-origin");
  }
  assert.deepEqual(JSON.parse(calls[7].options.body), {
    source: "query source",
    blockIndex: 0,
    context: { today: { year: 2026, month: 8, day: 24 } },
  });
});

test("collaboration client validates versioned snapshots and keeps typed conflicts", async () => {
  const clientId = "22222222-2222-4222-8222-222222222222";
  const operationId = "33333333-3333-4333-8333-333333333333";
  const actorId = "44444444-4444-4444-8444-444444444444";
  const collaborationState = {
    wireVersion: COLLABORATION_WIRE_VERSION,
    epoch: 2,
    version: 7,
    revision: "a".repeat(64),
    frozen: false,
  };
  const snapshot = {
    wireVersion: COLLABORATION_WIRE_VERSION,
    nodeId,
    actorId,
    state: collaborationState,
    source: "= 文档\n",
    participants: [{
      actorId, clientId, role: "editor", cursor: 0, selectionStart: 0, selectionEnd: 0, expiresAt: 100,
    }],
  };
  assert.equal(requireCollaborationSnapshot(snapshot, nodeId), snapshot);
  assert.equal(createCollaborationClientId(() => clientId), clientId);
  assert.throws(() => createCollaborationClientId(() => "not-a-uuid"), /client UUID/);

  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    if (path.endsWith("/drafts")) {
      return jsonResponse({
        wireVersion: COLLABORATION_WIRE_VERSION,
        status: "conflict",
        nodeId,
        actorId,
        clientId,
        operationId,
        transactionId: "",
        requestBaseRevision: "0".repeat(64),
        requestBaseVersion: 6,
        appliedBaseRevision: "a".repeat(64),
        appliedBaseVersion: 7,
        resultRevision: "a".repeat(64),
        state: { ...collaborationState, frozen: true, reason: "overlapping_concurrent_edit" },
        transformed: false,
        operations: [],
        auditRecorded: false,
        errorCode: "collaboration_conflict",
      }, 409);
    }
    return jsonResponse(snapshot);
  };
  assert.equal((await serverApi.collaborationSnapshot(nodeId)).state.version, 7);
  const conflict = await serverApi.commitCollaborationDraft(nodeId, {
    wireVersion: COLLABORATION_WIRE_VERSION,
    clientId,
    operationId,
    epoch: 2,
    baseVersion: 6,
    baseRevision: "0".repeat(64),
    source: "dirty",
  });
  assert.equal(requireCollaborationOperation(conflict, nodeId).status, "conflict");
  assert.equal(conflict.state.frozen, true);
  assert.deepEqual(calls.map(({ path }) => path), [
    `/api/v1/collaboration/documents/${nodeId}`,
    `/api/v1/collaboration/documents/${nodeId}/drafts`,
  ]);
  assert.equal(calls[1].options.headers["x-weftext-csrf"], "same-origin");
});

test("product payload guards preserve typed permission-filtered projections", () => {
  const annotation = {
    nodeId,
    workspaceRevision: workspaceScopeA,
    revision: "a".repeat(64),
    store: { version: 3, document_id: nodeId, annotations: [] },
  };
  const tasks = { nodeId, occurrences: [], diagnostics: [] };
  const preview = {
    planId,
    nodeId,
    baseWorkspaceRevision: workspaceScopeA,
    documentChanges: [],
    authoring: { proposedSource: "= Exact\n", assignedId: null },
  };
  const query = {
    valid: true,
    workspaceRevision: workspaceScopeA,
    execution: {
      blockIndex: 0,
      analysis: { blocks: [], diagnostics: [] },
      result: { columns: [{ outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }], rows: [], groups: [] },
      csv: "name\r\n",
    },
  };
  assert.equal(requireAnnotationReadPayload(annotation, nodeId), annotation);
  assert.equal(requireTaskInspectionPayload(tasks, nodeId), tasks);
  assert.equal(requireTaskPreviewPayload(preview, nodeId), preview);
  assert.deepEqual(taskPreviewConfirmation(preview, nodeId), {
    kind: "edit",
    proposedSource: "= Exact\n",
    generatedTaskIds: [],
  });
  assert.equal(requireQueryExecutionPayload(query), query);
  assert.throws(() => requireAnnotationReadPayload({ ...annotation, nodeId: planId }, nodeId), /批注 v3 投影/);
  assert.throws(() => requireTaskInspectionPayload({ ...tasks, occurrences: null }, nodeId), /任务检查投影/);
  assert.throws(() => requireTaskPreviewPayload({ ...preview, planId: "replayed-plan" }, nodeId), /任务事务预览/);
  assert.throws(() => requireTaskPreviewPayload({ ...preview, authoring: undefined }, nodeId), /任务事务预览/);
  assert.throws(() => requireQueryExecutionPayload({ ...query, execution: { ...query.execution, result: { rows: [] } } }), /查询结果表/);
});

test("CJK selections and structured task targets use Core UTF-8 identities", () => {
  const source = "A你🙂Z";
  assert.deepEqual(annotationTargetFromSelection("text_range", source, 1, 4), { kind: "text_range", start: 1, end: 8 });
  assert.deepEqual(annotationTargetFromSelection("insertion_point", source, 2, 2), { kind: "insertion_point", position: 4 });
  assert.deepEqual(annotationTargetFromSelection("block_at", source, 4, 4), { kind: "block_at", sourceOffset: 8 });
  assert.deepEqual(taskTargetForOccurrence({ task: { metadata: { id: nodeId }, range: { start: 7, end: 9 } } }), { kind: "id", id: nodeId });
  assert.deepEqual(taskTargetForOccurrence({ task: { metadata: null, range: { start: 7, end: 9 } } }), { kind: "occurrence", range: { start: 7, end: 9 } });
  assert.throws(() => taskTargetForOccurrence({ task: {} }), /Core UTF-8 range/);
});

test("surface contexts reject stale revisions, navigation, and async input races", () => {
  const workspace = { workspaceRevision: workspaceScopeA };
  const documentA = { nodeId, revision: "a".repeat(64) };
  const context = captureSurfaceContext(workspace, documentA, "first query");
  assert.equal(surfaceContextMatches(context, workspace, documentA, "first query"), true);
  assert.equal(surfaceContextMatches(context, { workspaceRevision: workspaceScopeB }, documentA, "first query"), false);
  assert.equal(surfaceContextMatches(context, workspace, { ...documentA, nodeId: planId }, "first query"), false);
  assert.equal(surfaceContextMatches(context, workspace, { ...documentA, revision: "b".repeat(64) }, "first query"), false);
  assert.equal(surfaceContextMatches(context, workspace, documentA, "second query"), false);
});

test("API client uses only versioned same-origin UUID routes", async () => {
  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    return { ok: true, status: 200, json: async () => ({ ok: true }) };
  };
  await serverApi.openDocument("550e8400-e29b-41d4-a716-446655440000");
  await serverApi.preview("550e8400-e29b-41d4-a716-446655440000", "a".repeat(64), "source");
  await serverApi.commit("550e8400-e29b-41d4-a716-446655440000", "a".repeat(64), "source");
  assert.deepEqual(calls.map((call) => call.path), [
    "/api/v1/documents/550e8400-e29b-41d4-a716-446655440000",
    "/api/v1/documents/550e8400-e29b-41d4-a716-446655440000/preview",
    "/api/v1/documents/550e8400-e29b-41d4-a716-446655440000",
  ]);
  assert.deepEqual(calls.map((call) => call.options.method ?? "GET"), ["GET", "POST", "PUT"]);
  assert.equal(calls[1].options.headers["x-weftext-csrf"], "same-origin");
  assert.equal(calls[2].options.credentials, "same-origin");
});

test("authentication client restores, logs in, and logs out through same-origin CSRF requests", async () => {
  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    return jsonResponse({ authenticated: true, actorScope: actorScopeA });
  };
  await serverApi.session();
  await serverApi.login("owner", "password value");
  await serverApi.logout();
  assert.deepEqual(calls.map((call) => call.path), [
    "/api/v1/auth/session",
    "/api/v1/auth/login",
    "/api/v1/auth/logout",
  ]);
  assert.equal(calls[0].options.credentials, "same-origin");
  assert.equal(calls[1].options.headers["x-weftext-csrf"], "same-origin");
  assert.equal(calls[2].options.headers["x-weftext-csrf"], "same-origin");
});

test("permission client uses capability-protected member, ACL, and audit routes", async () => {
  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    return jsonResponse([]);
  };
  await serverApi.members();
  await serverApi.createMember("editor.one", "password value", "editor");
  await serverApi.updateMember(actorScopeA, "viewer", true);
  await serverApi.nodeAcl();
  await serverApi.setNodeAcl(actorScopeA, "550e8400-e29b-41d4-a716-446655440000", "read");
  await serverApi.audit();
  assert.deepEqual(calls.map((call) => call.path), [
    "/api/v1/admin/members",
    "/api/v1/admin/members",
    `/api/v1/admin/members/${actorScopeA}`,
    "/api/v1/admin/node-acl",
    "/api/v1/admin/node-acl",
    "/api/v1/admin/audit",
  ]);
  assert.deepEqual(calls.map((call) => call.options.method ?? "GET"), ["GET", "POST", "PUT", "GET", "PUT", "GET"]);
  assert.equal(calls[1].options.headers["x-weftext-csrf"], "same-origin");
  assert.equal(calls[4].options.headers["x-weftext-csrf"], "same-origin");
});

test("Owner backup client uses typed paired preview, commit, verify, restore, and drill routes", async () => {
  const calls = [];
  const workspaceId = "550e8400-e29b-41d4-a716-446655440000";
  const controlId = "11111111-1111-4111-8111-111111111111";
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    if (path.endsWith("/capabilities")) return jsonResponse({
      schema: "weftext.server-backup-capabilities.v1", ownerOnly: true,
      fullWorkspaceAndControlPlanePair: true, exclusiveLease: true, apiQuiescence: true,
      alternateCleanRestore: true, restoreDrill: true, managedShape: "X/X.adoc",
      sessionRestorePolicy: "invalidate_all",
      reverseProxySecretRestoreAction: "regenerate_and_rotate_at_first_server_start",
      annotations: "node_local_weftext.annotations.json",
    });
    if (path.endsWith("/preview")) {
      const restore = path.includes("/restore/") || path.includes("/drill/");
      return jsonResponse({
        stage: path.includes("/drill/") ? "drill_preview" : restore ? "restore_preview" : "preview",
        plan: restore
          ? { schema: "weftext.server-restore-pair-plan.v1", planDigest: "d".repeat(64), workspaceRestoreId: workspaceId, controlPlaneRestoreId: controlId }
          : { schema: "weftext.server-backup-pair-plan.v1", planDigest: "b".repeat(64), workspaceSnapshotId: workspaceId, controlPlaneBackupId: controlId },
        ...(restore ? { cleanTargetsRequired: true } : { quiesced: true }),
      });
    }
    if (path.endsWith("/backup/commit")) return jsonResponse({
      stage: "committed", quiesced: true, auditRecorded: true,
      receipt: {
        schema: "weftext.server-backup-pair-receipt.v1", planDigest: "b".repeat(64),
        workspaceSnapshotDirectory: "C:/backups/workspace",
        controlPlaneSnapshotDirectory: "C:/backups/control", complete: true,
        verification: { schema: "weftext.server-backup-pair-verification.v1", exactPair: true },
      },
    });
    if (path.endsWith("/backup/verify")) return jsonResponse({
      stage: "verified",
      verification: { schema: "weftext.server-backup-pair-verification.v1", exactPair: true },
    });
    if (path.endsWith("/restore/verify")) return jsonResponse({
      stage: "restore_verified",
      verification: { schema: "weftext.server-restore-pair-verification.v1", exactPair: true },
    });
    if (path.endsWith("/restore/commit") || path.endsWith("/drill/commit")) return jsonResponse({
      stage: path.includes("/drill/") ? "drill_completed" : "restored",
      cleanTargets: true,
      auditRecorded: true,
      receipt: {
        schema: "weftext.server-restore-pair-receipt.v1", planDigest: "d".repeat(64), complete: true,
        verification: { schema: "weftext.server-restore-pair-verification.v1", exactPair: true },
      },
    });
    return jsonResponse({ stage: "unexpected" });
  };
  await serverApi.backupCapabilities();
  await serverApi.previewServerBackup("C:/backups");
  await serverApi.commitServerBackup("b".repeat(64));
  await serverApi.verifyServerBackup("C:/backups/workspace", "C:/backups/control");
  const restoreBody = {
    workspaceSnapshotDirectory: "C:/backups/workspace",
    controlPlaneSnapshotDirectory: "C:/backups/control",
    restoredWorkspaceRoot: "C:/restore/Workspace",
    restoredControlPlaneRoot: "C:/restore/control",
  };
  await serverApi.previewServerRestore(restoreBody);
  await serverApi.commitServerRestore("d".repeat(64));
  await serverApi.verifyServerRestore(restoreBody);
  await serverApi.previewServerRestoreDrill({
    workspaceSnapshotDirectory: restoreBody.workspaceSnapshotDirectory,
    controlPlaneSnapshotDirectory: restoreBody.controlPlaneSnapshotDirectory,
    drillWorkspaceRoot: "C:/drill/Workspace",
    drillControlPlaneRoot: "C:/drill/control",
  });
  await serverApi.commitServerRestoreDrill("d".repeat(64));
  assert.deepEqual(calls.map(({ path }) => path), [
    "/api/v1/admin/backup/capabilities",
    "/api/v1/admin/backup/preview",
    "/api/v1/admin/backup/commit",
    "/api/v1/admin/backup/verify",
    "/api/v1/admin/restore/preview",
    "/api/v1/admin/restore/commit",
    "/api/v1/admin/restore/verify",
    "/api/v1/admin/backup/drill/preview",
    "/api/v1/admin/backup/drill/commit",
  ]);
  for (const call of calls.slice(1)) {
    assert.equal(call.options.headers["x-weftext-csrf"], "same-origin");
  }
  globalThis.fetch = async () => jsonResponse({
    stage: "committed", quiesced: false, auditRecorded: true,
    receipt: {
      schema: "weftext.server-backup-pair-receipt.v1", planDigest: "b".repeat(64),
      workspaceSnapshotDirectory: "C:/backups/workspace",
      controlPlaneSnapshotDirectory: "C:/backups/control", complete: true,
      verification: { schema: "weftext.server-backup-pair-verification.v1", exactPair: true },
    },
  });
  await assert.rejects(serverApi.commitServerBackup("b".repeat(64)), /备份提交回执/);
});

test("Trash client accepts only typed item summaries and sends intent rather than store paths", async () => {
  const itemId = "550e8400-e29b-41d4-a716-446655440000";
  const operationId = "6ba7b810-9dad-41d1-80b4-00c04fd430c8";
  const digest = "a".repeat(64);
  const item = {
    manifest: {
      schema: "weftext.trash-item/v1", trashItemId: itemId, operationId, kind: "resource",
      trashedAt: "2026-08-24T12:00:00+08:00", originStatus: "known",
      originalOwnerNodeId: nodeId, originalName: "figure.png", sha256: digest, byteLength: 2048,
    },
    containedNodeIds: [],
    restore: {
      originResolution: "active", originalAvailable: true, withAncestorsAvailable: false,
      requiredAncestorItemIds: [], blockedReason: null,
    },
  };
  const inventory = {
    workspaceRevision: "workspace-revision", state: "ready", legacyMigrationRequired: false, items: [item],
    reconciliation: { required: false, issueCount: 0 },
  };
  const plan = {
    planId, baseWorkspaceRevision: "workspace-revision", action: "trash_restore",
    scopeSummary: null,
    identityMap: [],
    capturedTarget: { kind: "trash_item", trashItemId: itemId, resolvedBy: "explicit_row" },
    targetNodeIds: [nodeId],
    draftSensitiveNodeIds: [nodeId],
    draftGate: {
      requiredCleanNodeIds: [nodeId], blockingDirtyNodeIds: [], observationDigest: "b".repeat(64),
    },
    trashItemChanges: [{
      disposition: "restored", manifest: item.manifest,
      destinationNodeId: nodeId, destinationName: "figure.png",
    }],
  };
  assert.equal(requireTrashInventoryPayload(inventory), inventory);
  assert.equal(requireTrashPlanPayload(plan), plan);
  assert.deepEqual(trashPermanentDeleteConfirmation([item]), [{
    trashItemId: itemId, payloadSha256: digest, payloadByteLength: 2048,
  }]);
  assert.throws(
    () => requireTrashInventoryPayload({ ...inventory, items: [{ ...item, itemPath: ".weftext-trash/_weftext.items/x" }] }),
    /Trash Item 摘要/,
  );

  const calls = [];
  globalThis.fetch = async (path, options = {}) => {
    calls.push({ path, options });
    return jsonResponse(path.endsWith("/trash") ? inventory : path.includes("/preview") ? plan : { committed: true });
  };
  await serverApi.trashInventory();
  await serverApi.previewTrashNode(nodeId, { baseWorkspaceRevision: "workspace-revision", trashedAt: "2026-08-24T12:00:00+08:00" });
  await serverApi.previewTrashResources({ baseWorkspaceRevision: "workspace-revision", trashedAt: "2026-08-24T12:00:00+08:00", resources: [{ ownerNodeId: nodeId, name: "figure.png" }] });
  await serverApi.previewTrashRestore(itemId, { baseWorkspaceRevision: "workspace-revision", mode: "original" });
  await serverApi.previewTrashPermanentDelete({ baseWorkspaceRevision: "workspace-revision", items: trashPermanentDeleteConfirmation([item]) });
  await serverApi.previewTrashLegacyMigration({ baseWorkspaceRevision: "workspace-revision", trashedAt: "2026-08-24T12:00:00+08:00" });
  await serverApi.commitTrash(planId);
  assert.deepEqual(calls.map(({ path }) => path), [
    "/api/v1/trash",
    `/api/v1/trash/nodes/${nodeId}/preview`,
    "/api/v1/trash/resources/preview",
    `/api/v1/trash/items/${itemId}/restore/preview`,
    "/api/v1/trash/permanent-delete/preview",
    "/api/v1/trash/migrate-legacy/preview",
    `/api/v1/trash/transactions/${planId}/commit`,
  ]);
  assert.equal(calls.every(({ path }) => !path.includes("_weftext.items") && !path.includes("payload")), true);
  assert.equal(calls.slice(1).every(({ options }) => options.headers["x-weftext-csrf"] === "same-origin"), true);
});

test("structured stale conflict is preserved for the UI", async () => {
  globalThis.fetch = async () => ({
    ok: false,
    status: 409,
    json: async () => ({ error: { code: "stale_revision", message: "stale", conflict: { actualRevision: "b".repeat(64) } } }),
  });
  await assert.rejects(serverApi.commit("550e8400-e29b-41d4-a716-446655440000", "a".repeat(64), "source"), (error) => {
    assert.ok(error instanceof ApiFailure);
    assert.equal(error.status, 409);
    assert.equal(error.code, "stale_revision");
    assert.equal(error.conflict.actualRevision, "b".repeat(64));
    return true;
  });
});

test("non-JSON failures preserve HTTP status and fallback message", async () => {
  globalThis.fetch = async () => ({
    ok: false,
    status: 400,
    text: async () => "plain rejection",
  });
  await assert.rejects(serverApi.search("query"), (error) => {
    assert.ok(error instanceof ApiFailure);
    assert.equal(error.status, 400);
    assert.equal(error.code, "unknown_error");
    assert.equal(error.message, "plain rejection");
    return true;
  });
});

test("preview consumes Core blocks and excludes frontmatter", () => {
  assert.deepEqual(previewLines({ blocks: [
    { kind: "frontmatter", text: "secret" },
    { kind: "heading", text: "标题", headingLevel: 8 },
  ] }), [{ kind: "heading", text: "标题", level: 8, quoteDepth: null }]);
});

test("tree and search navigation preserve exact dirty source before leaving", () => {
  const currentDocument = { nodeId: "node-a", revision: "a".repeat(64), source: "server source" };
  for (const origin of ["tree", "search"]) {
    const saved = [];
    const allowed = safeguardOutgoingDraft({
      origin,
      currentDocument,
      workspaceScope: workspaceScopeA,
      actorScope: actorScopeA,
      targetNodeId: "node-b",
      source: "exact unsaved source\r\n",
      saveDraft: (draft) => { saved.push(draft); return true; },
      confirmDiscard: () => false,
    });
    assert.equal(allowed, true);
    assert.equal(saved.length, 1);
    assert.equal(saved[0].origin, origin);
    assert.equal(saved[0].source, "exact unsaved source\r\n");
    assert.equal(saved[0].baseRevision, currentDocument.revision);
  }
});

test("navigation is blocked when a dirty draft cannot be saved and discard is declined", () => {
  const request = {
    origin: "tree",
    currentDocument: { nodeId: "node-a", revision: "a".repeat(64), source: "disk" },
    workspaceScope: workspaceScopeA,
    actorScope: actorScopeA,
    targetNodeId: "node-b",
    source: "dirty",
    saveDraft: () => false,
    confirmDiscard: () => false,
  };
  assert.equal(safeguardOutgoingDraft(request), false);
  assert.equal(safeguardOutgoingDraft({ ...request, workspaceScope: null }), false);
});

test("late tree, search, preview, and commit responses cannot replace a newer context", () => {
  const gate = createLatestRequestGate();
  const treeRequest = gate.begin();
  const searchRequest = gate.begin();
  assert.equal(gate.isCurrent(treeRequest), false);
  assert.equal(gate.isCurrent(searchRequest), true);

  const documentA = { nodeId: "node-a", revision: "a".repeat(64), source: "A" };
  const documentB = { nodeId: "node-b", revision: "b".repeat(64), source: "B" };
  const previewOrCommit = captureMutationContext(documentA, "A dirty");
  assert.equal(mutationContextMatches(previewOrCommit, documentB, "B"), false);
  assert.equal(mutationContextMatches(previewOrCommit, documentA, "A changed again"), false);
  assert.equal(mutationContextMatches(previewOrCommit, documentA, "A dirty"), true);
  assert.equal(createDeviceDraft(documentA, "A", workspaceScopeA, actorScopeA), null);
  assert.equal(createDeviceDraft(documentA, "A dirty", workspaceScopeA, actorScopeA).source, "A dirty");
});

test("device drafts restore only in the same workspace and actor scopes across restart", () => {
  const documentPayload = { nodeId: "same-node", revision: "a".repeat(64), source: "server" };
  const draft = createDeviceDraft(documentPayload, "unsaved", workspaceScopeA, actorScopeA, 1234);
  const storage = new Map([
    [deviceDraftStorageKey(workspaceScopeA, actorScopeA), JSON.stringify({ "same-node": draft })],
    ["weftext.server-drafts.v2", JSON.stringify({ "same-node": { ...draft, version: 2, actorScope: undefined } })],
  ]);
  const restoreAfterRestart = (workspaceScope, actorScope) => {
    const records = JSON.parse(storage.get(deviceDraftStorageKey(workspaceScope, actorScope)) ?? "{}");
    const candidate = records[documentPayload.nodeId];
    return isDeviceDraftForWorkspace(candidate, workspaceScope, actorScope, documentPayload.nodeId)
      ? candidate
      : null;
  };

  assert.notEqual(deviceDraftStorageKey(workspaceScopeA, actorScopeA), deviceDraftStorageKey(workspaceScopeB, actorScopeA));
  assert.notEqual(deviceDraftStorageKey(workspaceScopeA, actorScopeA), deviceDraftStorageKey(workspaceScopeA, actorScopeB));
  assert.deepEqual(restoreAfterRestart(workspaceScopeA, actorScopeA), draft);
  assert.equal(restoreAfterRestart(workspaceScopeB, actorScopeA), null);
  assert.equal(restoreAfterRestart(workspaceScopeA, actorScopeB), null);
  assert.equal(isDeviceDraftForWorkspace({ ...draft, actorScope: undefined, version: 2 }, workspaceScopeA, actorScopeA, "same-node"), false);
});

test("subscription requests revision reconciliation and reports disconnects", () => {
  class FakeEventSource {
    constructor(path) {
      this.path = path;
      this.listeners = new Map();
      FakeEventSource.instance = this;
    }
    addEventListener(name, listener) {
      this.listeners.set(name, listener);
    }
    emit(name, data = "") {
      this.listeners.get(name)?.({ data });
    }
    close() {
      this.closed = true;
    }
  }
  globalThis.EventSource = FakeEventSource;
  const reasons = [];
  const changes = [];
  let disconnects = 0;
  const close = serverApi.subscribe({
    onChange: (change) => changes.push(change),
    onResync: (value) => reasons.push(value.reason),
    onDisconnect: () => { disconnects += 1; },
  });
  const events = FakeEventSource.instance;
  assert.equal(events.path, "/api/v1/changes");
  events.emit("open");
  events.emit("open");
  events.emit("resync-required", JSON.stringify({ reason: "lagged", missedEvents: 9 }));
  events.emit("node-committed", JSON.stringify({ nodeId: "node-a", revision: "c".repeat(64) }));
  events.emit("error");
  assert.deepEqual(reasons, ["connected", "reconnected", "lagged"]);
  assert.equal(changes[0].nodeId, "node-a");
  assert.equal(disconnects, 1);
  close();
  assert.equal(events.closed, true);
});

test("collaboration subscription exposes participant, conflict, annotation, and lag events", () => {
  class FakeEventSource {
    constructor(path) {
      this.path = path;
      this.listeners = new Map();
      FakeEventSource.instance = this;
    }
    addEventListener(name, listener) { this.listeners.set(name, listener); }
    emit(name, payload) { this.listeners.get(name)?.({ data: JSON.stringify(payload) }); }
    close() { this.closed = true; }
  }
  globalThis.EventSource = FakeEventSource;
  const received = [];
  const resync = [];
  const close = serverApi.subscribeCollaboration({
    onEvent: (event) => received.push(event.eventType),
    onResync: (event) => resync.push(event.reason),
  });
  const events = FakeEventSource.instance;
  assert.equal(events.path, "/api/v1/collaboration/events");
  for (const eventType of ["presence", "conflict", "annotation-mutated"]) {
    events.emit(eventType, { wireVersion: COLLABORATION_WIRE_VERSION, eventType });
  }
  events.emit("resync-required", { wireVersion: COLLABORATION_WIRE_VERSION, reason: "lagged" });
  assert.deepEqual(received, ["presence", "conflict", "annotation-mutated"]);
  assert.deepEqual(resync, ["lagged"]);
  close();
  assert.equal(events.closed, true);
});
