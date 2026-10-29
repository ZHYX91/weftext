import type { DocumentBasis, SourcePatch } from "./native-value-form";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { decodeEditorMapResponse, exactWritingRange } from "./writing";
import { message, setUiText, setUiAttr, replaceUiChildren, errorUiText, UiError, type UiText } from "./ui-locale";
import { checkLocalResourceSizes, decodeLocalResourceInsertion, decodeLocalResourceLocator, decodeLocalResourcePlan, localResourceKey,
    removeLocalResourceLocator, resourceBasisKey, resourceDigest, resourceOwner, sameResourceOwner, writeLocalResourceLocator,
    type LocalResourceLocator, type LocalResourceMode, type LocalResourceOwner, type LocalResourcePlan, type LocalResourceRecord } from "./resource-import-batch-state";

export interface ResourceImportBatchHost {
    request<T>(path: string, body?: unknown): Promise<T>;
    getDocument(): DocumentBasis | null;
    /** Source qualification only. Import-only does not require a writable body. */
    canEdit(): boolean;
    applyPatch(patch: SourcePatch, basis: DocumentBasis): Promise<void>;
    prepareAction(): Promise<void>;
    /** Resource environment refresh only; never adopt a saved author document. */
    changed(root: string): Promise<void>;
    notify(text: UiText, error?: boolean): void;
    run(operation: () => Promise<void>): Promise<void>;
    storage?: Pick<Storage, "getItem" | "setItem" | "removeItem">;
}
function checkSelectedSizes(files: readonly { bytes: number }[]): void {
    try { checkLocalResourceSizes(files); }
    catch { throw new UiError(message("localResourceBatch.sizeLimit")); }
}

/** One local upload owner. The locator is a recovery address, never a Core fact
 * store: resource outcomes are freshly read from the original Core operation. */
export class ResourceImportBatch {
    readonly element = document.createElement("dialog");
    private readonly guard = new ProposalCloseGuard();
    private readonly storage: Pick<Storage, "getItem" | "setItem" | "removeItem">;
    private record: LocalResourceRecord | null = null;
    private blocked = false;
    private storageAttempt: LocalResourceLocator | null = null;
    private removing = false;
    private loadingOwner: LocalResourceOwner | null = null;
    private loadingRaw: string | null | undefined;
    private plan: LocalResourcePlan | null = null;
    private knownCommitted = false;
    private status = "draft";
    private refreshed = false;
    private qualifiedRoute: string | null = null;
    private busy = false;
    private draft: { basis: DocumentBasis; files: readonly File[]; names: string[]; mode: LocalResourceMode } | null = null;
    private originalWire: string | null = null;
    private originalRoute: string | null = null;
    private insertionBasis: DocumentBasis | null = null;
    private error: UiText | null = null;
    private closeWaiters: (() => void)[] = [];
    private readonly composing = new Set<EventTarget>();

    constructor(private readonly host: ResourceImportBatchHost) {
        this.storage = host.storage ?? {
            getItem: key => localStorage.getItem(key),
            setItem: (key, value) => localStorage.setItem(key, value),
            removeItem: key => localStorage.removeItem(key),
        };
        this.element.className = "modal writing-dialog resource-import-batch";
        this.element.dataset.resourceImportBatch = "local";
        this.element.style.width = "min(48rem, calc(100vw - 2rem))";
        this.element.style.maxHeight = "calc(100vh - 2rem)";
        this.element.style.overflow = "auto";
        setUiAttr(this.element, "aria-label", message("localResourceBatch.title"));
        this.guard.bind(this.element);
        this.element.addEventListener("compositionstart", event => { if (event.target) this.composing.add(event.target); });
        this.element.addEventListener("compositionend", event => { if (event.target) this.composing.delete(event.target); });
        this.element.addEventListener("cancel", event => { event.preventDefault(); if (!this.busy && !this.composing.size) this.hide(); });
    }

    hasPending(): boolean { return !!this.record || this.blocked || !!this.draft || this.element.open; }
    assertCloseReady(): void { this.guard.assertReady(this.hasPending()); }
    freezeForClose(): boolean { return this.guard.freeze(this.hasPending()); }
    thawAfterClose(): void { this.guard.thaw(); }

    /** Resolves when this dialog is dismissed. Pending original operations stay
     * owned and close-blocking after hiding; a later open reveals the same batch. */
    async open(files: readonly File[], sessionId: string): Promise<void> {
        this.guard.assertMutable();
        if (!this.record && !this.draft && !this.blocked) {
            const basis = this.host.getDocument();
            if (!basis || basis.sessionId !== sessionId) throw new UiError(message("localResourceBatch.noDocument"));
            const owner = resourceOwner(basis);
            this.load(owner);
            if (this.record) this.status = "unknown";
            if (!this.record && !this.blocked) {
                checkSelectedSizes(files.map(file => ({ bytes: file.size })));
                this.draft = { basis: { ...basis }, files: [...files], names: files.map(file => file.name), mode: "importOnly" };
            }
        }
        this.show();
        if (this.record && !this.busy) void this.run(() => this.inspect());
        await new Promise<void>(resolve => this.closeWaiters.push(resolve));
    }

    /** Navigation may discover a locator without focusing or showing a dialog.
     * A live owner from another note is never replaced by the new note. */
    async recover(): Promise<void> {
        this.guard.assertMutable();
        if (this.busy || this.draft) return;
        if (this.blocked) { await this.run(() => this.recheckStorage()); return; }
        if (!this.record) {
            const basis = this.host.getDocument();
            if (!basis?.workspaceId || !basis.nodeId) return;
            this.load(resourceOwner(basis));
            if (this.blocked) { this.render(); return; }
        }
        if (!this.record || !this.currentOwner()) return;
        this.status = this.knownCommitted ? "committed" : "unknown";
        await this.run(() => this.inspect());
    }

    refresh(): void { if (this.element.open && !this.composing.size) this.render(); }

    private show(): void {
        if (!this.element.isConnected) document.body.append(this.element);
        this.render();
        if (!this.element.open) this.element.showModal();
    }
    private hide(): void {
        this.guard.assertMutable();
        if (this.busy || this.composing.size) return;
        if (!this.record && !this.blocked) { this.draft = null; this.originalWire = null; this.originalRoute = null; this.insertionBasis = null; }
        this.element.close();
        for (const resolve of this.closeWaiters.splice(0)) resolve();
    }
    private button(key: Parameters<typeof message>[0], action: () => Promise<void> | void, control: string): HTMLButtonElement {
        const button = document.createElement("button"); button.type = "button"; button.dataset.resourceImportControl = control;
        const owner = this.record ?? this.draft;
        setUiText(button, message(key)); button.disabled = this.busy;
        button.addEventListener("click", () => {
            if (button.isConnected && this.element.open && this.element.contains(button) && owner === (this.record ?? this.draft) && !this.busy && !this.composing.size)
                void this.run(async () => { await action(); });
        });
        return button;
    }
    private async run(action: () => Promise<void>): Promise<void> {
        if (this.busy || this.composing.size) return;
        this.guard.assertMutable(); this.busy = true; this.error = null; this.render();
        try { await this.guard.run(() => this.host.run(action)); }
        catch (error) { this.error = errorUiText(error); this.host.notify(this.error, true); }
        finally { this.busy = false; this.render(); }
    }
    private currentOwner(): DocumentBasis | null {
        const basis = this.host.getDocument(), expected = this.record?.locator.owner ?? (this.draft && resourceOwner(this.draft.basis));
        if (!basis || !expected || !basis.workspaceId || !basis.nodeId) return null;
        return basis.workspaceId === expected.workspaceId && basis.nodeId === expected.nodeId ? basis : null;
    }
    private route(basis: DocumentBasis): string { return JSON.stringify([basis.workspaceId, basis.nodeId, basis.sessionId, basis.root, basis.path]); }
    private async requireQualifiedRoute(): Promise<DocumentBasis> {
        let basis = this.requireOwner();
        if (this.qualifiedRoute !== this.route(basis)) { await this.inspect(); basis = this.requireOwner(); }
        if (this.qualifiedRoute !== this.route(basis)) throw new UiError(message("localResourceBatch.scopeChanged"));
        return basis;
    }
    private requireOwner(): DocumentBasis {
        const basis = this.currentOwner();
        if (!basis) throw new UiError(message("localResourceBatch.scopeChanged"));
        return basis;
    }
    private stored(): LocalResourceRecord {
        const record = this.record;
        try { if (!record || this.storage.getItem(localResourceKey(record.locator.owner)) !== record.raw) throw new Error(); }
        catch { this.blocked = true; throw new UiError(message("localResourceBatch.storageFailed")); }
        return record;
    }
    private async storageLock<T>(owner: LocalResourceOwner, action: () => T): Promise<T> {
        if (!navigator.locks?.request) { this.blocked = true; throw new UiError(message("localResourceBatch.storageFailed")); }
        return navigator.locks.request(`weftext-resource-import:${localResourceKey(owner)}`, { mode: "exclusive" }, action);
    }
    private async retain(locator: LocalResourceLocator, current: () => boolean = () => true): Promise<void> {
        const expected = this.record;
        let retiredBeforeWrite = false;
        this.storageAttempt = locator;
        try {
            await this.storageLock(locator.owner, () => {
                if (this.record !== expected || this.guard.frozen || !current()) {
                    retiredBeforeWrite = true;
                    throw new UiError(message("localResourceBatch.sourceChanged"));
                }
                this.record = writeLocalResourceLocator(this.storage, locator, expected); this.storageAttempt = null;
            });
        }
        catch (error) {
            if (retiredBeforeWrite) { this.storageAttempt = null; throw error; }
            this.blocked = true; throw new UiError(message("localResourceBatch.storageFailed"));
        }
    }
    private load(owner: LocalResourceOwner): void {
        this.loadingOwner = owner; this.loadingRaw = undefined;
        try {
            this.loadingRaw = this.storage.getItem(localResourceKey(owner));
            if (this.loadingRaw === null) { this.record = null; return; }
            const locator = decodeLocalResourceLocator(this.loadingRaw);
            if (locator.owner.workspaceId !== owner.workspaceId || locator.owner.nodeId !== owner.nodeId) throw new Error();
            this.record = { raw: this.loadingRaw, locator };
        }
        catch { this.blocked = true; this.error = message("localResourceBatch.storageFailed"); }
    }
    /** A transient storage failure can be retried, but only the exact original
     * or the exact write attempted by this owner may become its new token. */
    private async recheckStorage(): Promise<void> {
        const record = this.record, attempted = this.storageAttempt;
        try {
            const owner = record?.locator.owner ?? attempted?.owner ?? this.loadingOwner;
            if (!owner) throw new Error();
            await this.storageLock(owner, () => {
                if (record || attempted) {
                    const raw = this.storage.getItem(localResourceKey(owner));
                    if (record && raw === record.raw) { this.blocked = false; this.storageAttempt = null; this.removing = false; }
                    else if (attempted && raw === JSON.stringify(attempted)) {
                        this.record = { raw, locator: decodeLocalResourceLocator(raw, owner) };
                        this.draft = null; this.blocked = false; this.storageAttempt = null;
                    } else if (!record && attempted && raw === null) {
                        this.record = writeLocalResourceLocator(this.storage, attempted, null);
                        this.draft = null; this.blocked = false; this.storageAttempt = null;
                    } else if (record && this.removing && raw === null) {
                        this.resetFinished(); return;
                    } else throw new Error();
                } else {
                    if (!this.loadingOwner) throw new Error();
                    const raw = this.storage.getItem(localResourceKey(this.loadingOwner));
                    if (this.loadingRaw !== undefined && this.loadingRaw !== raw) throw new Error();
                    if (raw === null) this.record = null;
                    else {
                        const locator = decodeLocalResourceLocator(raw);
                        if (locator.owner.workspaceId !== this.loadingOwner.workspaceId || locator.owner.nodeId !== this.loadingOwner.nodeId) throw new Error();
                        this.record = { raw, locator };
                    }
                    this.blocked = false;
                }
            });
        } catch { this.blocked = true; throw new UiError(message("localResourceBatch.storageFailed")); }
        if (this.record) await this.inspect();
    }
    private async prepare(): Promise<void> {
        const draft = this.draft;
        if (!draft || this.record || this.blocked) return;
        await this.host.prepareAction();
        const basis = this.requireOwner();
        const owner = resourceOwner(basis), files = [...draft.files], names = [...draft.names], mode = draft.mode;
        const route = this.route(basis);
        const sameRoute = () => this.draft === draft && !!this.currentOwner() && this.route(this.requireOwner()) === route;
        checkSelectedSizes(files.map(file => ({ bytes: file.size })));
        const operationId = crypto.randomUUID(), uploads: { name: string; bytes: string }[] = [], info: LocalResourceLocator["files"] = [];
        for (let index = 0; index < files.length; index++) {
            const file = files[index], bytes = new Uint8Array(await file.arrayBuffer());
            if (!sameRoute() || file.size !== bytes.length) throw new UiError(message("localResourceBatch.sourceChanged"));
            let binary = "";
            for (let offset = 0; offset < bytes.length; offset += 8192) binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
            uploads.push({ name: names[index], bytes: btoa(binary) });
            info.push({ name: names[index], fileName: file.name, bytes: bytes.length, sha256: await resourceDigest(bytes) });
            if (!sameRoute()) throw new UiError(message("localResourceBatch.sourceChanged"));
        }
        const wire = JSON.stringify({ sessionId: basis.sessionId, operationId, files: uploads });
        const wireDigest = await resourceDigest(wire);
        if (!sameRoute()) throw new UiError(message("localResourceBatch.sourceChanged"));
        this.insertionBasis = { ...basis };
        await this.retain({ version: 1, owner, operationId, requestKind: "operation", wireDigest, files: info, mode, insertion: "pending", phase: "original" }, sameRoute);
        if (!sameRoute()) throw new UiError(message("localResourceBatch.sourceChanged"));
        this.originalWire = wire; this.originalRoute = route; this.draft = null; this.status = "unknown";
        await this.sendOriginalPrepare();
    }
    private async sendOriginalPrepare(): Promise<void> {
        const record = this.stored(); this.requireOwner();
        if (!this.originalWire || this.knownCommitted) return;
        const route = this.route(this.requireOwner());
        if (route !== this.originalRoute) throw new UiError(message("localResourceBatch.scopeChanged"));
        if (await resourceDigest(this.originalWire) !== record.locator.wireDigest) throw new UiError(message("error.invalidResponse"));
        if (this.route(this.requireOwner()) !== route) throw new UiError(message("localResourceBatch.scopeChanged"));
        this.stored();
        const originalSession = (JSON.parse(this.originalWire) as { sessionId: string }).sessionId;
        const basis = this.requireOwner();
        try {
            await this.receivePlan(await this.host.request("/api/resources/prepare-import-batch", JSON.parse(this.originalWire)),
                originalSession === basis.sessionId && sameResourceOwner(resourceOwner(basis), record.locator.owner) ? basis : undefined);
        }
        catch (error) { await this.inspect(); throw error; }
    }
    private async receivePlan(wire: unknown, routeBasis?: DocumentBasis): Promise<void> {
        const record = this.stored(), plan = decodeLocalResourcePlan(wire, record.locator);
        if (this.plan && (this.plan.planId !== plan.planId || JSON.stringify(this.plan.imports) !== JSON.stringify(plan.imports))) throw new UiError(message("error.invalidResponse"));
        if (this.knownCommitted && plan.status !== "committed") throw new UiError(message("error.invalidResponse"));
        this.plan = plan; this.status = plan.status;
        const current = this.currentOwner();
        const qualified = !!routeBasis && !!current && this.route(routeBasis) === this.route(current);
        if (qualified) this.qualifiedRoute = this.route(routeBasis!);
        if (plan.status === "committed") {
            this.knownCommitted = true;
            if (!this.refreshed) {
                try { await this.host.changed(qualified ? routeBasis!.root : record.locator.owner.root); this.refreshed = true; }
                catch { throw new UiError(message("localResourceBatch.refreshFailed")); }
            }
        }
    }
    private fact(value: unknown): { operationId: string; status: string } {
        const fact = value as { operationId?: unknown; status?: unknown } | null;
        if (!fact || fact.operationId !== this.record?.locator.operationId || !["unknown", "prepared", "committed", "recoveryPending", "resolved", "cancelled"].includes(String(fact.status))) throw new UiError(message("error.invalidResponse"));
        if (this.knownCommitted && fact.status !== "committed") throw new UiError(message("error.invalidResponse"));
        return fact as { operationId: string; status: string };
    }
    private async inspect(): Promise<void> {
        const record = this.stored(); this.requireOwner();
        let fact = this.fact(await this.host.request("/api/operations/status", { operationId: record.locator.operationId }));
        this.stored();
        if (fact.status === "unknown") {
            fact = this.fact(await this.host.request("/api/operations/settle", { operationId: record.locator.operationId, requestKind: record.locator.requestKind, wireDigest: record.locator.wireDigest }));
            this.stored();
        }
        this.status = fact.status;
        if (fact.status === "committed") this.knownCommitted = true;
        if (["prepared", "committed", "recoveryPending"].includes(fact.status)) {
            const basis = this.requireOwner();
            await this.receivePlan(await this.host.request("/api/resources/import-batch/status", { sessionId: basis.sessionId, operationId: record.locator.operationId }), basis);
        }
    }
    private async commit(): Promise<void> {
        const record = this.stored();
        await this.requireQualifiedRoute();
        const plan = this.plan;
        if (!plan || plan.status !== "prepared" || this.knownCommitted) return;
        // Keep the visible source at confirmation as the sole automatic insert
        // basis. Later input can never be adopted from a resource receipt.
        this.insertionBasis = { ...this.requireOwner() };
        try {
            const fact = this.fact(await this.host.request("/api/operations/commit", { operationId: record.locator.operationId, planId: plan.planId }));
            this.status = fact.status;
            if (fact.status === "committed") this.knownCommitted = true;
            await this.inspect();
        } catch (error) { await this.inspect(); throw error; }
        if (this.knownCommitted && this.record?.locator.mode !== "importOnly") await this.insert(this.insertionBasis);
    }
    private async cancelPlan(): Promise<void> {
        const record = this.stored(); await this.requireQualifiedRoute();
        if (this.knownCommitted || this.status !== "prepared") return;
        await this.host.request("/api/recovery/resolve", { operationId: record.locator.operationId, action: "keepCurrent" });
        await this.inspect();
        if (["resolved", "cancelled"].includes(this.status)) await this.finish();
    }
    private async insert(captured?: DocumentBasis | null): Promise<void> {
        const record = this.stored();
        if (!this.knownCommitted || this.plan?.status !== "committed" || record.locator.mode === "importOnly" || record.locator.insertion !== "pending") return;
        await this.host.prepareAction();
        await this.requireQualifiedRoute();
        const plan = this.plan;
        if (!plan || plan.status !== "committed") throw new UiError(message("error.invalidResponse"));
        const basis = { ...(captured ?? this.requireOwner()) }, key = resourceBasisKey(basis);
        const basisCurrent = () => this.plan === plan && this.host.canEdit() && !!this.currentOwner()
            && resourceBasisKey(this.requireOwner()) === key && !this.guard.frozen;
        const current = () => this.record === record && basisCurrent();
        if (!current()) throw new UiError(message("localResourceBatch.sourceChanged"));
        const sourceHash = await resourceDigest(basis.source);
        if (!current()) throw new UiError(message("localResourceBatch.sourceChanged"));
        const map = decodeEditorMapResponse(await this.host.request("/api/editor-map", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial }), basis.serial);
        if (!current()) throw new UiError(message("localResourceBatch.sourceChanged"));
        if (map.sourceHash !== sourceHash || !map.syntaxValid || map.unavailableReason || !map.bodyInsertion?.writable || !exactWritingRange(basis.source, map.bodyInsertion)) throw new UiError(message("localResourceBatch.sourceChanged"));
        const wire = await this.host.request("/api/resources/import-batch/insert", { sessionId: basis.sessionId, operationId: record.locator.operationId, source: basis.source, serial: basis.serial, sourceHash, mode: record.locator.mode === "autoAppend" ? "auto" : "links" });
        if (!current()) throw new UiError(message("localResourceBatch.sourceChanged"));
        const patch = decodeLocalResourceInsertion(wire, record.locator, plan, basis, sourceHash, map);
        if (!current()) throw new UiError(message("localResourceBatch.sourceChanged"));
        // Consume before delivery: applyPatch may change the editor then reject
        // its persistence ACK. Neither equal text nor this old proposal grants
        // permission to append again after that boundary.
        await this.retain({ ...record.locator, insertion: "applying" }, current);
        const applying = this.record;
        if (!applying || !basisCurrent() || applying.locator.insertion !== "applying") throw new UiError(message("localResourceBatch.sourceChanged"));
        try { await this.host.applyPatch(patch, basis); }
        catch (error) { try { await this.retain({ ...this.record!.locator, insertion: "uncertain" }, () => this.record === applying); } catch { /* keep the durable applying marker */ } throw error; }
        await this.retain({ ...this.record!.locator, insertion: "inserted" }, () => this.record === applying);
        this.host.notify(message("localResourceBatch.inserted"));
    }
    private async finish(): Promise<void> {
        const record = this.stored();
        if (!this.knownCommitted && !["resolved", "cancelled"].includes(this.status)) return;
        if (this.knownCommitted && !this.refreshed) throw new UiError(message("localResourceBatch.refreshFailed"));
        this.removing = true;
        try { await this.storageLock(record.locator.owner, () => {
            if (this.record !== record || this.guard.frozen) throw new Error();
            removeLocalResourceLocator(this.storage, record);
        }); }
        catch { this.blocked = true; throw new UiError(message("localResourceBatch.storageFailed")); }
        if (this.record !== record) throw new UiError(message("localResourceBatch.storageFailed"));
        this.resetFinished();
    }
    private resetFinished(): void {
        this.record = null; this.plan = null; this.originalWire = null; this.originalRoute = null; this.insertionBasis = null; this.draft = null;
        this.knownCommitted = false; this.status = "draft"; this.refreshed = false; this.blocked = false;
        this.qualifiedRoute = null;
        this.storageAttempt = null; this.removing = false; this.loadingOwner = null; this.loadingRaw = undefined;
        // The action owns busy until its tracked run settles; close the dialog
        // directly without granting another action a gap during delivery.
        this.element.close(); for (const resolve of this.closeWaiters.splice(0)) resolve();
    }
    private render(): void {
        const focused = this.element.contains(document.activeElement) ? document.activeElement as HTMLElement : null;
        const focusedControl = focused?.dataset.resourceImportControl, focusedName = focused?.dataset.resourceImportName;
        const title = document.createElement("h2"); setUiText(title, message("localResourceBatch.title"));
        const help = document.createElement("p"); setUiText(help, message("localResourceBatch.help"));
        const info = this.record?.locator.files ?? this.draft?.files.map((file, index) => ({ name: this.draft!.names[index], fileName: file.name, bytes: file.size })) ?? [];
        const table = document.createElement("table"); table.style.width = "100%";
        const headers = document.createElement("tr");
        for (const key of [null, "localResourceBatch.originalName", "localResourceBatch.targetName", "localResourceBatch.bytes"] as const) {
            const cell = document.createElement("th"); if (key) setUiText(cell, message(key)); else cell.textContent = "#"; headers.append(cell);
        }
        const head = document.createElement("thead"); head.append(headers); const body = document.createElement("tbody");
        info.forEach((file, index) => {
            const row = document.createElement("tr"); row.dataset.resourceImportIndex = String(index);
            const ordinal = document.createElement("td"); ordinal.textContent = String(index + 1);
            const original = document.createElement("td"); original.textContent = file.fileName; original.style.overflowWrap = "anywhere";
            const target = document.createElement("td");
            if (this.draft && !this.record) {
                const draft = this.draft, input = document.createElement("input"); input.type = "text"; input.value = file.name; input.disabled = this.busy; input.maxLength = 240;
                input.dataset.resourceImportName = String(index); setUiAttr(input, "aria-label", message("localResourceBatch.targetName"));
                input.style.width = "100%"; input.addEventListener("input", () => { if (input.isConnected && this.element.open && this.element.contains(input) && this.draft === draft && !this.record && !this.busy) draft.names[index] = input.value; }); target.append(input);
            } else { target.textContent = file.name; target.style.overflowWrap = "anywhere"; }
            const bytes = document.createElement("td"); bytes.textContent = file.bytes.toLocaleString(); row.append(ordinal, original, target, bytes); body.append(row);
        });
        table.append(head, body);
        const total = document.createElement("p"); total.dataset.resourceImportTotal = ""; total.textContent = `${info.length} / 64 · ${info.reduce((sum, file) => sum + file.bytes, 0).toLocaleString()} / 67,108,864 B`;
        const modeLabel = document.createElement("label"); modeLabel.style.display = "block"; setUiText(modeLabel, message("localResourceBatch.mode"));
        const mode = document.createElement("select"); mode.dataset.resourceImportControl = "mode"; setUiAttr(mode, "aria-label", message("localResourceBatch.mode"));
        mode.style.width = "100%"; mode.style.maxWidth = "100%"; mode.style.minWidth = "0";
        for (const value of ["importOnly", "autoAppend", "linksAppend"] as const) { const option = document.createElement("option"); option.value = value; setUiText(option, message(`localResourceBatch.${value}`)); option.disabled = value !== "importOnly" && !this.host.canEdit(); mode.append(option); }
        mode.value = this.record?.locator.mode ?? this.draft?.mode ?? "importOnly"; mode.disabled = this.busy || !!this.record || this.blocked;
        const draft = this.draft; mode.addEventListener("change", () => { if (mode.isConnected && this.element.open && this.element.contains(mode) && draft && this.draft === draft && !this.record && !this.busy) draft.mode = mode.value as LocalResourceMode; }); modeLabel.append(mode);
        const modeHelp = document.createElement("p"); setUiText(modeHelp, message("localResourceBatch.modeHelp"));
        const status = document.createElement("p"); status.dataset.resourceImportStatus = ""; status.setAttribute("role", "status");
        const insertion = this.record?.locator.insertion;
        const statusKey = this.knownCommitted ? insertion === "inserted" ? "inserted" : insertion === "applying" || insertion === "uncertain" ? "uncertain" : "committed" : this.status === "prepared" ? "prepared" : this.status === "resolved" || this.status === "cancelled" ? "resolved" : this.record ? "unknown" : "draft";
        setUiText(status, message(`localResourceBatch.${statusKey}`));
        const changes = document.createElement("ul"); changes.dataset.resourceImportChanges = "";
        for (const change of this.plan?.changes ?? []) { const item = document.createElement("li"); item.textContent = `${change.path} · ${change.action} · ${change.bytes.toLocaleString()} B`; item.style.overflowWrap = "anywhere"; changes.append(item); }
        const controls = document.createElement("div"); controls.className = "dialog-actions";
        if (this.blocked) controls.append(this.button("localResourceBatch.recover", () => this.recheckStorage(), "recover-storage"));
        if (this.draft && !this.record && !this.blocked) controls.append(this.button("localResourceBatch.prepare", () => this.prepare(), "prepare"));
        if (this.record && !this.blocked) {
            const inScope = !!this.currentOwner();
            const recover = this.button("localResourceBatch.recover", () => this.inspect(), "recover"); recover.disabled ||= !inScope; controls.append(recover);
            if (!this.knownCommitted && this.status === "unknown" && this.originalWire) { const retry = this.button("localResourceBatch.retryOriginal", () => this.sendOriginalPrepare(), "retry"); retry.disabled ||= !inScope || this.originalRoute !== this.route(this.requireOwner()); controls.append(retry); }
            if (!this.knownCommitted && this.status === "prepared" && this.plan) {
                const confirm = this.button("localResourceBatch.confirm", () => this.commit(), "confirm"), cancel = this.button("localResourceBatch.cancelPlan", () => this.cancelPlan(), "cancel-plan"); confirm.disabled ||= !inScope; cancel.disabled ||= !inScope; controls.append(confirm, cancel);
            }
            if (this.knownCommitted && this.record.locator.mode !== "importOnly" && insertion === "pending") { const insert = this.button("localResourceBatch.insert", () => this.insert(), "insert"); insert.disabled ||= !inScope || !this.host.canEdit() || !this.refreshed || this.plan?.status !== "committed"; controls.append(insert); }
            if (this.knownCommitted || ["resolved", "cancelled"].includes(this.status)) {
                const finish = this.button(this.knownCommitted ? insertion === "inserted" ? "localResourceBatch.finishInserted" : insertion === "uncertain" || insertion === "applying" ? "localResourceBatch.finishUncertain" : "localResourceBatch.finishOnly" : "localResourceBatch.finishInserted", () => this.finish(), "finish");
                finish.disabled ||= this.knownCommitted && !this.refreshed; controls.append(finish);
            }
            if (!inScope) { const warning = document.createElement("p"); setUiText(warning, message("localResourceBatch.scopeChanged")); controls.append(warning); }
        }
        const close = document.createElement("button"); close.type = "button"; close.dataset.resourceImportControl = "close"; setUiText(close, message("common.close")); close.disabled = this.busy;
        close.addEventListener("click", () => { if (close.isConnected && this.element.open && this.element.contains(close)) this.hide(); }); controls.append(close);
        const error = document.createElement("p"); error.setAttribute("role", "alert"); error.hidden = !this.error; if (this.error) setUiText(error, this.error);
        replaceUiChildren(this.element, title, help, table, total, modeLabel, modeHelp, status, changes, error, controls);
        this.element.setAttribute("aria-busy", String(this.busy));
        if (this.element.open && focused && !this.busy) {
            const candidates = [...this.element.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("input,select,button")];
            const target = candidates.find(item => !item.disabled && (focusedControl && item.dataset.resourceImportControl === focusedControl || focusedName !== undefined && item.dataset.resourceImportName === focusedName))
                ?? candidates.find(item => !item.disabled);
            target?.focus({ preventScroll: true });
        }
    }
}
