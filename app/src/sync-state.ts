import { message as uiMessage, UiError } from "./ui-locale";
import { decodeCoreDescription, decodeDescriptionFields, exactKeys, type CoreDescription, type ErrorDescription } from "./ui-diagnostics";
import { decodeKnowledgeLoss, type KnowledgeLoss } from "./knowledge-state";
import { decodeApiErrorBody, type ApiErrorBody } from "./api-error";
export interface ObjectKey {
    kind: "node" | "directory" | "rootDirectory" | "schema" | "settings" | "rules" | "tags";
    id?: string;
}
export interface SyncIdentity {
    workspaceId: string;
    genesis: string;
}
export interface RemoteSyncProgress extends ErrorDescription {
    requestId: string;
    identity: SyncIdentity;
    status: "running" | "complete" | "failed" | "stopped";
    recordsDone: number;
    recordsTotal: number;
    bytesDone: number;
    bytesTotal: number;
    error: ApiErrorBody | null;
}
export const sameSyncIdentity = (a: SyncIdentity, b: SyncIdentity): boolean => a.workspaceId === b.workspaceId && a.genesis === b.genesis;
export interface ObjectState {
    parent: ObjectKey | null;
    name: string;
    retired: boolean;
    files: Record<string, string>;
    fileRevisions: Record<string, string>;
}
export interface SyncVersion {
    commit: string;
    revision: string;
    state: ObjectState;
}
export type SyncAdmission = { status: "accepted" } | { status: "staged"; missingParents: string[]; missingBlobs: string[] } | { status: "rejected"; reason: CoreDescription };
export interface ReceivedSyncVersion extends SyncVersion {
    parents: string[];
    before: string[];
    admission: SyncAdmission;
}
export interface ReceivedSyncVersions {
    versions: ReceivedSyncVersion[];
    nextCommit: string | null;
    coverage: "complete" | "partial";
}
export interface SyncObject {
    object: ObjectKey;
    view: {
        versions: SyncVersion[];
        commonBases: SyncVersion[];
    };
}
export interface SyncStatus {
    identity: SyncIdentity;
    heads: string[];
    objects: SyncObject[];
    commits: {
        id: string;
        admission: SyncAdmission;
        unsettledDependencies: ObjectKey[];
    }[];
}
export interface SyncBinding {
    root: string;
    identity: SyncIdentity;
    replicaId: string;
    pendingPublications: number;
    partial: boolean;
    installedObjects: ObjectKey[];
    installedVersions: [
        ObjectKey,
        string
    ][];
    selectedObjects: ObjectKey[] | null;
}
export interface SyncAdoptionCut {
    identity: SyncIdentity;
    expectedInstalled: {
        object: ObjectKey;
        revisions: string[];
    }[];
    changes: {
        path: string;
        baseId: string;
        before: number[];
        after: number[];
    }[];
    removedMembers?: {
        path: string;
        baseId: string;
    }[];
}
export interface SyncDependency {
    object: ObjectKey;
    revisions: string[];
}
export interface SyncRevalidation {
    object: ObjectKey;
    revisions: string[];
    state: ObjectState;
    reviewedDependencies: SyncDependency[];
    decision: {
        kind: "preserve";
        revision: string;
    } | {
        kind: "repair";
        baseRevision: string;
    };
}
export interface SyncRevalidationCut {
    expectedInstalled: SyncDependency[];
    reviewedVersions: SyncDependency[];
    objects: (SyncObject & {
        originalDependencies: [
            string,
            SyncDependency[]
        ][];
        unsettledDependencies: ObjectKey[];
    })[];
    files: {
        object: ObjectKey;
        objectRevision: string;
        fileRevision: string;
        relativePath: string;
        digest: string;
    }[];
    coverage: string;
}
export interface SyncAnnotationTargetStatus {
    owner: {
        workspaceId: string;
        nodeId: string;
    };
    annotationId: string;
    state: string;
    reason: string;
}
export interface SyncRevalidationReview {
    operationId?: string;
    expectedInstalled: SyncDependency[];
    reviewedVersions: SyncDependency[];
    resolutions: SyncRevalidation[];
    annotationTargets: SyncAnnotationTargetStatus[];
}
export interface SyncRevalidationInput {
    operationId?: string;
    root: string;
    identity: SyncIdentity;
    resolutions: SyncRevalidation[];
    blobs: Record<string, number[]>;
}
export interface SyncRequest {
    operationId: string;
    root: string;
    intent: {
        kind: "cloneSync" | "applySync";
        identity: SyncIdentity;
        objects: ObjectKey[];
    } | {
        kind: "mergeSync";
        identity: SyncIdentity;
        resolutions: {
            object: ObjectKey;
            revisions: string[];
            state: ObjectState;
        }[];
        blobs: Record<string, number[]>;
    } | ({
        kind: "adoptSyncExternal";
    } & SyncAdoptionCut) | ({
        kind: "revalidateSync";
        identity: SyncIdentity;
        blobs: Record<string, number[]>;
    } & Omit<SyncRevalidationReview, "operationId" | "annotationTargets">);
}
export interface SyncPlan {
    operationId: string;
    planId: string;
    kind: SyncRequest["intent"]["kind"];
    status: SyncOperationStatus;
    changes: {
        path: string;
        action: string;
        bytes: number;
    }[];
    nodeMap: Record<string, string>;
    newNodeId: string | null;
    revisions: Record<string, string>;
    resourceMap: Record<string, string>;
    losses: KnowledgeLoss[];
}
export type SyncOperationStatus = "prepared" | "committed" | "recoveryPending" | "resolved";
export interface SyncReceipt { operationId: string; planId: string; status: SyncOperationStatus; path: string; nodeIds: string[]; kind?: SyncPlan["kind"]; affectedSource?: string; documentError?: ApiErrorBody; }
export interface SyncPending {
    request: SyncRequest;
    plan?: SyncPlan;
    receipt?: SyncReceipt;
    attempted: boolean;
    status: SyncOperationStatus | "preparing" | "unknown";
}
export interface SyncFlowHost {
    request<T>(path: string, body: unknown): Promise<T>;
    store(pending: SyncPending | null): void;
}
export const objectKey = (key: ObjectKey): string => `${key.kind}:${key.id ?? ""}`;
const invalidSyncWire = (): never => { throw Object.assign(new UiError(uiMessage("error.invalidResponse")), { code: "resultUnknown" }); };
function record(value: unknown): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) return invalidSyncWire(); return value as Record<string, unknown>; }
function keys(value: unknown, required: readonly string[], optional: readonly string[] = []): Record<string, unknown> { const item = record(value); if (!exactKeys(item, required, optional)) return invalidSyncWire(); return item; }
function text(value: unknown, nonempty = false): string { if (typeof value !== "string" || nonempty && !value) return invalidSyncWire(); return value; }
function bool(value: unknown): boolean { if (typeof value !== "boolean") return invalidSyncWire(); return value; }
function count(value: unknown): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) return invalidSyncWire(); return value; }
function array<T>(value: unknown, decode: (value: unknown) => T): T[] { if (!Array.isArray(value)) return invalidSyncWire(); return value.map(decode); }
const strings = (value: unknown): string[] => array(value, value => text(value));
function stringMap(value: unknown): Record<string, string> { return Object.fromEntries(Object.entries(record(value)).map(([key, value]) => [key, text(value)])); }
export function decodeSyncIdentity(value: unknown): SyncIdentity { const item = keys(value, ["workspaceId", "genesis"]); return { workspaceId: text(item.workspaceId, true), genesis: text(item.genesis, true) }; }
export function decodeRemoteSyncProgress(value: unknown): RemoteSyncProgress {
    const item = keys(value, ["requestId", "identity", "status", "recordsDone", "recordsTotal", "bytesDone", "bytesTotal", "explanation", "diagnostics", "causes", "error"]), description = decodeDescriptionFields(item), error = item.error === null ? null : decodeApiErrorBody(item.error);
    if (!description || error === undefined || typeof item.status !== "string" || !["running", "complete", "failed", "stopped"].includes(item.status)) return invalidSyncWire();
    return { requestId: text(item.requestId, true), identity: decodeSyncIdentity(item.identity), status: item.status as RemoteSyncProgress["status"], recordsDone: count(item.recordsDone), recordsTotal: count(item.recordsTotal), bytesDone: count(item.bytesDone), bytesTotal: count(item.bytesTotal), ...description, error };
}
export function decodeSyncObjectKey(value: unknown): ObjectKey {
    const item = record(value);
    if (typeof item.kind !== "string") return invalidSyncWire();
    const identified = ["node", "directory", "rootDirectory"].includes(item.kind);
    keys(item, identified ? ["kind", "id"] : ["kind"]);
    if (!["node", "directory", "rootDirectory", "schema", "settings", "rules", "tags"].includes(item.kind as string)) return invalidSyncWire();
    return { kind: item.kind as ObjectKey["kind"], ...(identified ? { id: text(item.id, true) } : {}) };
}
export function decodeSyncAdmission(value: unknown): SyncAdmission {
    const item = record(value);
    if (item.status === "accepted") { keys(item, ["status"]); return { status: "accepted" }; }
    if (item.status === "staged") { keys(item, ["status", "missingParents", "missingBlobs"]); return { status: "staged", missingParents: strings(item.missingParents), missingBlobs: strings(item.missingBlobs) }; }
    if (item.status === "rejected") { keys(item, ["status", "reason"]); return { status: "rejected", reason: decodeCoreDescription(item.reason) ?? invalidSyncWire() }; }
    return invalidSyncWire();
}
function decodeState(value: unknown): ObjectState { const item = keys(value, ["parent", "name", "retired", "files", "fileRevisions"]); return { parent: item.parent === null ? null : decodeSyncObjectKey(item.parent), name: text(item.name), retired: bool(item.retired), files: stringMap(item.files), fileRevisions: stringMap(item.fileRevisions) }; }
function decodeVersion(value: unknown): SyncVersion { const item = keys(value, ["commit", "revision", "state"]); return { commit: text(item.commit), revision: text(item.revision), state: decodeState(item.state) }; }
function decodeObject(value: unknown): SyncObject { const item = keys(value, ["object", "view"]), view = keys(item.view, ["versions", "commonBases"]); return { object: decodeSyncObjectKey(item.object), view: { versions: array(view.versions, decodeVersion), commonBases: array(view.commonBases, decodeVersion) } }; }
export function decodeSyncStatus(value: unknown): SyncStatus {
    const item = keys(value, ["identity", "heads", "objects", "commits"]);
    return { identity: decodeSyncIdentity(item.identity), heads: strings(item.heads), objects: array(item.objects, decodeObject), commits: array(item.commits, value => { const commit = keys(value, ["id", "admission", "unsettledDependencies"]); return { id: text(commit.id), admission: decodeSyncAdmission(commit.admission), unsettledDependencies: array(commit.unsettledDependencies, decodeSyncObjectKey) }; }) };
}
export function decodeSyncBinding(value: unknown): SyncBinding {
    const item = keys(value, ["root", "identity", "replicaId", "pendingPublications", "partial", "installedObjects", "installedVersions", "selectedObjects"]);
    return { root: text(item.root), identity: decodeSyncIdentity(item.identity), replicaId: text(item.replicaId), pendingPublications: count(item.pendingPublications), partial: bool(item.partial), installedObjects: array(item.installedObjects, decodeSyncObjectKey), installedVersions: array(item.installedVersions, value => { if (!Array.isArray(value) || value.length !== 2) return invalidSyncWire(); return [decodeSyncObjectKey(value[0]), text(value[1])] as [ObjectKey, string]; }), selectedObjects: item.selectedObjects === null ? null : array(item.selectedObjects, decodeSyncObjectKey) };
}
export function decodeReceivedSyncVersions(value: unknown): ReceivedSyncVersions {
    const item = keys(value, ["versions", "nextCommit", "coverage"]);
    if (item.coverage !== "complete" && item.coverage !== "partial") return invalidSyncWire();
    return { versions: array(item.versions, value => { const entry = keys(value, ["commit", "revision", "state", "parents", "before", "admission"]); return { commit: text(entry.commit), revision: text(entry.revision), state: decodeState(entry.state), parents: strings(entry.parents), before: strings(entry.before), admission: decodeSyncAdmission(entry.admission) }; }), nextCommit: item.nextCommit === null ? null : text(item.nextCommit), coverage: item.coverage };
}
const decodeDependency = (value: unknown): SyncDependency => { const item = keys(value, ["object", "revisions"]); return { object: decodeSyncObjectKey(item.object), revisions: strings(item.revisions) }; };
export function decodeSyncRevalidationCut(value: unknown): SyncRevalidationCut {
    const item = keys(value, ["expectedInstalled", "reviewedVersions", "objects", "files", "coverage"]);
    return { expectedInstalled: array(item.expectedInstalled, decodeDependency), reviewedVersions: array(item.reviewedVersions, decodeDependency), coverage: text(item.coverage), objects: array(item.objects, value => { const entry = keys(value, ["object", "view", "originalDependencies", "unsettledDependencies"]); return { ...decodeObject({ object: entry.object, view: entry.view }), originalDependencies: array(entry.originalDependencies, value => { if (!Array.isArray(value) || value.length !== 2) return invalidSyncWire(); return [text(value[0]), array(value[1], decodeDependency)] as [string, SyncDependency[]]; }), unsettledDependencies: array(entry.unsettledDependencies, decodeSyncObjectKey) }; }), files: array(item.files, value => { const file = keys(value, ["object", "objectRevision", "fileRevision", "relativePath", "digest"]); return { object: decodeSyncObjectKey(file.object), objectRevision: text(file.objectRevision), fileRevision: text(file.fileRevision), relativePath: text(file.relativePath), digest: text(file.digest) }; }) };
}
function decodeResolution(value: unknown): SyncRevalidation {
    const item = keys(value, ["object", "revisions", "state", "reviewedDependencies", "decision"]), decision = record(item.decision);
    if (decision.kind !== "preserve" && decision.kind !== "repair") return invalidSyncWire();
    keys(decision, decision.kind === "preserve" ? ["kind", "revision"] : ["kind", "baseRevision"]);
    return { object: decodeSyncObjectKey(item.object), revisions: strings(item.revisions), state: decodeState(item.state), reviewedDependencies: array(item.reviewedDependencies, decodeDependency), decision: decision.kind === "preserve" ? { kind: "preserve", revision: text(decision.revision) } : { kind: "repair", baseRevision: text(decision.baseRevision) } };
}
export function decodeSyncRevalidationReview(value: unknown): SyncRevalidationReview {
    const item = keys(value, ["expectedInstalled", "reviewedVersions", "resolutions", "annotationTargets"], ["operationId"]);
    return { ...(Object.hasOwn(item, "operationId") ? { operationId: text(item.operationId, true) } : {}), expectedInstalled: array(item.expectedInstalled, decodeDependency), reviewedVersions: array(item.reviewedVersions, decodeDependency), resolutions: array(item.resolutions, decodeResolution), annotationTargets: array(item.annotationTargets, value => { const target = keys(value, ["owner", "annotationId", "state", "reason"]), owner = keys(target.owner, ["workspaceId", "nodeId"]); return { owner: { workspaceId: text(owner.workspaceId), nodeId: text(owner.nodeId) }, annotationId: text(target.annotationId), state: text(target.state), reason: text(target.reason) }; }) };
}
export function decodeSyncPlan(value: unknown): SyncPlan {
    const item = keys(value, ["operationId", "planId", "kind", "status", "changes", "nodeMap", "newNodeId", "revisions", "resourceMap", "losses"]);
    if (!["cloneSync", "applySync", "mergeSync", "adoptSyncExternal", "revalidateSync"].includes(item.kind as string)) return invalidSyncWire();
    return { operationId: text(item.operationId, true), planId: text(item.planId, true), kind: item.kind as SyncPlan["kind"], status: operationStatus(item.status), changes: array(item.changes, value => { const change = keys(value, ["path", "action", "bytes"]); return { path: text(change.path), action: text(change.action), bytes: count(change.bytes) }; }), nodeMap: stringMap(item.nodeMap), newNodeId: item.newNodeId === null ? null : text(item.newNodeId), revisions: stringMap(item.revisions), resourceMap: stringMap(item.resourceMap), losses: array(item.losses, decodeKnowledgeLoss) };
}
function operationStatus(value: unknown): SyncOperationStatus { if (!["prepared", "committed", "recoveryPending", "resolved"].includes(value as string)) return invalidSyncWire(); return value as SyncOperationStatus; }
function decodeSyncReceipt(value: unknown, application = false): SyncReceipt {
    const item = keys(value, ["operationId", "planId", "status", "path", "nodeIds", ...(application ? ["kind"] : [])], application ? ["affectedSource", "documentError"] : []);
    if (application && (item.status !== "committed" || !["cloneSync", "applySync", "mergeSync", "adoptSyncExternal", "revalidateSync"].includes(item.kind as string))) return invalidSyncWire();
    return { operationId: text(item.operationId, true), planId: text(item.planId, true), status: operationStatus(item.status), path: text(item.path), nodeIds: strings(item.nodeIds), ...(application ? { kind: item.kind as SyncPlan["kind"] } : {}), ...(Object.hasOwn(item, "affectedSource") ? { affectedSource: text(item.affectedSource) } : {}), ...(Object.hasOwn(item, "documentError") ? { documentError: decodeApiErrorBody(item.documentError) ?? invalidSyncWire() } : {}) };
}
export type SyncStatusResult = { variant: "plan"; plan: SyncPlan } | { variant: "receipt"; receipt: SyncReceipt } | { variant: "fact"; operationId: string; status: "unknown" | "resolved" | "recoveryPending" };
/** The status route has three declared result forms; only its full committed receipt proves a write. */
export function decodeSyncOperationStatus(value: unknown): SyncStatusResult {
    const item = record(value);
    if (item.status === "committed") return { variant: "receipt", receipt: decodeSyncReceipt(item, true) };
    if (exactKeys(item, ["operationId", "status"]) && (item.status === "unknown" || item.status === "resolved" || item.status === "recoveryPending")) return { variant: "fact", operationId: text(item.operationId, true), status: item.status };
    return { variant: "plan", plan: decodeSyncPlan(item) };
}
function bytes(value: unknown): number[] { return array(value, value => { const byte = count(value); if (byte > 255) return invalidSyncWire(); return byte; }); }
function blobs(value: unknown): Record<string, number[]> { return Object.fromEntries(Object.entries(record(value)).map(([key, value]) => [key, bytes(value)])); }
export function decodeSyncRequest(value: unknown): SyncRequest {
    const item = keys(value, ["operationId", "root", "intent"]), intent = record(item.intent);
    const common = { operationId: text(item.operationId, true), root: text(item.root) }, identity = decodeSyncIdentity(intent.identity);
    if (intent.kind === "cloneSync" || intent.kind === "applySync") { keys(intent, ["kind", "identity", "objects"]); return { ...common, intent: { kind: intent.kind, identity, objects: array(intent.objects, decodeSyncObjectKey) } }; }
    if (intent.kind === "mergeSync") { keys(intent, ["kind", "identity", "resolutions", "blobs"]); return { ...common, intent: { kind: "mergeSync", identity, resolutions: array(intent.resolutions, value => { const r = keys(value, ["object", "revisions", "state"]); return { object: decodeSyncObjectKey(r.object), revisions: strings(r.revisions), state: decodeState(r.state) }; }), blobs: blobs(intent.blobs) } }; }
    if (intent.kind === "revalidateSync") { keys(intent, ["kind", "identity", "expectedInstalled", "reviewedVersions", "resolutions", "blobs"]); return { ...common, intent: { kind: "revalidateSync", identity, expectedInstalled: array(intent.expectedInstalled, decodeDependency), reviewedVersions: array(intent.reviewedVersions, decodeDependency), resolutions: array(intent.resolutions, decodeResolution), blobs: blobs(intent.blobs) } }; }
    if (intent.kind === "adoptSyncExternal") { keys(intent, ["kind", "identity", "expectedInstalled", "changes"], ["removedMembers"]); return { ...common, intent: { kind: "adoptSyncExternal", identity, expectedInstalled: array(intent.expectedInstalled, decodeDependency), changes: array(intent.changes, value => { const c = keys(value, ["path", "baseId", "before", "after"]); return { path: text(c.path), baseId: text(c.baseId), before: bytes(c.before), after: bytes(c.after) }; }), ...(Object.hasOwn(intent, "removedMembers") ? { removedMembers: array(intent.removedMembers, value => { const member = keys(value, ["path", "baseId"]); return { path: text(member.path), baseId: text(member.baseId) }; }) } : {}) } }; }
    return invalidSyncWire();
}
export function decodeSyncPending(value: unknown): SyncPending {
    const item = keys(value, ["request", "attempted", "status"], ["plan", "receipt"]), request = decodeSyncRequest(item.request), status = item.status === "preparing" || item.status === "unknown" ? item.status : operationStatus(item.status), plan = Object.hasOwn(item, "plan") ? decodeSyncPlan(item.plan) : undefined;
    const receipt = Object.hasOwn(item, "receipt") ? decodeSyncReceipt(item.receipt, Object.hasOwn(record(item.receipt), "kind")) : undefined;
    if (plan && (plan.operationId !== request.operationId || plan.kind !== request.intent.kind)) return invalidSyncWire();
    if (receipt && (receipt.operationId !== request.operationId || plan && receipt.planId !== plan.planId || receipt.kind !== undefined && receipt.kind !== request.intent.kind)) return invalidSyncWire();
    if (plan && (plan.status === "committed" || plan.status === "resolved") && status !== plan.status || receipt && (receipt.status === "committed" || receipt.status === "resolved") && status !== receipt.status) return invalidSyncWire();
    return { request, attempted: bool(item.attempted), status, ...(plan ? { plan } : {}), ...(receipt ? { receipt } : {}) };
}
export function freezeSyncAdoptionRequest(root: string, identity: SyncIdentity, cut: SyncAdoptionCut, operationId: string): SyncRequest {
    if (!sameSyncIdentity(cut.identity, identity))
        throw new UiError(uiMessage("syncState.adoptionLibraryMismatch"));
    return structuredClone({ operationId, root, intent: { kind: "adoptSyncExternal", identity: cut.identity, expectedInstalled: cut.expectedInstalled, changes: cut.changes,
            ...(cut.removedMembers === undefined ? {} : { removedMembers: cut.removedMembers }) } });
}
/** The durable request is stored before its first network effect. Status reads
 * never execute; a lost prepare can only retry the exact original request. */
export class SyncFlow {
    busy = false;
    pending: SyncPending | null;
    constructor(private readonly host: SyncFlowHost, pending: SyncPending | null = null) { this.pending = pending === null ? null : decodeSyncPending(pending); }
    private retain(): void { this.host.store(this.pending); }
    async recover(operationId: string): Promise<void> {
        if (this.busy || this.pending)
            throw new UiError(uiMessage("syncState.resolveRetainedOperationFirst"));
        this.busy = true;
        try {
            const incoming = keys(await this.host.request<unknown>("/api/sync/operation", { operationId }), ["request", "plan"]), request = decodeSyncRequest(incoming.request), plan = decodeSyncPlan(incoming.plan);
            if (request.operationId !== operationId || plan.operationId !== operationId || plan.kind !== request.intent.kind)
                throw new UiError(uiMessage("syncState.lookupIdMismatch"));
            this.pending = { request, plan, status: plan.status, attempted: true };
            this.retain();
        }
        finally {
            this.busy = false;
        }
    }
    async prepare(request: SyncRequest, onRetained?: () => void): Promise<void> {
        if (this.busy || this.pending)
            throw new UiError(uiMessage("syncState.resolveOriginalSyncFirst"));
        this.pending = { request: decodeSyncRequest(request), attempted: false, status: "preparing" };
        this.retain();
        onRetained?.();
        await this.retryPrepare();
    }
    async retryPrepare(): Promise<void> {
        const pending = this.pending;
        if (this.busy || !pending || pending.attempted)
            throw new UiError(uiMessage("syncState.possiblyExecutedCannotReprepare"));
        this.retain();
        this.busy = true;
        try {
            const plan = await this.host.request<unknown>("/api/operations/prepare", structuredClone(pending.request));
            this.acceptPlan(plan);
            this.retain();
        }
        finally {
            this.busy = false;
        }
    }
    private acceptPlan(value: unknown): void {
        const plan = decodeSyncPlan(value);
        const pending = this.pending!;
        if (plan.operationId !== pending.request.operationId || plan.kind !== pending.request.intent.kind || (pending.plan && plan.planId !== pending.plan.planId))
            throw new UiError(uiMessage("syncState.prepareResponseMismatch"));
        if (pending.plan) {
            const { status: previousStatus, ...previous } = pending.plan, { status: currentStatus, ...current } = plan;
            if (JSON.stringify(previous) !== JSON.stringify(current) || pending.status === "committed" && plan.status !== "committed")
                throw new UiError(uiMessage("syncState.statusPreparationMismatch"));
        }
        pending.plan = plan;
        pending.status = plan.status;
    }
    async inspect(): Promise<void> {
        const pending = this.pending;
        if (this.busy || !pending)
            return;
        this.busy = true;
        try {
            const result = decodeSyncOperationStatus(await this.host.request<unknown>("/api/operations/status", { operationId: pending.request.operationId }));
            const id = result.variant === "plan" ? result.plan.operationId : result.variant === "receipt" ? result.receipt.operationId : result.operationId;
            if (id !== pending.request.operationId)
                throw new UiError(uiMessage("syncState.responseOtherOperation"));
            if (result.variant === "plan") this.acceptPlan(result.plan);
            else if (result.variant === "receipt") this.acceptReceipt(result.receipt);
            else {
                if (pending.status === "committed") throw new UiError(uiMessage("syncState.statusPreparationMismatch"));
                pending.status = result.status;
            }
            this.retain();
        }
        finally {
            this.busy = false;
        }
    }
    async commit(): Promise<void> {
        const pending = this.pending;
        if (this.busy || !pending?.plan || pending.status !== "prepared")
            throw new UiError(uiMessage("syncState.readCheckOriginalPreparation"));
        pending.attempted = true;
        pending.status = "unknown";
        this.retain();
        this.busy = true;
        try {
            const result = decodeSyncReceipt(await this.host.request<unknown>("/api/operations/commit", { operationId: pending.request.operationId, planId: pending.plan.planId }), true);
            if (result.operationId !== pending.request.operationId || result.planId !== pending.plan.planId)
                throw new UiError(uiMessage("syncState.receiptPreparationMismatch"));
            this.acceptReceipt(result);
            this.retain();
        }
        finally {
            this.busy = false;
        }
    }
    forget(): void {
        const p = this.pending;
        if (this.busy || (p && !["committed", "resolved"].includes(p.status) && p.attempted))
            throw new UiError(uiMessage("syncState.originalMayHaveEffect"));
        this.host.store(null);
        this.pending = null;
    }
    private acceptReceipt(receipt: SyncReceipt): void {
        const pending = this.pending!;
        if (receipt.operationId !== pending.request.operationId || pending.plan && receipt.planId !== pending.plan.planId || receipt.kind !== undefined && receipt.kind !== pending.request.intent.kind || pending.status === "committed" && receipt.status !== "committed" || pending.receipt && (receipt.planId !== pending.receipt.planId || receipt.path !== pending.receipt.path || JSON.stringify(receipt.nodeIds) !== JSON.stringify(pending.receipt.nodeIds)))
            throw new UiError(uiMessage("syncState.receiptPreparationMismatch"));
        pending.status = receipt.status;
        pending.receipt = receipt;
    }
    async cancelPrepared(): Promise<void> {
        const p = this.pending;
        if (this.busy || !p?.plan || p.status !== "prepared")
            throw new UiError(uiMessage("syncState.checkOriginalPreparedState"));
        this.busy = true;
        try {
            const result = decodeSyncReceipt(await this.host.request<unknown>("/api/sync/cancel-prepared", { operationId: p.request.operationId, planId: p.plan.planId }));
            if (result.operationId !== p.request.operationId || result.planId !== p.plan.planId)
                throw new UiError(uiMessage("syncState.cancellationResultMismatch"));
            this.acceptReceipt(result);
            this.retain();
        }
        finally {
            this.busy = false;
        }
    }
}
