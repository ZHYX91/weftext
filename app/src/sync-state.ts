export interface ObjectKey { kind: "node" | "directory" | "rootDirectory" | "schema" | "settings" | "rules" | "tags"; id?: string }
export interface SyncIdentity { workspaceId: string; genesis: string }
export const sameSyncIdentity = (a: SyncIdentity, b: SyncIdentity): boolean => a.workspaceId === b.workspaceId && a.genesis === b.genesis;
export interface ObjectState { parent: ObjectKey | null; name: string; retired: boolean; files: Record<string, string>; fileRevisions: Record<string, string> }
export interface SyncVersion { commit: string; revision: string; state: ObjectState }
export interface SyncAdmission { status: string; missingParents?: string[]; missingBlobs?: string[]; reason?: string }
export interface ReceivedSyncVersion extends SyncVersion { parents: string[]; before: string[]; admission: SyncAdmission }
export interface ReceivedSyncVersions { versions: ReceivedSyncVersion[]; nextCommit: string | null; coverage: "complete" | "partial" }
export interface SyncObject { object: ObjectKey; view: { versions: SyncVersion[]; commonBases: SyncVersion[] } }
export interface SyncStatus {
  identity: SyncIdentity; heads: string[]; objects: SyncObject[];
  commits: { id: string; admission: SyncAdmission; unsettledDependencies: ObjectKey[] }[];
}
export interface SyncBinding {
  root: string; identity: SyncIdentity; replicaId: string; pendingPublications: number; partial: boolean;
  installedObjects: ObjectKey[]; installedVersions: [ObjectKey, string][]; selectedObjects: ObjectKey[] | null;
}
export interface SyncAdoptionCut {
  identity: SyncIdentity;
  expectedInstalled: { object: ObjectKey; revisions: string[] }[];
  changes: { path: string; baseId: string; before: number[]; after: number[] }[];
}
export interface SyncRequest { operationId: string; root: string; intent: { kind: "cloneSync" | "applySync"; identity: SyncIdentity; objects: ObjectKey[] } | { kind: "mergeSync"; identity: SyncIdentity; resolutions: { object: ObjectKey; revisions: string[]; state: ObjectState }[]; blobs: Record<string, number[]> } | ({ kind: "adoptSyncExternal" } & SyncAdoptionCut) }
export interface SyncPlan { operationId: string; planId: string; status: string; changes: { path: string; action: string; bytes: number }[]; losses: { message: string }[] }
export interface SyncPending { request: SyncRequest; plan?: SyncPlan; attempted: boolean; status: string }
export interface SyncFlowHost {
  request<T>(path: string, body: unknown): Promise<T>;
  store(pending: SyncPending | null): void;
}
export const objectKey = (key: ObjectKey): string => `${key.kind}:${key.id ?? ""}`;

/** The durable request is stored before its first network effect. Status reads
 * never execute; a lost prepare can only retry the exact original request. */
export class SyncFlow {
  busy = false;
  constructor(private readonly host: SyncFlowHost, public pending: SyncPending | null = null) {}
  private retain(): void { this.host.store(this.pending); }
  async recover(operationId: string): Promise<void> {
    if (this.busy || this.pending) throw new Error("请先处理当前保留的原操作。");
    this.busy = true;
    try {
      const result = await this.host.request<{ request: SyncRequest; plan: SyncPlan }>("/api/sync/operation", { operationId });
      if (result.request.operationId !== operationId || result.plan.operationId !== operationId || !result.plan.planId) throw new Error("查回结果与原编号不一致。");
      this.pending = { request: result.request, plan: result.plan, status: result.plan.status, attempted: true }; this.retain();
    } finally { this.busy = false; }
  }
  async prepare(request: SyncRequest): Promise<void> {
    if (this.busy || this.pending) throw new Error("请先检查或结束原同步操作。");
    this.pending = { request: structuredClone(request), attempted: false, status: "preparing" };
    this.retain(); await this.retryPrepare();
  }
  async retryPrepare(): Promise<void> {
    const pending = this.pending;
    if (this.busy || !pending || pending.attempted) throw new Error("不能重新准备可能已经执行的操作，请检查原结果。");
    this.retain(); this.busy = true;
    try {
      const plan = await this.host.request<SyncPlan>("/api/operations/prepare", pending.request);
      this.acceptPlan(plan); this.retain();
    } finally { this.busy = false; }
  }
  private acceptPlan(plan: SyncPlan): void {
    const pending = this.pending!;
    if (plan.operationId !== pending.request.operationId || !plan.planId || (pending.plan && plan.planId !== pending.plan.planId)) throw new Error("响应与原同步准备不一致；原记录保留。");
    pending.plan = plan; pending.status = plan.status;
  }
  async inspect(): Promise<void> {
    const pending = this.pending; if (this.busy || !pending) return;
    this.busy = true;
    try {
      const result = await this.host.request<SyncPlan>("/api/operations/status", { operationId: pending.request.operationId });
      if (result.operationId !== pending.request.operationId) throw new Error("响应属于另一操作。");
      if (pending.plan && result.planId && result.planId !== pending.plan.planId) throw new Error("状态响应与原准备不一致。");
      if (result.status === "prepared") this.acceptPlan(result);
      else pending.status = result.status;
      this.retain();
    } finally { this.busy = false; }
  }
  async commit(): Promise<void> {
    const pending = this.pending;
    if (this.busy || !pending?.plan || pending.status !== "prepared") throw new Error("先读取并核对原准备结果。");
    pending.attempted = true; pending.status = "unknown"; this.retain(); this.busy = true;
    try {
      const result = await this.host.request<SyncPlan>("/api/operations/commit", { operationId: pending.request.operationId, planId: pending.plan.planId });
      if (result.operationId !== pending.request.operationId || result.planId !== pending.plan.planId) throw new Error("同步回执与原准备不一致。");
      pending.status = result.status; this.retain();
    } finally { this.busy = false; }
  }
  forget(): void {
    const p = this.pending;
    if (this.busy || (p && !["committed", "resolved"].includes(p.status) && p.attempted)) throw new Error("原操作可能有作用，请先检查原结果。");
    this.host.store(null); this.pending = null;
  }
  async cancelPrepared(): Promise<void> {
    const p = this.pending;
    if (this.busy || !p?.plan || p.status !== "prepared") throw new Error("请先检查原操作仍处于准备状态。");
    this.busy = true;
    try {
      const result = await this.host.request<SyncPlan>("/api/sync/cancel-prepared", { operationId: p.request.operationId, planId: p.plan.planId });
      if (result.operationId !== p.request.operationId || result.planId !== p.plan.planId) throw new Error("取消结果与原操作不一致。");
      p.status = result.status; this.retain();
    } finally { this.busy = false; }
  }
}
