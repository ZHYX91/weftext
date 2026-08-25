const API_ROOT = "/api/v1";
const MANAGED_GENERATION = "ascii_doc_v1";
const MANAGED_PROFILE = "ascii_doc_v1";
const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const SHA256 = /^[0-9a-f]{64}$/;
export const COLLABORATION_WIRE_VERSION = "weftext.collaboration.v1";
export const SERVER_ROLES = ["owner", "admin", "editor", "commenter", "viewer"];
export const ROLE_CAPABILITY_KEYS = [
  "readVisibleContent",
  "editDocuments",
  "mutateStructure",
  "writeAnnotations",
  "permanentlyDelete",
  "manageMembers",
  "manageWorkspace",
];

export class ApiFailure extends Error {
  constructor(status, payload, fallbackText = "") {
    super(payload?.error?.message ?? (fallbackText.trim() || `Server request failed (${status})`));
    this.name = "ApiFailure";
    this.status = status;
    this.code = payload?.error?.code ?? "unknown_error";
    this.conflict = payload?.error?.conflict ?? null;
  }
}

export function requireCanonicalWorkspacePayload(payload) {
  if (
    payload?.documentFormat?.generation !== MANAGED_GENERATION ||
    payload.documentFormat.canonicalExtension !== "adoc" ||
    payload.documentFormat.mediaType !== "text/asciidoc"
  ) {
    throw new Error("Server 返回了非 canonical AsciiDoc 工作区，已停止打开");
  }
  return payload;
}

export function requireCanonicalDocumentPayload(payload) {
  if (payload?.profile?.profile !== MANAGED_PROFILE) {
    throw new Error("Server 返回了非 canonical AsciiDoc 托管文档，已停止编辑");
  }
  requireCanonicalNodeMetadata(payload.metadata);
  if (payload.metadata.id !== payload.nodeId) {
    throw new Error("Server 返回的文档身份与节点元数据不一致，已停止编辑");
  }
  if (!payload.properties || !Array.isArray(payload.properties.properties) || !Array.isArray(payload.properties.diagnostics)) {
    throw new Error("Server 返回了无效的 AsciiDoc 文档头属性投影，已停止编辑");
  }
  return payload;
}

export function requireCanonicalNodeMetadata(metadata) {
  const keys = [
    "schema", "id", "icon", "resolvedIcon", "aliases", "childSort", "childSortDirection",
    "siblingRank", "adjacentHeadingBody", "diagnostics",
  ];
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)
    || Object.keys(metadata).length !== keys.length
    || !keys.every((key) => Object.hasOwn(metadata, key))
    || metadata.schema !== "weftext.node-metadata.v1"
    || !UUID_V4.test(metadata.id)
    || (metadata.icon !== null && typeof metadata.icon !== "string")
    || !Array.isArray(metadata.aliases) || !metadata.aliases.every((alias) => typeof alias === "string")
    || !["name", "manual"].includes(metadata.childSort)
    || !["ascending", "descending"].includes(metadata.childSortDirection)
    || (metadata.siblingRank !== null && (!Number.isSafeInteger(metadata.siblingRank) || metadata.siblingRank <= 0))
    || ![null, "separate", "run_in"].includes(metadata.adjacentHeadingBody)
    || !Array.isArray(metadata.diagnostics)) {
    throw new Error("Server 返回了无效的 canonical 节点元数据投影，已停止编辑");
  }
  if (metadata.resolvedIcon !== null
    && (typeof metadata.resolvedIcon !== "object"
      || !["emoji", "built_in"].includes(metadata.resolvedIcon.kind)
      || metadata.resolvedIcon.value !== metadata.icon
      || typeof metadata.resolvedIcon.glyph !== "string")) {
    throw new Error("Server 返回了不一致的节点图标投影，已停止编辑");
  }
  return metadata;
}

export function requireRoleCapabilities(capabilities) {
  if (!capabilities || typeof capabilities !== "object" || Array.isArray(capabilities)
    || Object.keys(capabilities).length !== ROLE_CAPABILITY_KEYS.length
    || !ROLE_CAPABILITY_KEYS.every((key) => typeof capabilities[key] === "boolean")) {
    throw new Error("Server 返回了无效的角色能力契约，已停止受保护操作");
  }
  return capabilities;
}

export function requireRoleCapabilityMap(roleCapabilities) {
  if (!roleCapabilities || typeof roleCapabilities !== "object" || Array.isArray(roleCapabilities)
    || Object.keys(roleCapabilities).length !== SERVER_ROLES.length
    || !SERVER_ROLES.every((role) => Object.hasOwn(roleCapabilities, role))) {
    throw new Error("Server 返回了不完整的五角色能力矩阵");
  }
  for (const role of SERVER_ROLES) requireRoleCapabilities(roleCapabilities[role]);
  return roleCapabilities;
}

export function requireSessionCapabilities(session, roleCapabilities = null) {
  if (!session || session.authenticated !== true || !SERVER_ROLES.includes(session.role)) {
    throw new Error("Server 返回了无效的认证会话");
  }
  requireRoleCapabilities(session.capabilities);
  if (roleCapabilities !== null) {
    const advertised = requireRoleCapabilityMap(roleCapabilities)[session.role];
    if (ROLE_CAPABILITY_KEYS.some((key) => advertised[key] !== session.capabilities[key])) {
      throw new Error("认证会话能力与 Server 五角色能力矩阵不一致");
    }
  }
  if (!session.capabilities.readVisibleContent) {
    throw new Error("当前会话没有读取可见内容的能力");
  }
  return session;
}

export function productSurfaceAccess(session) {
  const capabilities = requireSessionCapabilities(session).capabilities;
  return {
    readAnnotations: capabilities.readVisibleContent,
    writeAnnotations: capabilities.writeAnnotations,
    readTasks: capabilities.readVisibleContent,
    writeTasks: capabilities.mutateStructure,
    executeQueries: capabilities.readVisibleContent,
    editDocuments: capabilities.editDocuments,
    mutateStructure: capabilities.mutateStructure,
    permanentlyDelete: capabilities.permanentlyDelete,
    manageMembers: capabilities.manageMembers,
    manageWorkspace: capabilities.manageWorkspace,
  };
}

export function requireServerBackupCapabilities(payload) {
  if (payload?.schema !== "weftext.server-backup-capabilities.v1"
    || payload.ownerOnly !== true
    || payload.fullWorkspaceAndControlPlanePair !== true
    || payload.exclusiveLease !== true
    || payload.apiQuiescence !== true
    || payload.alternateCleanRestore !== true
    || payload.restoreDrill !== true
    || payload.sessionRestorePolicy !== "invalidate_all"
    || payload.reverseProxySecretRestoreAction !== "regenerate_and_rotate_at_first_server_start"
    || payload.managedShape !== "X/X.adoc"
    || payload.annotations !== "node_local_weftext.annotations.json") {
    throw new Error("Server 返回了不完整的备份安全能力契约");
  }
  return payload;
}

export function requireServerBackupPreview(payload, stage) {
  const restoring = stage !== "preview";
  if (payload?.stage !== stage || !SHA256.test(payload.plan?.planDigest)
    || payload.plan.schema !== (restoring
      ? "weftext.server-restore-pair-plan.v1"
      : "weftext.server-backup-pair-plan.v1")
    || !UUID_V4.test(payload.plan.workspaceSnapshotId ?? payload.plan.workspaceRestoreId)
    || !UUID_V4.test(payload.plan.controlPlaneBackupId ?? payload.plan.controlPlaneRestoreId)
    || (!restoring && payload.quiesced !== true)
    || (restoring && payload.cleanTargetsRequired !== true)) {
    throw new Error("Server 返回了无效的成对备份或恢复预览");
  }
  return payload;
}

function requireExactPairVerification(verification, schema) {
  if (verification?.schema !== schema || verification.exactPair !== true) {
    throw new Error("Server 没有返回精确配对校验结果");
  }
  return verification;
}

export function requireServerBackupCommit(payload) {
  if (payload?.stage !== "committed" || payload.quiesced !== true
    || payload.auditRecorded !== true || payload.receipt?.complete !== true
    || payload.receipt.schema !== "weftext.server-backup-pair-receipt.v1"
    || !SHA256.test(payload.receipt.planDigest)
    || typeof payload.receipt.workspaceSnapshotDirectory !== "string"
    || typeof payload.receipt.controlPlaneSnapshotDirectory !== "string") {
    throw new Error("Server 返回了无效的成对备份提交回执");
  }
  requireExactPairVerification(
    payload.receipt.verification,
    "weftext.server-backup-pair-verification.v1",
  );
  return payload;
}

export function requireServerBackupVerification(payload) {
  if (payload?.stage !== "verified") throw new Error("Server 返回了错配的备份校验阶段");
  requireExactPairVerification(
    payload.verification,
    "weftext.server-backup-pair-verification.v1",
  );
  return payload;
}

export function requireServerRestoreCommit(payload, stage) {
  if (payload?.stage !== stage || payload.cleanTargets !== true
    || payload.auditRecorded !== true || payload.receipt?.complete !== true
    || payload.receipt.schema !== "weftext.server-restore-pair-receipt.v1"
    || !SHA256.test(payload.receipt.planDigest)) {
    throw new Error("Server 返回了无效的成对恢复或演练回执");
  }
  requireExactPairVerification(
    payload.receipt.verification,
    "weftext.server-restore-pair-verification.v1",
  );
  return payload;
}

export function requireServerRestoreVerification(payload) {
  if (payload?.stage !== "restore_verified") {
    throw new Error("Server 返回了错配的恢复校验阶段");
  }
  requireExactPairVerification(
    payload.verification,
    "weftext.server-restore-pair-verification.v1",
  );
  return payload;
}

const TRASH_ORIGIN_STATES = ["active", "in_trash", "missing", "unknown", "reconciliation_required"];

function requireTrashManifest(manifest) {
  const common = ["schema", "trashItemId", "operationId", "kind", "trashedAt", "originStatus", "originalName"];
  const kindKeys = manifest?.kind === "node"
    ? ["nodeId", "originalParentNodeId", "ancestorNodeIds", "payloadSha256", "payloadByteLength", "payloadEntryCount"]
    : manifest?.kind === "resource"
      ? ["originalOwnerNodeId", "sha256", "byteLength"]
      : [];
  const keys = [...common, ...kindKeys];
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)
    || Object.keys(manifest).length !== keys.length || !keys.every((key) => Object.hasOwn(manifest, key))
    || manifest.schema !== "weftext.trash-item/v1" || !UUID_V4.test(manifest.trashItemId)
    || !UUID_V4.test(manifest.operationId) || !["known", "unknown"].includes(manifest.originStatus)
    || typeof manifest.originalName !== "string" || !manifest.originalName
    || Number.isNaN(Date.parse(manifest.trashedAt))) {
    throw new Error("Server 返回了无效的 Trash Item manifest 投影");
  }
  if (manifest.kind === "node"
    && (!UUID_V4.test(manifest.nodeId)
      || (manifest.originalParentNodeId !== null && !UUID_V4.test(manifest.originalParentNodeId))
      || !Array.isArray(manifest.ancestorNodeIds) || !manifest.ancestorNodeIds.every((id) => UUID_V4.test(id))
      || !SHA256.test(manifest.payloadSha256) || !Number.isSafeInteger(manifest.payloadByteLength)
      || manifest.payloadByteLength < 0 || !Number.isSafeInteger(manifest.payloadEntryCount)
      || manifest.payloadEntryCount < 1)) {
    throw new Error("Server 返回了无效的节点 Trash Item manifest 投影");
  }
  if (manifest.kind === "resource"
    && ((manifest.originalOwnerNodeId !== null && !UUID_V4.test(manifest.originalOwnerNodeId))
      || !SHA256.test(manifest.sha256) || !Number.isSafeInteger(manifest.byteLength)
      || manifest.byteLength < 0)) {
    throw new Error("Server 返回了无效的资源 Trash Item manifest 投影");
  }
  return manifest;
}

export function requireTrashInventoryPayload(payload) {
  if (!payload || typeof payload.workspaceRevision !== "string"
    || !["ready", "legacy_migration_required", "reconciliation_required"].includes(payload.state)
    || typeof payload.legacyMigrationRequired !== "boolean" || !Array.isArray(payload.items)
    || !payload.reconciliation || typeof payload.reconciliation.required !== "boolean"
    || !Number.isSafeInteger(payload.reconciliation.issueCount) || payload.reconciliation.issueCount < 0
    || payload.legacyMigrationRequired !== (payload.state === "legacy_migration_required")
    || payload.reconciliation.required !== (payload.state === "reconciliation_required")
    || (payload.state !== "ready" && payload.items.length !== 0)) {
    throw new Error("Server 返回了无效的 Trash inventory 投影");
  }
  const seen = new Set();
  for (const item of payload.items) {
    if (!item || Object.keys(item).length !== 3 || !item.manifest || !item.restore
      || !Array.isArray(item.containedNodeIds)
      || !item.containedNodeIds.every((id) => UUID_V4.test(id))) {
      throw new Error("Server 返回了无效的 Trash Item 摘要");
    }
    requireTrashManifest(item.manifest);
    if (seen.has(item.manifest.trashItemId)) throw new Error("Server 返回了重复的 Trash Item ID");
    seen.add(item.manifest.trashItemId);
    if (Object.keys(item.restore).length !== 5
      || !TRASH_ORIGIN_STATES.includes(item.restore.originResolution)
      || typeof item.restore.originalAvailable !== "boolean"
      || typeof item.restore.withAncestorsAvailable !== "boolean"
      || !Array.isArray(item.restore.requiredAncestorItemIds)
      || !item.restore.requiredAncestorItemIds.every((id) => UUID_V4.test(id))
      || ![null, "string"].includes(item.restore.blockedReason === null ? null : typeof item.restore.blockedReason)) {
      throw new Error("Server 返回了无效的 Trash 恢复可用性投影");
    }
    if (new Set(item.containedNodeIds).size !== item.containedNodeIds.length) {
      throw new Error("Server 返回了重复的 Trash payload 节点身份");
    }
  }
  return payload;
}

export function requireTrashPlanPayload(payload) {
  if (!payload || !UUID_V4.test(payload.planId) || typeof payload.baseWorkspaceRevision !== "string"
    || typeof payload.action !== "string" || !Array.isArray(payload.trashItemChanges)
    || !Array.isArray(payload.identityMap) || !Array.isArray(payload.targetNodeIds)
    || !Array.isArray(payload.draftSensitiveNodeIds)
    || !payload.targetNodeIds.every((id) => UUID_V4.test(id))
    || !payload.draftSensitiveNodeIds.every((id) => UUID_V4.test(id))
    || new Set(payload.targetNodeIds).size !== payload.targetNodeIds.length
    || new Set(payload.draftSensitiveNodeIds).size !== payload.draftSensitiveNodeIds.length
    || payload.targetNodeIds.some((id, index) => id !== [...payload.targetNodeIds].sort()[index])
    || payload.draftSensitiveNodeIds.some((id, index) => id !== [...payload.draftSensitiveNodeIds].sort()[index])
    || !payload.draftGate || !Array.isArray(payload.draftGate.requiredCleanNodeIds)
    || !Array.isArray(payload.draftGate.blockingDirtyNodeIds)
    || payload.draftGate.blockingDirtyNodeIds.length !== 0
    || payload.draftGate.requiredCleanNodeIds.some((id, index) => id !== payload.draftSensitiveNodeIds[index])
    || payload.draftGate.requiredCleanNodeIds.length !== payload.draftSensitiveNodeIds.length
    || !SHA256.test(payload.draftGate.observationDigest)) {
    throw new Error("Server 返回了无效的 Trash 事务预览");
  }
  if (payload.capturedTarget !== null
    && (!payload.capturedTarget || !["node", "trash_item", "owned_resource"].includes(payload.capturedTarget.kind)
      || !["focused_pane", "explicit_row", "caller_explicit"].includes(payload.capturedTarget.resolvedBy))) {
    throw new Error("Server 返回了无效的 Trash 动作目标证据");
  }
  if (payload.scopeSummary !== null
    && (!payload.scopeSummary || !UUID_V4.test(payload.scopeSummary.rootNode?.nodeId)
      || typeof payload.scopeSummary.rootNode.displayName !== "string"
      || !["preserve", "rekey"].includes(payload.scopeSummary.identityPolicy)
      || !Array.isArray(payload.scopeSummary.affectedDocumentNodeIds)
      || !Array.isArray(payload.scopeSummary.rewrittenDocumentNodeIds))) {
    throw new Error("Server 返回了无效的 Trash 操作范围证据");
  }
  for (const change of payload.trashItemChanges) {
    if (!change || Object.keys(change).length !== 4
      || !["stored", "restored", "permanently_deleted", "migrated"].includes(change.disposition)
      || (change.destinationNodeId !== null && !UUID_V4.test(change.destinationNodeId))
      || (change.destinationName !== null && typeof change.destinationName !== "string")) {
      throw new Error("Server 返回了无效的 Trash 事务预览");
    }
    requireTrashManifest(change.manifest);
  }
  return payload;
}

export function trashPermanentDeleteConfirmation(items) {
  return items.map(({ manifest }) => ({
    trashItemId: manifest.trashItemId,
    payloadSha256: manifest.payloadSha256 ?? manifest.sha256,
    payloadByteLength: manifest.payloadByteLength ?? manifest.byteLength,
  }));
}

export function requireAnnotationReadPayload(payload, nodeId) {
  if (!payload || payload.nodeId !== nodeId || typeof payload.workspaceRevision !== "string"
    || typeof payload.revision !== "string" || !payload.store || payload.store.version !== 3
    || payload.store.document_id !== nodeId || !Array.isArray(payload.store.annotations)) {
    throw new Error("Server 返回了无效的批注 v3 投影");
  }
  return payload;
}

export function requireTaskInspectionPayload(payload, nodeId) {
  if (!payload || payload.nodeId !== nodeId || !Array.isArray(payload.occurrences)
    || !Array.isArray(payload.diagnostics)
    || !payload.occurrences.every((occurrence) => occurrence?.nodeId === nodeId
      && typeof occurrence.revision === "string"
      && occurrence.task && typeof occurrence.task.description === "string"
      && occurrence.task.range && Number.isSafeInteger(occurrence.task.range.start)
      && Number.isSafeInteger(occurrence.task.range.end)
      && occurrence.task.range.start >= 0
      && occurrence.task.range.end >= occurrence.task.range.start)) {
    throw new Error("Server 返回了无效的任务检查投影");
  }
  return payload;
}

export function requireTaskPreviewPayload(payload, nodeId) {
  const authoring = payload?.authoring;
  const completion = payload?.completion;
  const exactProjectionCount = Number(Boolean(authoring)) + Number(Boolean(completion));
  if (!payload || !UUID_V4.test(payload.planId) || payload.nodeId !== nodeId
    || typeof payload.baseWorkspaceRevision !== "string" || !Array.isArray(payload.documentChanges)
    || exactProjectionCount !== 1
    || !payload.documentChanges.every((change) => UUID_V4.test(change?.nodeId)
      && typeof change.path === "string"
      && typeof change.baseRevision === "string"
      && typeof change.nextRevision === "string"
      && Number.isSafeInteger(change.editCount)
      && change.editCount >= 0)
    || (authoring && (typeof authoring.proposedSource !== "string"
      || (authoring.assignedId !== null && authoring.assignedId !== undefined && !UUID_V4.test(authoring.assignedId))))
    || (completion && (typeof completion.proposedSource !== "string"
      || (completion.nextTaskId !== null && completion.nextTaskId !== undefined && !UUID_V4.test(completion.nextTaskId))))
    || (payload.dependencies !== undefined
      && (!Array.isArray(payload.dependencies) || !payload.dependencies.every((id) => UUID_V4.test(id))))) {
    throw new Error("Server 返回了无效的任务事务预览");
  }
  return payload;
}

export function taskPreviewConfirmation(payload, nodeId) {
  requireTaskPreviewPayload(payload, nodeId);
  const recurrence = Boolean(payload.completion);
  const dependencies = Array.isArray(payload.dependencies);
  return {
    kind: recurrence ? "recurrence" : dependencies ? "dependencies" : "edit",
    proposedSource: recurrence ? payload.completion.proposedSource : payload.authoring.proposedSource,
    generatedTaskIds: [payload.authoring?.assignedId, payload.completion?.nextTaskId].filter(Boolean),
  };
}

export function requireQueryExecutionPayload(payload) {
  const execution = payload?.execution;
  if (typeof payload?.valid !== "boolean" || typeof payload.workspaceRevision !== "string"
    || !execution || !Number.isSafeInteger(execution.blockIndex)
    || !execution.analysis || !Array.isArray(execution.analysis.blocks)
    || !Array.isArray(execution.analysis.diagnostics)
    || (execution.csv !== null && typeof execution.csv !== "string")) {
    throw new Error("Server 返回了无效的统一查询投影");
  }
  if (execution.result !== null
    && (!Array.isArray(execution.result.columns) || !Array.isArray(execution.result.rows)
      || !Array.isArray(execution.result.groups))) {
    throw new Error("Server 返回了无效的查询结果表");
  }
  if (execution.result !== null
    && (!execution.result.columns.every((column) => column && typeof column.outputName === "string"
      && typeof column.path === "string"
      && typeof column.field === "string" && typeof column.valueType === "string"
      && typeof column.nullable === "boolean"
      && (column.propertyKey === null || typeof column.propertyKey === "string"))
      || !execution.result.rows.every((row) => Array.isArray(row?.cells)
        && row.cells.every((cell) => cell?.column && typeof cell.column.outputName === "string"
          && typeof cell.column.path === "string"))
      || !execution.result.groups.every((group) => group?.column
        && typeof group.column.outputName === "string"
        && typeof group.column.path === "string"))) {
    throw new Error("Server 返回了无效的查询列身份");
  }
  return payload;
}

export function requireCollaborationState(state) {
  if (!state || state.wireVersion !== COLLABORATION_WIRE_VERSION
    || !Number.isSafeInteger(state.epoch) || state.epoch < 1
    || !Number.isSafeInteger(state.version) || state.version < 0
    || typeof state.revision !== "string" || typeof state.frozen !== "boolean"
    || (state.reason !== undefined && typeof state.reason !== "string")) {
    throw new Error("Server 返回了无效的实时协作状态");
  }
  return state;
}

export function requireCollaborationSnapshot(payload, nodeId) {
  if (!payload || payload.wireVersion !== COLLABORATION_WIRE_VERSION
    || payload.nodeId !== nodeId || !UUID_V4.test(payload.actorId)
    || typeof payload.source !== "string" || !Array.isArray(payload.participants)) {
    throw new Error("Server 返回了无效的实时协作快照");
  }
  requireCollaborationState(payload.state);
  for (const participant of payload.participants) {
    if (!UUID_V4.test(participant.actorId) || !UUID_V4.test(participant.clientId)
      || typeof participant.role !== "string" || !Number.isSafeInteger(participant.cursor)
      || !Number.isSafeInteger(participant.selectionStart)
      || !Number.isSafeInteger(participant.selectionEnd)
      || !Number.isSafeInteger(participant.expiresAt)) {
      throw new Error("Server 返回了无效的实时协作参与者");
    }
  }
  return payload;
}

export function requireCollaborationOperation(payload, nodeId) {
  if (!payload || payload.wireVersion !== COLLABORATION_WIRE_VERSION
    || payload.nodeId !== nodeId || !["accepted", "replayed", "conflict", "frozen", "resync_required"].includes(payload.status)
    || !UUID_V4.test(payload.actorId) || !UUID_V4.test(payload.clientId)
    || !UUID_V4.test(payload.operationId) || !Array.isArray(payload.operations)) {
    throw new Error("Server 返回了无效的实时协作操作结果");
  }
  requireCollaborationState(payload.state);
  return payload;
}

async function request(path, options = {}) {
  const method = options.method ?? "GET";
  const stateChanging = !["GET", "HEAD", "OPTIONS"].includes(method);
  const response = await fetch(`${API_ROOT}${path}`, {
    ...options,
    credentials: "same-origin",
    headers: {
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...(stateChanging ? { "x-weftext-csrf": "same-origin" } : {}),
      ...options.headers,
    },
  });
  const { payload, text } = await readResponsePayload(response);
  if (!response.ok) throw new ApiFailure(response.status, payload, text);
  return payload;
}

async function readResponsePayload(response) {
  if (typeof response.text === "function") {
    const text = await response.text();
    if (!text) return { payload: {}, text };
    try {
      return { payload: JSON.parse(text), text };
    } catch {
      return { payload: null, text };
    }
  }
  return { payload: await response.json(), text: "" };
}

async function collaborationRequest(path, options = {}) {
  const response = await fetch(`${API_ROOT}${path}`, {
    ...options,
    credentials: "same-origin",
    headers: {
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...(!["GET", "HEAD", "OPTIONS"].includes(options.method ?? "GET")
        ? { "x-weftext-csrf": "same-origin" }
        : {}),
      ...options.headers,
    },
  });
  const { payload, text } = await readResponsePayload(response);
  if (!response.ok && payload?.wireVersion !== COLLABORATION_WIRE_VERSION) {
    throw new ApiFailure(response.status, payload, text);
  }
  return payload;
}

export const serverApi = {
  health: () => request("/health"),
  capabilities: () => request("/capabilities"),
  bootstrap: (bootstrapSecret, password) =>
    request("/auth/bootstrap", {
      method: "POST",
      body: JSON.stringify({ bootstrapSecret, password }),
    }),
  login: (login, password) =>
    request("/auth/login", {
      method: "POST",
      body: JSON.stringify({ login, password }),
    }),
  session: () => request("/auth/session"),
  logout: () => request("/auth/logout", { method: "POST" }),
  revokeAll: () => request("/auth/revoke-all", { method: "POST" }),
  members: () => request("/admin/members"),
  createMember: (login, password, role) =>
    request("/admin/members", {
      method: "POST",
      body: JSON.stringify({ login, password, role }),
    }),
  updateMember: (actorScope, role, enabled) =>
    request(`/admin/members/${encodeURIComponent(actorScope)}`, {
      method: "PUT",
      body: JSON.stringify({ role, enabled }),
    }),
  nodeAcl: () => request("/admin/node-acl"),
  setNodeAcl: (actorScope, nodeId, access) =>
    request("/admin/node-acl", {
      method: "PUT",
      body: JSON.stringify({ actorScope, nodeId, access }),
    }),
  audit: () => request("/admin/audit"),
  backupCapabilities: async () => requireServerBackupCapabilities(
    await request("/admin/backup/capabilities"),
  ),
  previewServerBackup: async (backupParent) => requireServerBackupPreview(
    await request("/admin/backup/preview", {
      method: "POST",
      body: JSON.stringify({ backupParent }),
    }),
    "preview",
  ),
  commitServerBackup: async (planDigest) => requireServerBackupCommit(
    await request("/admin/backup/commit", {
      method: "POST",
      body: JSON.stringify({ planDigest }),
    }),
  ),
  verifyServerBackup: async (workspaceSnapshotDirectory, controlPlaneSnapshotDirectory) =>
    requireServerBackupVerification(await request("/admin/backup/verify", {
      method: "POST",
      body: JSON.stringify({ workspaceSnapshotDirectory, controlPlaneSnapshotDirectory }),
    })),
  previewServerRestore: async (body) => requireServerBackupPreview(
    await request("/admin/restore/preview", {
      method: "POST",
      body: JSON.stringify(body),
    }),
    "restore_preview",
  ),
  commitServerRestore: async (planDigest) => requireServerRestoreCommit(
    await request("/admin/restore/commit", {
      method: "POST",
      body: JSON.stringify({ planDigest }),
    }),
    "restored",
  ),
  verifyServerRestore: async (body) => requireServerRestoreVerification(
    await request("/admin/restore/verify", {
      method: "POST",
      body: JSON.stringify(body),
    }),
  ),
  previewServerRestoreDrill: async (body) => requireServerBackupPreview(
    await request("/admin/backup/drill/preview", {
      method: "POST",
      body: JSON.stringify(body),
    }),
    "drill_preview",
  ),
  commitServerRestoreDrill: async (planDigest) => requireServerRestoreCommit(
    await request("/admin/backup/drill/commit", {
      method: "POST",
      body: JSON.stringify({ planDigest }),
    }),
    "drill_completed",
  ),
  inventory: () => request("/workspace"),
  trashInventory: async () => requireTrashInventoryPayload(await request("/trash")),
  previewTrashNode: async (nodeId, body) => requireTrashPlanPayload(
    await request(`/trash/nodes/${encodeURIComponent(nodeId)}/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  ),
  previewTrashResources: async (body) => requireTrashPlanPayload(
    await request("/trash/resources/preview", { method: "POST", body: JSON.stringify(body) }),
  ),
  previewTrashRestore: async (trashItemId, body) => requireTrashPlanPayload(
    await request(`/trash/items/${encodeURIComponent(trashItemId)}/restore/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  ),
  previewTrashPermanentDelete: async (body) => requireTrashPlanPayload(
    await request("/trash/permanent-delete/preview", { method: "POST", body: JSON.stringify(body) }),
  ),
  previewTrashLegacyMigration: async (body) => requireTrashPlanPayload(
    await request("/trash/migrate-legacy/preview", { method: "POST", body: JSON.stringify(body) }),
  ),
  commitTrash: (planId) => request(`/trash/transactions/${encodeURIComponent(planId)}/commit`, { method: "POST" }),
  openDocument: (nodeId) => request(`/documents/${encodeURIComponent(nodeId)}`),
  search: (query) => request(`/search?q=${encodeURIComponent(query)}`),
  annotations: (nodeId) => request(`/annotations/${encodeURIComponent(nodeId)}`),
  commitAnnotation: (nodeId, action) =>
    request(`/annotations/${encodeURIComponent(nodeId)}`, {
      method: "POST",
      body: JSON.stringify(action),
    }),
  inspectTasks: (nodeId) => request(`/tasks/nodes/${encodeURIComponent(nodeId)}`),
  previewTaskEdit: (nodeId, body) =>
    request(`/tasks/nodes/${encodeURIComponent(nodeId)}/edit/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  previewTaskRecurrence: (nodeId, body) =>
    request(`/tasks/nodes/${encodeURIComponent(nodeId)}/recurrence/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  previewTaskDependencies: (nodeId, body) =>
    request(`/tasks/nodes/${encodeURIComponent(nodeId)}/dependencies/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  commitTask: (planId) =>
    request(`/tasks/transactions/${encodeURIComponent(planId)}/commit`, { method: "POST" }),
  executeQuery: (source, blockIndex, context) =>
    request("/queries/execute", {
      method: "POST",
      body: JSON.stringify({ source, blockIndex, context }),
    }),
  preview: (nodeId, baseRevision, source) =>
    request(`/documents/${encodeURIComponent(nodeId)}/preview`, {
      method: "POST",
      body: JSON.stringify({ baseRevision, source }),
    }),
  commit: (nodeId, baseRevision, source) =>
    request(`/documents/${encodeURIComponent(nodeId)}`, {
      method: "PUT",
      body: JSON.stringify({ baseRevision, source }),
    }),
  collaborationSnapshot: async (nodeId) => requireCollaborationSnapshot(
    await collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}`),
    nodeId,
  ),
  commitCollaborationOperation: async (nodeId, operation) => requireCollaborationOperation(
    await collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}/operations`, {
      method: "POST",
      body: JSON.stringify(operation),
    }),
    nodeId,
  ),
  commitCollaborationDraft: async (nodeId, draft) => requireCollaborationOperation(
    await collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}/drafts`, {
      method: "POST",
      body: JSON.stringify(draft),
    }),
    nodeId,
  ),
  updateCollaborationPresence: (nodeId, presence) =>
    collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}/presence`, {
      method: "POST",
      body: JSON.stringify(presence),
    }),
  leaveCollaborationPresence: (nodeId, clientId) =>
    collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}/presence/${encodeURIComponent(clientId)}`, {
      method: "DELETE",
    }),
  acknowledgeCollaborationResync: (nodeId, resync) =>
    collaborationRequest(`/collaboration/documents/${encodeURIComponent(nodeId)}/resync`, {
      method: "POST",
      body: JSON.stringify(resync),
    }),
  subscribe: ({ onChange, onResync, onDisconnect = () => {} }) => {
    const events = new EventSource(`${API_ROOT}/changes`);
    let opened = false;
    events.addEventListener("open", () => {
      onResync({ reason: opened ? "reconnected" : "connected" });
      opened = true;
    });
    events.addEventListener("node-committed", (event) => {
      try {
        onChange(JSON.parse(event.data));
      } catch {
        onResync({ reason: "invalid-event" });
      }
    });
    events.addEventListener("resync-required", (event) => {
      try {
        onResync(JSON.parse(event.data));
      } catch {
        onResync({ reason: "invalid-resync-event" });
      }
    });
    events.addEventListener("error", () => onDisconnect());
    return () => events.close();
  },
  subscribeCollaboration: ({ onEvent, onResync, onDisconnect = () => {} }) => {
    const events = new EventSource(`${API_ROOT}/collaboration/events`);
    for (const name of ["operation-committed", "presence", "conflict", "external-edit", "resynced", "annotation-mutated"]) {
      events.addEventListener(name, (event) => {
        try {
          const payload = JSON.parse(event.data);
          if (payload.wireVersion !== COLLABORATION_WIRE_VERSION) throw new Error("wire version");
          onEvent(payload);
        } catch {
          onResync({ reason: "invalid-collaboration-event" });
        }
      });
    }
    events.addEventListener("resync-required", (event) => {
      try {
        onResync(JSON.parse(event.data));
      } catch {
        onResync({ reason: "invalid-resync-event" });
      }
    });
    events.addEventListener("error", () => onDisconnect());
    return () => events.close();
  },
};

export function createCollaborationClientId(randomUUID = globalThis.crypto?.randomUUID?.bind(globalThis.crypto)) {
  const value = randomUUID?.();
  if (!UUID_V4.test(value ?? "")) throw new Error("浏览器无法生成安全的实时协作 client UUID");
  return value;
}

export function createLatestRequestGate() {
  let generation = 0;
  return {
    begin() {
      generation += 1;
      return generation;
    },
    invalidate() {
      generation += 1;
    },
    isCurrent(token) {
      return token === generation;
    },
  };
}

export function captureMutationContext(documentPayload, source) {
  return documentPayload
    ? { nodeId: documentPayload.nodeId, revision: documentPayload.revision, source }
    : null;
}

export function mutationContextMatches(context, documentPayload, source) {
  return Boolean(
    context &&
      documentPayload &&
      context.nodeId === documentPayload.nodeId &&
      context.revision === documentPayload.revision &&
      context.source === source,
  );
}

export function captureSurfaceContext(workspacePayload, documentPayload, input = null) {
  return workspacePayload && documentPayload
    ? {
        nodeId: documentPayload.nodeId,
        revision: documentPayload.revision,
        workspaceRevision: workspacePayload.workspaceRevision,
        input,
      }
    : null;
}

export function surfaceContextMatches(context, workspacePayload, documentPayload, input = context?.input) {
  return Boolean(
    context
      && workspacePayload
      && documentPayload
      && context.nodeId === documentPayload.nodeId
      && context.revision === documentPayload.revision
      && context.workspaceRevision === workspacePayload.workspaceRevision
      && context.input === input,
  );
}

export function utf8ByteOffset(source, utf16Offset) {
  const bounded = Math.max(0, Math.min(source.length, Number(utf16Offset) || 0));
  return new TextEncoder().encode(source.slice(0, bounded)).length;
}

export function annotationTargetFromSelection(kind, source, selectionStart, selectionEnd) {
  const start = utf8ByteOffset(source, selectionStart);
  const end = utf8ByteOffset(source, selectionEnd);
  if (kind === "document") return { kind: "document" };
  if (kind === "text_range") return { kind: "text_range", start, end };
  if (kind === "insertion_point") return { kind: "insertion_point", position: start };
  if (kind === "block_at") return { kind: "block_at", sourceOffset: start };
  throw new Error("批注目标类型无效");
}

export function taskTargetForOccurrence(occurrence) {
  const task = occurrence?.task;
  if (task?.metadata?.id && UUID_V4.test(task.metadata.id)) {
    return { kind: "id", id: task.metadata.id };
  }
  if (!task?.range || !Number.isSafeInteger(task.range.start) || !Number.isSafeInteger(task.range.end)) {
    throw new Error("任务 occurrence 缺少 Core UTF-8 range");
  }
  return { kind: "occurrence", range: task.range };
}

export function deviceDraftStorageKey(workspaceScope, actorScope) {
  return /^[0-9a-f]{64}$/.test(workspaceScope ?? "") && /^[0-9a-f]{64}$/.test(actorScope ?? "")
    ? `weftext.server-drafts.v3.${workspaceScope}.${actorScope}`
    : null;
}

export function isDeviceDraftForWorkspace(draft, workspaceScope, actorScope, nodeId = draft?.nodeId) {
  return Boolean(
    deviceDraftStorageKey(workspaceScope, actorScope) &&
      draft?.version === 3 &&
      draft.workspaceScope === workspaceScope &&
      draft.actorScope === actorScope &&
      draft.nodeId === nodeId,
  );
}

export function createDeviceDraft(documentPayload, source, workspaceScope, actorScope, updatedAt = Date.now()) {
  if (!documentPayload || source === documentPayload.source || !deviceDraftStorageKey(workspaceScope, actorScope)) return null;
  return {
    version: 3,
    workspaceScope,
    actorScope,
    nodeId: documentPayload.nodeId,
    baseRevision: documentPayload.revision,
    source,
    updatedAt,
  };
}

export function safeguardOutgoingDraft({
  origin,
  currentDocument,
  workspaceScope,
  actorScope,
  targetNodeId,
  source,
  saveDraft,
}) {
  if (!currentDocument || currentDocument.nodeId === targetNodeId) return true;
  if (source === currentDocument.source) return true;
  if (!deviceDraftStorageKey(workspaceScope, actorScope)) return false;
  const draft = createDeviceDraft(currentDocument, source, workspaceScope, actorScope);
  draft.origin = origin;
  if (saveDraft(draft)) return true;
  return false;
}

export function previewLines(model) {
  return (model?.blocks ?? [])
    .filter((block) => block.kind !== "frontmatter")
    .map((block) => ({
      kind: block.kind,
      text: block.text,
      level: block.headingLevel ?? null,
      quoteDepth: block.quoteDepth ?? null,
    }));
}
