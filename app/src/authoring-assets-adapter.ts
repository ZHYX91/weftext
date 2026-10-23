import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { EndnoteModuleInstaller, decodeEndnoteModuleState, type EndnoteDistribution, type EndnoteModuleCatalog, type EndnoteModuleHost, type EndnoteModuleIdentity, type EndnoteModuleReady, type EndnoteModuleReceipt, type EndnoteModuleState } from "./endnote-module";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface AuthoringAssetsContext {
    sessionId: string;
    audience: string;
    root?: string;
}
export interface AuthoringAssetsOptions {
    request: Request;
    prefix: string;
    context(): AuthoringAssetsContext | null;
    changed(ready?: EndnoteModuleReady): void | Promise<void>;
    committed?(context: AuthoringAssetsContext): void;
    storage?: Pick<Storage, "getItem" | "setItem">;
}
interface Catalog extends EndnoteModuleCatalog {
    identity: EndnoteModuleIdentity;
}
/** Session adapters only call Core resource APIs; no host file write or Save. */
export class AuthoringAssetsAdapter {
    private installers = new Set<EndnoteModuleInstaller>();
    private epoch = 0;
    private closeFrozen = false;
    private closeWork = 0;
    private unresolved = new Set<string>();
    constructor(private options: AuthoringAssetsOptions) { }
    assertCloseReady(): void {
        if (this.closeWork || this.installers.size || this.unresolved.size)
            throw new UiError(uiMessage("endnoteModule.originalInstallUnconfirmed"));
    }
    freezeForClose(): boolean {
        if (this.closeFrozen) return false;
        try { this.assertCloseReady(); } catch { return false; }
        this.closeFrozen = true;
        return true;
    }
    thawAfterClose(): void { this.closeFrozen = false; }
    invalidate(): void {
        this.epoch++;
        for (const installer of this.installers)
            installer.invalidate();
        this.installers.clear();
    }
    private async use<T>(run: (installer: EndnoteModuleInstaller) => Promise<T>): Promise<T> {
        if (this.closeFrozen) throw new UiError(uiMessage("endnoteModule.originalInstallUnconfirmed"));
        this.closeWork++;
        try {
        const context = this.options.context(), epoch = this.epoch;
        if (!context?.sessionId || !context.audience)
            throw new UiError(uiMessage("authoringAssets.moduleWriteNotPermitted"));
        const check = () => {
            const now = this.options.context();
            if (this.epoch !== epoch || !now || now.sessionId !== context.sessionId || now.audience !== context.audience)
                throw new UiError(uiMessage("authoringAssets.moduleOwnerOrAuthorityChanged"));
        };
        const catalog = await this.options.request<Catalog>(`${this.options.prefix}/catalog`, { sessionId: context.sessionId });
        check();
        if (!catalog?.identity || !catalog.owner || catalog.identity.owner !== catalog.owner)
            throw new UiError(uiMessage("authoringAssets.moduleOwnerMissing"));
        const selected = structuredClone(catalog.identity), storage = this.options.storage ?? localStorage;
        const key = `weftext-endnotes-install:${encodeURIComponent(context.audience)}:${encodeURIComponent(selected.scope)}:${encodeURIComponent(selected.owner)}:${encodeURIComponent(selected.modulePath)}`;
        const host: EndnoteModuleHost = {
            identity: () => {
                try {
                    check();
                    return selected;
                }
                catch {
                    return null;
                }
            },
            distribution: () => this.options.request<EndnoteDistribution>(this.options.prefix),
            listResources: async (identity) => this.options.request<Catalog>(`${this.options.prefix}/catalog`, { sessionId: context.sessionId, identity }),
            readResource: (resource, identity) => this.options.request(`${this.options.prefix}/read`, { sessionId: context.sessionId, identity, resourceRef: resource.resourceRef }),
            compilerInputAuthorized: async (resource, identity) => (await this.options.request<{
                authorized: boolean;
            }>(`${this.options.prefix}/authorized`, { sessionId: context.sessionId, identity, resourceRef: resource.resourceRef })).authorized === true,
            prepareImport: request => this.options.request(`${this.options.prefix}/prepare`, { sessionId: context.sessionId, ...request }),
            commitImport: request => this.options.request(`${this.options.prefix}/commit`, { sessionId: context.sessionId, ...request }),
            importStatus: request => this.options.request(`${this.options.prefix}/status`, { sessionId: context.sessionId, ...request }),
            settleImport: request => this.options.request(`${this.options.prefix}/settle`, { sessionId: context.sessionId, ...request }),
            committed: () => this.options.committed?.(context),
            load: () => {
                const raw = storage.getItem(key);
                if (raw) {
                    this.unresolved.add(key);
                    decodeEndnoteModuleState(raw);
                    // A loaded record is only a locator for checking the original
                    // operation. It is never a new native close acknowledgement.
                }
                return raw;
            },
            persist: (state: EndnoteModuleState) => {
                if (state.request.identity.scope !== selected.scope || state.request.identity.owner !== selected.owner)
                    throw new UiError(uiMessage("authoringAssets.moduleRecoveryOwnerMismatch"));
                this.unresolved.add(key);
                storage.setItem(key, JSON.stringify(state));
            },
            observed: receipt => {
                if (["committed", "cancelled", "resolved"].includes(receipt.status)) this.unresolved.delete(key);
                else this.unresolved.add(key);
            },
        };
        const installer = new EndnoteModuleInstaller(host);
        this.installers.add(installer);
        try {
            const value = await run(installer);
            check();
            return value;
        }
        finally {
            this.installers.delete(installer);
        }
        } finally { this.closeWork--; }
    }
    private async deliver(run: (installer: EndnoteModuleInstaller) => Promise<EndnoteModuleReady>): Promise<EndnoteModuleReady> {
        if (this.closeFrozen) throw new UiError(uiMessage("endnoteModule.originalInstallUnconfirmed"));
        this.closeWork++;
        try { const ready = await this.use(run); await this.options.changed(ready); return ready; }
        finally { this.closeWork--; }
    }
    install(): Promise<EndnoteModuleReady> { return this.deliver(installer => installer.install()); }
    retryOriginal(): Promise<EndnoteModuleReady> { return this.deliver(installer => installer.retryOriginal()); }
    restartAfterCancelled(): Promise<EndnoteModuleReady> { return this.deliver(installer => installer.restartAfterCancelled()); }
    recover(): Promise<EndnoteModuleReceipt | null> { return this.use(installer => installer.recover()); }
    settleOriginal(): Promise<EndnoteModuleReceipt> { return this.use(installer => installer.settleOriginal()); }
}
