import { EndnoteModuleInstaller, type EndnoteDistribution, type EndnoteModuleCatalog, type EndnoteModuleHost, type EndnoteModuleIdentity, type EndnoteModuleReady, type EndnoteModuleReceipt, type EndnoteModuleState } from "./endnote-module";

type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface AuthoringAssetsContext { sessionId: string; audience: string }
export interface AuthoringAssetsOptions {
  request: Request;
  prefix: string;
  context(): AuthoringAssetsContext | null;
  changed(ready?: EndnoteModuleReady): void | Promise<void>;
  storage?: Pick<Storage, "getItem" | "setItem">;
}
interface Catalog extends EndnoteModuleCatalog { identity: EndnoteModuleIdentity }
/** Session adapters only call Core resource APIs; no host file write or Save. */
export class AuthoringAssetsAdapter {
  private installers = new Set<EndnoteModuleInstaller>();
  private epoch = 0;
  constructor(private options: AuthoringAssetsOptions) {}
  invalidate(): void { this.epoch++; for (const installer of this.installers) installer.invalidate(); this.installers.clear(); }
  private async use<T>(run: (installer: EndnoteModuleInstaller) => Promise<T>): Promise<T> {
    const context = this.options.context(), epoch = this.epoch;
    if (!context?.sessionId || !context.audience) throw new Error("当前正文会话没有模块资源写入资格，未发起安装。");
    const check = () => { const now = this.options.context(); if (this.epoch !== epoch || !now || now.sessionId !== context.sessionId || now.audience !== context.audience) throw new Error("模块安装期间正文归属或授权已变化；原请求保留，未改写正文。"); };
    const catalog = await this.options.request<Catalog>(`${this.options.prefix}/catalog`, { sessionId: context.sessionId }); check();
    if (!catalog?.identity || !catalog.owner || catalog.identity.owner !== catalog.owner) throw new Error("模块资源目录没有准确的本篇所有者，未安装。");
    const selected = structuredClone(catalog.identity), storage = this.options.storage ?? localStorage;
    const key = `weftext-endnotes-install:${encodeURIComponent(context.audience)}:${encodeURIComponent(selected.scope)}:${encodeURIComponent(selected.owner)}:${encodeURIComponent(selected.modulePath)}`;
    const host: EndnoteModuleHost = {
      identity: () => { try { check(); return selected; } catch { return null; } },
      distribution: () => this.options.request<EndnoteDistribution>(this.options.prefix),
      listResources: async identity => this.options.request<Catalog>(`${this.options.prefix}/catalog`, { sessionId: context.sessionId, identity }),
      readResource: (resource, identity) => this.options.request(`${this.options.prefix}/read`, { sessionId: context.sessionId, identity, resourceRef: resource.resourceRef }),
      compilerInputAuthorized: async (resource, identity) => (await this.options.request<{ authorized: boolean }>(`${this.options.prefix}/authorized`, { sessionId: context.sessionId, identity, resourceRef: resource.resourceRef })).authorized === true,
      prepareImport: request => this.options.request(`${this.options.prefix}/prepare`, { sessionId: context.sessionId, ...request }),
      commitImport: request => this.options.request(`${this.options.prefix}/commit`, { sessionId: context.sessionId, ...request }),
      importStatus: request => this.options.request(`${this.options.prefix}/status`, { sessionId: context.sessionId, ...request }),
      settleImport: request => this.options.request(`${this.options.prefix}/settle`, { sessionId: context.sessionId, ...request }),
      load: () => storage.getItem(key),
      persist: (state: EndnoteModuleState) => { if (state.request.identity.scope !== selected.scope || state.request.identity.owner !== selected.owner) throw new Error("模块恢复记录归属不符。"); storage.setItem(key, JSON.stringify(state)); },
    };
    const installer = new EndnoteModuleInstaller(host); this.installers.add(installer);
    try { const value = await run(installer); check(); return value; }
    finally { this.installers.delete(installer); }
  }
  async install(): Promise<EndnoteModuleReady> { const ready = await this.use(installer => installer.install()); await this.options.changed(ready); return ready; }
  async retryOriginal(): Promise<EndnoteModuleReady> { const ready = await this.use(installer => installer.retryOriginal()); await this.options.changed(ready); return ready; }
  async restartAfterCancelled(): Promise<EndnoteModuleReady> { const ready = await this.use(installer => installer.restartAfterCancelled()); await this.options.changed(ready); return ready; }
  recover(): Promise<EndnoteModuleReceipt | null> { return this.use(installer => installer.recover()); }
  settleOriginal(): Promise<EndnoteModuleReceipt> { return this.use(installer => installer.settleOriginal()); }
}
