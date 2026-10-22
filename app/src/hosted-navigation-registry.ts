import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { registryIntent, registryReceipt, type RegistryCatalog, type RegistryDraft, type RegistryReceipt, type RegistryRequest, type RegistryResult } from "./hosted-navigation-registry-state";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
type Job = {
    request: RegistryRequest;
    receipt?: RegistryReceipt;
    commit?: {
        operationId: string;
        planId: string;
    };
    phase?: "prepare" | "commit" | "settle";
};
type Cache = {
    draft: RegistryDraft;
    job?: Job;
};
const initial = (): Cache => ({ draft: { kind: "admitDirectory", slot: "", reference: "", displayName: "", parentRef: null } });
const ended = (status?: string) => ["committed", "resolved", "cancelled"].includes(status ?? "");
export interface HostedRegistryHost {
    authorInputs?: AuthorInputFactory;
    identity(): HostedLinksIdentity | null;
    authorizationChanged(message: UiText, current: {
        actor: string;
        deployment: string;
        generation: number;
    }): void;
    registryChanged?(): void;
}
function selection(label: UiText, values: [
    string,
    UiText
][], value: string) {
    const select = ui("select", "data-input");
    setUiAttr(select, "aria-label", label);
    if (value && !values.some(([id]) => id === value))
        values.push([value, uiMessage("hostedRegistry.originalSelectionUnavailable")]);
    for (const [id, text] of values) {
        const option = ui("option", "", text);
        option.value = id;
        select.append(option);
    }
    select.value = value;
    return select;
}
/** Directory navigation is a capability of its own, never a Query scan. */
export class HostedNavigationRegistry {
    private epoch = 0;
    private inputScopes: AuthorInputScopes;
    private key = "";
    private cache = initial();
    private catalog?: RegistryCatalog;
    private catalogTree?: TagTree;
    private busy = false;
    private verified = false;
    private unreadable?: UiText;
    private catalogSerial = 0;
    private resolveSerial = 0;
    private message = ui("p", "hint");
    private editor = ui("section");
    private resolved = ui("section");
    constructor(private host: HTMLElement, private request: Request, private owner: HostedRegistryHost) { this.inputScopes = new AuthorInputScopes(owner.authorInputs); this.clear(false); }
    clear(removeCache = true): void {
        this.inputScopes.clear();
        this.catalogSerial++;
        this.resolveSerial++;
        this.epoch++;
        this.key = "";
        this.cache = initial();
        this.catalog = undefined;
        this.catalogTree = undefined;
        this.busy = false;
        this.verified = false;
        this.unreadable = undefined;
        setUiText(this.message, "");
        this.editor.replaceChildren();
        this.resolved.replaceChildren();
        this.host.replaceChildren();
        this.host.hidden = true;
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-navigation-registry:"))
                    sessionStorage.removeItem(key);
    }
    async connect(deployment: string, actor: string, generation: number): Promise<void> {
        this.clear(false);
        const epoch = this.epoch;
        this.key = hostedCachePrefix("navigation-registry", deployment, actor, generation) + "state";
        const identity = this.owner.identity();
        if (!identity || identity.deployment !== deployment || identity.actor !== actor || identity.generation !== generation) {
            this.clear(false);
            return;
        }
        try {
            const raw = sessionStorage.getItem(this.key);
            if (raw) {
                const parsed = JSON.parse(raw) as Cache;
                if (!parsed.draft || !["admitDirectory", "createTag", "renameTag", "moveTag"].includes(parsed.draft.kind) || [parsed.draft.slot, parsed.draft.reference, parsed.draft.displayName].some(v => typeof v !== "string") || parsed.draft.parentRef !== undefined && parsed.draft.parentRef !== null && typeof parsed.draft.parentRef !== "string")
                    throw new Error();
                parsed.draft.parentRef ??= null;
                if (parsed.job?.receipt)
                    registryReceipt(parsed.job.request, parsed.job.receipt);
                this.cache = parsed;
            }
        }
        catch {
            this.unreadable = uiMessage("hostedRegistry.recoveryUnparseable");
        }
        this.render();
        await this.refresh();
        if (epoch === this.epoch && this.cache.job)
            await this.run(() => this.lookup());
    }
    private async authority(identity: HostedLinksIdentity | null): Promise<boolean> {
        if (!identity || !sameHostedLinksIdentity(identity, this.owner.identity()))
            return false;
        const current = await this.request<{
            actor: string;
            deployment: string;
            generation: number;
        }>("me");
        if (!sameHostedLinksIdentity(identity, this.owner.identity()))
            return false;
        if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) {
            this.owner.authorizationChanged(uiMessage("hostedRegistry.authorizationChanged"), current);
            return false;
        }
        return true;
    }
    private persist(): void {
        if (!this.key || this.unreadable)
            throw new UiError(uiMessage("hostedRegistry.recoveryNotWritable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(this.cache));
        }
        catch {
            throw new UiError(uiMessage("hostedRegistry.requestNotDurable"));
        }
    }
    private keep(): void {
        try {
            this.persist();
        }
        catch (e) {
            setUiText(this.message, (e as Error).message);
        }
    }
    private async run(work: () => Promise<void>): Promise<void> {
        if (this.busy)
            return;
        this.busy = true;
        const epoch = this.epoch;
        this.render();
        try {
            await work();
        }
        catch (e) {
            if (epoch === this.epoch) {
                this.verified = false;
                this.resolved.replaceChildren();
                setUiText(this.message, (e as Error).message);
            }
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async refresh(): Promise<void> {
        const epoch = this.epoch, serial = ++this.catalogSerial, identity = this.owner.identity();
        try {
            const result = await this.request<RegistryCatalog>("navigation/registry");
            if (!await this.authority(identity) || epoch !== this.epoch || serial !== this.catalogSerial)
                return;
            if (result.directoryScope !== "configuredAuthorizedDirectories" || !["complete", "partial"].includes(result.directoryCoverage) || !Array.isArray(result.directories) || !Array.isArray(result.admissionSlots) || !result.tags || !["complete", "partial", "unavailable"].includes(result.tags.coverage) || !Array.isArray(result.tags.items))
                throw new UiError(uiMessage("hostedRegistry.catalogFormatUnconfirmed"));
            this.catalogTree = result.tags.read && result.tags.coverage === "complete" ? new TagTree(result.tags.items, true) : undefined;
            this.catalog = result;
            this.render();
        }
        catch (e) {
            if (epoch === this.epoch && serial === this.catalogSerial) {
                this.catalog = undefined;
                this.catalogTree = undefined;
                setUiText(this.message, (e as Error).message);
                this.render();
            }
        }
    }
    private accept(incoming: RegistryReceipt): void {
        const job = this.cache.job;
        if (!job)
            return;
        job.receipt = registryReceipt(job.request, incoming, job.receipt);
        job.phase = undefined;
        this.verified = true;
        this.keep();
        if (incoming.status === "committed" && job.request.intent.kind !== "admitDirectory") {
            this.resolveSerial++;
            this.resolved.replaceChildren();
            this.owner.registryChanged?.();
        }
        setUiText(this.message, incoming.status === "committed" ? uiMessage("hostedRegistry.committedNavigationOnly") : incoming.status === "prepared" ? uiMessage("hostedRegistry.checkProposalThenCommit") : uiMessage("hostedRegistry.factsCheckedNotFalseSuccess"));
    }
    private async prepare(): Promise<void> {
        if (this.cache.job)
            return;
        const catalog = this.catalog, draft = this.cache.draft;
        if (!catalog)
            throw new UiError(uiMessage("hostedRegistry.readAuthorizedCatalogFirst"));
        const intent = registryIntent(draft, catalog);
        const request = { operationId: crypto.randomUUID(), intent }, epoch = this.epoch, identity = this.owner.identity();
        this.cache.job = { request, phase: "prepare" };
        this.persist();
        const result = await this.request<RegistryReceipt>("navigation/registry/prepare", structuredClone(request));
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.accept(result);
    }
    private async lookup(): Promise<void> {
        const job = this.cache.job;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        const result = await this.request<RegistryReceipt>("navigation/registry/operation", { operationId: job.request.operationId });
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.accept(result);
        if (result.status === "committed")
            await this.refresh();
    }
    private async commit(): Promise<void> {
        const job = this.cache.job;
        if (!this.verified || !job?.receipt?.commitAllowed || !job.receipt.planId)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        job.commit ??= { operationId: job.request.operationId, planId: job.receipt.planId };
        job.phase = "commit";
        this.persist();
        const result = await this.request<RegistryReceipt>("navigation/registry/commit", job.commit);
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.accept(result);
        if (result.status === "committed")
            await this.refresh();
    }
    private async settle(): Promise<void> {
        const job = this.cache.job;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        job.phase = "settle";
        this.persist();
        const result = await this.request<RegistryReceipt>("navigation/registry/settle", { operationId: job.request.operationId });
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.accept(result);
    }
    private async resolve(body: unknown, expected: string): Promise<void> {
        const epoch = this.epoch, serial = ++this.resolveSerial, identity = this.owner.identity();
        const result = await this.request<RegistryResult>("navigation/registry/resolve", body);
        if (!await this.authority(identity) || epoch !== this.epoch || serial !== this.resolveSerial)
            return;
        if (!result || result.reference !== expected || result.status !== "available" || result.readOnly !== true)
            throw new UiError(uiMessage("hostedRegistry.addressEligibilityUnconfirmed"));
        this.resolved.replaceChildren(ui("h3", "", result.kind === "tag" ? uiMessage("hostedRegistry.authorizedTag") : uiMessage("hostedRegistry.authorizedDirectory")), ui("p", "", result.title ?? ""), ui("code", "", result.reference), action(uiMessage("hostedRegistry.copyVerifiedAddress"), () => {
            void this.run(async () => {
                if (await this.authority(identity) && epoch === this.epoch && serial === this.resolveSerial)
                    await navigator.clipboard.writeText(result.reference);
            });
        }));
    }
    private selectTag(reference: string | null): void {
        this.resolved.replaceChildren();
        this.resolveSerial++;
        if (!reference)
            return;
        setUiText(this.message, uiMessage("hostedRegistry.checkingSelectedTagIdentity"));
        const epoch = this.epoch, pending = this.resolve({ kind: "tag", reference }, reference), serial = this.resolveSerial;
        void pending.catch(error => {
            if (epoch === this.epoch && serial === this.resolveSerial)
                setUiText(this.message, String(error));
        });
    }
    private render(): void {
        const focused = this.editor.contains(document.activeElement) ? document.activeElement as HTMLElement : null, label = focused?.getAttribute("aria-label");
        const caret = focused instanceof HTMLInputElement && ["text", "search"].includes(focused.type) ? [focused.selectionStart, focused.selectionEnd] : null;
        const restore = () => {
            const control = label ? this.editor.querySelector<HTMLElement>(`[aria-label="${CSS.escape(label)}"]`) : null;
            control?.focus({ preventScroll: true });
            if (control instanceof HTMLInputElement && caret?.[0] !== null && caret?.[0] !== undefined && caret[1] !== null)
                control.setSelectionRange(caret[0], caret[1]);
        };
        this.host.hidden = !this.key;
        if (this.host.hidden)
            return;
        this.host.replaceChildren(ui("h2", "", uiMessage("hostedRegistry.title")), this.message, this.editor, this.resolved);
        this.editor.replaceChildren();
        this.editor.append(action(uiMessage("hostedRegistry.refreshCatalogs"), () => { void this.run(() => this.refresh()); }), ui("p", "hint", uiMessage("hostedRegistry.navigationScopeDescription")));
        if (this.unreadable) {
            this.editor.append(ui("p", "hint", this.unreadable));
            return;
        }
        const catalog = this.catalog;
        if (catalog) {
            if (catalog.directoryCoverage === "partial")
                this.editor.append(ui("p", "hint", uiMessage("hostedRegistry.directoryCatalogPartial")));
            for (const directory of catalog.directories) {
                const button = action(displayMessage("hostedRegistry.checkDirectory", { p0: uiDisplayParameter(directory.title) }), () => { void this.run(() => this.resolve({ kind: "directory", directory: directory.directory }, directory.reference)); });
                button.disabled = this.busy || !directory.canResolve || directory.status !== "available";
                this.editor.append(button);
            }
            if (catalog.tags.coverage !== "complete")
                this.editor.append(ui("p", "hint", catalog.tags.coverage === "partial" ? uiMessage("hostedRegistry.tagCatalogPartial") : uiMessage("hostedRegistry.tagReadUnavailable")));
            if (this.catalogTree)
                this.editor.append(tagPicker(this.catalogTree, { label: uiMessage("hostedRegistry.authorizedTagCatalog"), changed: reference => this.selectTag(reference) }));
        }
        const draft = this.cache.draft, form = ui("fieldset");
        form.append(ui("legend", "", uiMessage("hostedRegistry.proposal")));
        const kind = selection(uiMessage("hostedRegistry.operationKind"), [["admitDirectory", uiMessage("hostedRegistry.admitDirectory")], ["createTag", uiMessage("hostedRegistry.createTag")], ["renameTag", uiMessage("hostedRegistry.renameTag")], ["moveTag", uiMessage("hostedRegistry.moveTag")]], draft.kind);
        kind.onchange = () => {
            if (kind.isConnected === false)
                return;
            this.inputScopes.clear();
            draft.kind = kind.value as RegistryDraft["kind"];
            this.keep();
            this.render();
        };
        form.append(kind);
        if (draft.kind === "admitDirectory") {
            const slots = selection(uiMessage("hostedRegistry.directorySlot"), [["", uiMessage("hostedRegistry.selectSlot")], ...(catalog?.admissionSlots ?? []).map(s => [s.slot, s.title] as [
                    string,
                    UiText
                ])], draft.slot);
            slots.onchange = () => {
                if (slots.isConnected === false)
                    return;
                this.inputScopes.clear();
                draft.slot = slots.value;
                this.keep();
                this.render();
            };
            form.append(slots);
        }
        else {
            if (this.catalogTree) {
                const tree = this.catalogTree;
                if (draft.kind !== "createTag")
                    form.append(tagPicker(tree, { label: uiMessage("hostedRegistry.targetTag"), value: draft.reference, changed: reference => { this.inputScopes.clear(); draft.reference = reference ?? ""; this.keep(); this.render(); } }));
                if (draft.kind !== "renameTag")
                    form.append(tagPicker(tree, { label: uiMessage("tagNavigation.parentLabel"), value: draft.parentRef ?? "", root: true, excludedRoot: draft.kind === "moveTag" ? tree.byRef.get(draft.reference)?.tagId : undefined, changed: reference => { this.inputScopes.clear(); draft.parentRef = reference; this.keep(); } }));
            }
            else
                form.append(ui("p", "hint", uiMessage("hostedRegistry.completeStructureNotReadable")));
            if (draft.kind !== "moveTag") {
                const name = ui("input", "data-input"), duplicates = ui("div", "hint");
                setUiAttr(name, "aria-label", uiMessage("hostedRegistry.tagDisplayName"));
                name.value = draft.displayName;
                const epoch = this.epoch, target = JSON.stringify([draft.kind, draft.slot, draft.reference, draft.parentRef]);
                this.inputScopes.get(draft, { kind: "registry", target }, () => this.epoch === epoch && this.cache.draft === draft && JSON.stringify([draft.kind, draft.slot, draft.reference, draft.parentRef]) === target)?.bind(name, "display-name");
                const showNames = () => {
                    duplicates.replaceChildren();
                    if (draft.kind !== "createTag" || !this.catalogTree || !name.value)
                        return;
                    const same = [...this.catalogTree.byId.values()].filter(tag => tag.displayName === name.value);
                    if (same.length)
                        duplicates.append(ui("p", "", displayMessage("hostedRegistry.sameNameTagCount", { p0: same.length })), ...same.slice(0, 100).map(tag => ui("p", "", this.catalogTree!.label(tag.reference))));
                };
                name.oninput = () => { draft.displayName = name.value; this.keep(); showNames(); };
                showNames();
                form.append(name, duplicates);
            }
        }
        const canPrepare = !!catalog && (draft.kind === "admitDirectory" ? catalog.admissionSlots.some(s => s.slot === draft.slot) : catalog.tags.read && catalog.tags.coverage === "complete" && !!catalog.tags.basis && (draft.kind === "createTag" ? catalog.tags.create : draft.kind === "moveTag" ? catalog.tags.move : catalog.tags.rename));
        const prepare = action(uiMessage("hostedRegistry.prepareProposal"), () => { void this.run(() => this.prepare()); });
        prepare.disabled = this.busy || !!this.cache.job || !canPrepare;
        form.append(prepare);
        this.editor.append(form);
        const job = this.cache.job;
        if (!job) {
            restore();
            return;
        }
        const card = ui("section", "output-status"), receipt = this.verified ? job.receipt : undefined;
        card.append(ui("h3", "", uiMessage("hostedRegistry.originalOperation")), ui("p", "", job.request.operationId), ui("p", "", receipt?.status ?? uiMessage("hostedRegistry.idAwaitingAuthorization")));
        if (receipt) {
            for (const [key, label] of [["title", uiMessage("nodeContents.directoryKind")], ["beforeName", uiMessage("hostedRegistry.originalTag")], ["displayName", uiMessage("hostedRegistry.proposedName")]] as const)
                if (receipt.preview[key] !== undefined)
                    card.append(ui("p", "", () => `${uiTextValue(label)}：${uiTextValue(receipt.preview[key])}`));
        }
        if (receipt) {
            for (const [key, label] of [["beforeParentRef", uiMessage("hostedRegistry.originalParent")], ["parentRef", uiMessage("hostedRegistry.proposedParent")]] as const)
                if (receipt.preview[key] !== undefined)
                    card.append(ui("p", "", () => `${uiTextValue(label)}：${uiTextValue(receipt.preview[key] ?? uiMessage("tagPicker.rootLevel"))}`));
        }
        const commit = action(uiMessage("hostedRegistry.commitOriginalProposal"), () => { void this.run(() => this.commit()); });
        commit.disabled = this.busy || !!job.phase || !receipt?.commitAllowed;
        const status = action(uiMessage("hostedRegistry.checkOriginalOperation"), () => { void this.run(() => this.lookup()); });
        status.disabled = this.busy;
        const settle = action(uiMessage("hostedRegistry.endUninstalledOriginal"), () => { void this.run(() => this.settle()); });
        settle.disabled = this.busy || ended(receipt?.status);
        card.append(commit, status, settle);
        if (receipt?.result?.canResolve && receipt.result.reference) {
            const body = job.request.intent.kind === "admitDirectory" ? { kind: "admittedDirectory", slot: job.request.intent.slot, operationId: job.request.operationId, reference: receipt.result.reference } : { kind: "tag", reference: receipt.result.reference };
            card.append(action(uiMessage("hostedRegistry.checkCurrentOriginalAddress"), () => { void this.run(() => this.resolve(body, receipt.result!.reference)); }));
        }
        const fresh = action(uiMessage("hostedRegistry.prepareAnotherOperation"), () => { delete this.cache.job; this.verified = false; this.resolved.replaceChildren(); this.keep(); this.render(); });
        fresh.disabled = this.busy || !ended(receipt?.status);
        card.append(fresh);
        this.editor.append(card);
        restore();
    }
}
