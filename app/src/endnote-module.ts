/** Fixed distribution resources use the existing Core import operation. This
 * helper never saves, rewrites or appends to a document's author source. */
export interface EndnoteDistribution { name: "weftext-endnotes.typ"; source: string; sha256: string; version: string }
export interface EndnoteModuleIdentity { scope: string; owner: string; binding: string; modulePath: string }
export interface EndnoteModuleResource { resourceRef: string; owner: string; modulePath: string }
export interface EndnoteModuleCatalog { coverage: "complete" | "partial"; owner: string; resources: EndnoteModuleResource[] }
export interface EndnoteModuleRead { resourceRef: string; source: string; sha256: string }
export interface EndnoteModuleRequest { operationId: string; identity: EndnoteModuleIdentity; module: EndnoteDistribution }
export interface EndnoteModuleReceipt {
  operationId: string; status: "notRegistered" | "notCommitted" | "prepared" | "committed" | "recoveryPending" | "resolved" | "cancelled";
  planId?: string | null; commitAllowed: boolean; resourceRefs?: string[]; losses?: string[];
}
export interface EndnoteModuleState {
  format: "weftext.endnote-module-install"; version: 1; request: EndnoteModuleRequest;
  phase: "claimed" | "preparing" | "prepared" | "committing" | "committed" | "checking" | "cancelled" | "resolved";
  receipt?: EndnoteModuleReceipt;
}
export interface EndnoteModuleReady { resourceRef: string; modulePath: string; sha256: string; version: string; reused: boolean }
export interface EndnoteModuleHost {
  identity(): EndnoteModuleIdentity | null;
  /** GET of public product bytes; no owner/document source is sent. */
  distribution(): Promise<EndnoteDistribution>;
  listResources(identity: EndnoteModuleIdentity): Promise<EndnoteModuleCatalog>;
  readResource(resource: EndnoteModuleResource, identity: EndnoteModuleIdentity): Promise<EndnoteModuleRead>;
  /** Recheck actual compiler dependency permission separately from resource read. */
  compilerInputAuthorized(resource: EndnoteModuleResource, identity: EndnoteModuleIdentity): Promise<boolean>;
  prepareImport(request: EndnoteModuleRequest): Promise<EndnoteModuleReceipt>;
  commitImport(request: { operationId: string; planId: string; identity: EndnoteModuleIdentity }): Promise<EndnoteModuleReceipt>;
  importStatus(request: { operationId: string; identity: EndnoteModuleIdentity }): Promise<EndnoteModuleReceipt>;
  settleImport?(request: { operationId: string; identity: EndnoteModuleIdentity }): Promise<EndnoteModuleReceipt>;
  /** Storage is scoped by state.request.identity, never by a later active tab. */
  load(identity: EndnoteModuleIdentity): unknown;
  persist(state: EndnoteModuleState): void;
  committed?(identity: EndnoteModuleIdentity): void;
  newOperationId?(): string;
}
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
const phases = new Set(["claimed", "preparing", "prepared", "committing", "committed", "checking", "cancelled", "resolved"]);
const hashPattern = /^[a-f0-9]{64}$/;
function validString(value: unknown, limit = 2048): value is string { return typeof value === "string" && value.length > 0 && value.length <= limit && !/[\u0000-\u001f\u007f]/.test(value); }
function validPath(path: unknown): path is string { return validString(path, 1024) && !path.includes("\\") && path.split("/").every(part => part.length > 0 && ![".", ".."].includes(part) && !part.includes(":")) && path.split("/").at(-1) === "weftext-endnotes.typ"; }
function validIdentity(identity: unknown): identity is EndnoteModuleIdentity {
  if (!identity || typeof identity !== "object") return false; const value = identity as EndnoteModuleIdentity;
  return validString(value.scope) && validString(value.owner) && validString(value.binding) && validPath(value.modulePath);
}
function sameIdentity(a: EndnoteModuleIdentity | null, b: EndnoteModuleIdentity): boolean { return !!a && a.scope === b.scope && a.owner === b.owner && a.binding === b.binding && a.modulePath === b.modulePath; }
function validDistribution(value: unknown): value is EndnoteDistribution {
  if (!value || typeof value !== "object") return false; const module = value as EndnoteDistribution;
  return module.name === "weftext-endnotes.typ" && typeof module.source === "string" && module.source.length > 0 && module.source.length <= 262144 && !module.source.includes("\0") && hashPattern.test(module.sha256) && validString(module.version, 120);
}
async function digest(source: string): Promise<string> { return Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source))), value => value.toString(16).padStart(2, "0")).join(""); }
export function decodeEndnoteModuleState(value: unknown): EndnoteModuleState {
  const state = typeof value === "string" ? JSON.parse(value) as EndnoteModuleState : value as EndnoteModuleState;
  if (!state || state.format !== "weftext.endnote-module-install" || state.version !== 1 || !phases.has(state.phase) || !state.request || !validIdentity(state.request.identity) || !validDistribution(state.request.module) || !validString(state.request.operationId, 256)) throw new Error("尾注模块原安装记录无法核对；保留原记录，未发送新的资源写入。");
  if (state.receipt) state.receipt = endnoteModuleReceipt(state.request, state.receipt);
  return structuredClone(state);
}
export function endnoteModuleReceipt(request: EndnoteModuleRequest, value: EndnoteModuleReceipt, previous?: EndnoteModuleReceipt): EndnoteModuleReceipt {
  if (!value || value.operationId !== request.operationId || !statuses.has(value.status) || typeof value.commitAllowed !== "boolean" || value.planId != null && !validString(value.planId, 256) || value.resourceRefs !== undefined && (!Array.isArray(value.resourceRefs) || value.resourceRefs.some(ref => !validString(ref))) || value.losses !== undefined && (!Array.isArray(value.losses) || value.losses.some(loss => typeof loss !== "string"))) throw new Error("尾注模块回执不对应原操作，未采用替代资源计划。");
  if (value.commitAllowed && (value.status !== "prepared" || !value.planId) || previous?.planId && value.planId && previous.planId !== value.planId || previous?.status === "committed" && value.status !== "committed") throw new Error("尾注模块原计划或提交事实已变化；请继续核对原编号。");
  return structuredClone(value);
}

export class EndnoteModuleInstaller {
  private busy = false;
  private epoch = 0;
  constructor(private host: EndnoteModuleHost) {}
  /** Invalidate future steps, without pretending an in-flight write was cancelled. */
  invalidate(): void { this.epoch++; }
  private identity(): EndnoteModuleIdentity { const identity = this.host.identity(); if (!validIdentity(identity)) throw new Error("当前没有明确归属和获权集合，未安装尾注模块。"); return structuredClone(identity); }
  private check(identity: EndnoteModuleIdentity, epoch: number): void { if (epoch !== this.epoch || !sameIdentity(this.host.identity(), identity)) throw new Error("尾注模块的当前归属、授权或执行依赖已变化；原操作保留，未继续提交。"); }
  private save(state: EndnoteModuleState): void { try { this.host.persist(structuredClone(state)); } catch { throw new Error("尾注模块恢复记录不可可靠保存；未发送下一项资源写入，请核对原操作。"); } }
  private loaded(identity: EndnoteModuleIdentity): EndnoteModuleState | null {
    const raw = this.host.load(identity); if (raw == null || raw === "") return null;
    const state = decodeEndnoteModuleState(raw); if (!sameIdentity(state.request.identity, identity)) throw new Error("保留的尾注模块原操作属于另一依据，未重放到当前笔记。"); return state;
  }
  private async exclusive<T>(run: (identity: EndnoteModuleIdentity, epoch: number) => Promise<T>): Promise<T> { if (this.busy) throw new Error("原尾注模块操作正在核对，请等待同一操作完成。"); this.busy = true; try { return await run(this.identity(), this.epoch); } finally { this.busy = false; } }
  private async distribution(identity: EndnoteModuleIdentity, epoch: number): Promise<EndnoteDistribution> {
    const module = await this.host.distribution(); this.check(identity, epoch);
    if (!validDistribution(module) || await digest(module.source) !== module.sha256) throw new Error("随附尾注模块字节或版本无法验证；未导入替代内容。");
    this.check(identity, epoch); return structuredClone(module);
  }
  private async existing(identity: EndnoteModuleIdentity, module: EndnoteDistribution, epoch: number): Promise<EndnoteModuleResource | null> {
    const catalog = await this.host.listResources(identity); this.check(identity, epoch);
    if (!catalog || catalog.owner !== identity.owner || !["complete", "partial"].includes(catalog.coverage) || !Array.isArray(catalog.resources) || catalog.resources.length > 5000) throw new Error("本篇模块资源目录无法完整核对，未安装新资源。");
    const matches = catalog.resources.filter(resource => resource.modulePath === identity.modulePath);
    if (matches.length > 1 || matches.some(resource => resource.owner !== identity.owner || !validString(resource.resourceRef))) throw new Error("尾注模块位置存在不明确的资源归属，未覆盖或另行创建。");
    if (!matches.length) { if (catalog.coverage !== "complete") throw new Error("资源目录只有部分覆盖，不能据此认定尾注模块不存在。"); return null; }
    const resource = matches[0], read = await this.host.readResource(resource, identity); this.check(identity, epoch);
    if (!read || read.resourceRef !== resource.resourceRef || typeof read.source !== "string" || read.source.length > 262144 || !hashPattern.test(read.sha256) || await digest(read.source) !== read.sha256) throw new Error("已有模块的准确资源字节无法验证，未覆盖原文件。");
    this.check(identity, epoch);
    if (read.source !== module.source || read.sha256 !== module.sha256) throw new Error("同一模块位置已有不同内容；原资源和正文保留，请显式选择其它位置或人工核对，未覆盖。");
    return resource;
  }
  private async ready(identity: EndnoteModuleIdentity, module: EndnoteDistribution, epoch: number, reused: boolean, receipt?: EndnoteModuleReceipt): Promise<EndnoteModuleReady> {
    const resource = await this.existing(identity, module, epoch);
    if (!resource) throw new Error("原资源操作已核对，但当前模块资源不可用；不会换编号重新安装。");
    if (receipt?.resourceRefs?.length && !receipt.resourceRefs.includes(resource.resourceRef)) throw new Error("当前同名资源不是原安装回执的资源，未按路径冒领。");
    const authorized = await this.host.compilerInputAuthorized(resource, identity); this.check(identity, epoch);
    if (!authorized) throw new Error("模块资源已存在，但当前编译尚无该依赖的读取权限；正文未保存，未再次安装。");
    return { resourceRef: resource.resourceRef, modulePath: resource.modulePath, sha256: module.sha256, version: module.version, reused };
  }
  private receive(state: EndnoteModuleState, value: EndnoteModuleReceipt): EndnoteModuleReceipt {
    const receipt = endnoteModuleReceipt(state.request, value, state.receipt);
    const committed = receipt.status === "committed" && state.receipt?.status !== "committed";
    state.receipt = receipt;
    if (["prepared", "committed", "cancelled", "resolved"].includes(receipt.status)) state.phase = receipt.status as EndnoteModuleState["phase"];
    else state.phase = "checking";
    try { this.save(state); }
    finally { if (committed) this.host.committed?.(structuredClone(state.request.identity)); }
    return receipt;
  }
  private async status(state: EndnoteModuleState, epoch: number): Promise<EndnoteModuleReceipt> {
    this.check(state.request.identity, epoch); state.phase = "checking"; this.save(state);
    const value = await this.host.importStatus({ operationId: state.request.operationId, identity: structuredClone(state.request.identity) });
    const receipt = this.receive(state, value); this.check(state.request.identity, epoch); return receipt;
  }
  private async prepare(state: EndnoteModuleState, epoch: number): Promise<EndnoteModuleReceipt> {
    this.check(state.request.identity, epoch); state.phase = "preparing"; this.save(state);
    const value = await this.host.prepareImport(structuredClone(state.request));
    const receipt = this.receive(state, value); this.check(state.request.identity, epoch); return receipt;
  }
  private async finish(state: EndnoteModuleState, epoch: number, receipt: EndnoteModuleReceipt): Promise<EndnoteModuleReady> {
    const { identity, module, operationId } = state.request;
    if (receipt.status === "prepared") {
      if (!receipt.commitAllowed || !receipt.planId || receipt.losses?.length) throw new Error("尾注模块原计划当前不可提交或含转换损失；保留原操作，未自动接受损失。");
      this.check(identity, epoch); state.phase = "committing"; this.save(state);
      receipt = this.receive(state, await this.host.commitImport({ operationId, identity: structuredClone(identity), planId: receipt.planId })); this.check(identity, epoch);
    }
    if (receipt.status !== "committed") throw new Error(receipt.status === "notRegistered" ? "原模块安装尚未登记；请明确按原编号重试，不会创建新操作。" : "原模块安装尚未确认完成，请核对原操作状态；没有重新生成或保存正文。");
    return this.ready(identity, module, epoch, false, receipt);
  }
  private async begin(identity: EndnoteModuleIdentity, epoch: number, previousOperation?: string): Promise<EndnoteModuleReady> {
    const module = await this.distribution(identity, epoch), existing = await this.existing(identity, module, epoch);
    if (existing) return this.ready(identity, module, epoch, true);
    this.check(identity, epoch); const operationId = this.host.newOperationId?.() ?? crypto.randomUUID();
    if (!validString(operationId, 256) || operationId === previousOperation) throw new Error("无法分配新的准确模块操作编号，未发送写入。");
    const state: EndnoteModuleState = { format: "weftext.endnote-module-install", version: 1, phase: "claimed", request: { operationId, identity, module } };
    this.save(state); return this.finish(state, epoch, await this.prepare(state, epoch));
  }
  /** Call only from the explicit module preparation action. */
  install(): Promise<EndnoteModuleReady> {
    return this.exclusive(async (identity, epoch) => {
      const prior = this.loaded(identity);
      if (prior) {
        if (await digest(prior.request.module.source) !== prior.request.module.sha256) throw new Error("原模块安装字节摘要不一致，保留恢复记录，未发起操作。");
        const receipt = await this.status(prior, epoch), distribution = await this.distribution(identity, epoch);
        if (distribution.sha256 !== prior.request.module.sha256 || distribution.source !== prior.request.module.source || distribution.version !== prior.request.module.version) throw new Error("随附模块版本已变化；原操作已核对，未用新模块替换原请求或继续旧安装。");
        return this.finish(prior, epoch, receipt);
      }
      return this.begin(identity, epoch);
    });
  }
  /** Status-only recovery never prepares a replacement operation. */
  recover(): Promise<EndnoteModuleReceipt | null> { return this.exclusive(async (identity, epoch) => { const state = this.loaded(identity); return state ? this.status(state, epoch) : null; }); }
  /** A separate explicit action may replace a client claim only after a fresh
   * server cancellation. The original server tombstone remains authoritative. */
  restartAfterCancelled(): Promise<EndnoteModuleReady> {
    return this.exclusive(async (identity, epoch) => {
      const prior = this.loaded(identity); if (!prior) throw new Error("没有已取消的原模块安装；请使用首次准备入口。");
      const receipt = await this.status(prior, epoch);
      if (receipt.status !== "cancelled") throw new Error("原操作尚未明确取消，不能分配新编号；请继续核对或重试原操作。");
      return this.begin(identity, epoch, prior.request.operationId);
    });
  }
  /** An explicit retry keeps the complete original operation ID and bytes. */
  retryOriginal(): Promise<EndnoteModuleReady> {
    return this.exclusive(async (identity, epoch) => {
      const state = this.loaded(identity); if (!state) throw new Error("没有待重试的原模块安装，请先显式准备。");
      if (await digest(state.request.module.source) !== state.request.module.sha256) throw new Error("原安装输入无法验证，未发起重试。");
      let receipt = await this.status(state, epoch);
      const distribution = await this.distribution(identity, epoch);
      if (distribution.sha256 !== state.request.module.sha256 || distribution.source !== state.request.module.source || distribution.version !== state.request.module.version) throw new Error("随附模块已变化，未用旧输入重新准备；原操作继续保留供核对。");
      if (receipt.status === "notRegistered" || receipt.status === "notCommitted") receipt = await this.prepare(state, epoch);
      return this.finish(state, epoch, receipt);
    });
  }
  settleOriginal(): Promise<EndnoteModuleReceipt> {
    return this.exclusive(async (identity, epoch) => {
      const state = this.loaded(identity); if (!state || !this.host.settleImport) throw new Error("原安装没有可用的明确结束入口，未假报取消。");
      this.check(identity, epoch); state.phase = "checking"; this.save(state);
      const receipt = this.receive(state, await this.host.settleImport({ operationId: state.request.operationId, identity })); this.check(identity, epoch); return receipt;
    });
  }
}
