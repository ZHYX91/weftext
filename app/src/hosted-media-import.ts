import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, uiTextValue, errorUiText, UiError, disposeUiTree, replaceUiChildren } from "./ui-locale";
import { descriptionUiText } from "./ui-diagnostics";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { canCommitResourceImport, decodeResourceImportDraft, freezeResourceImport, resourceCollection, resourceImportEnded, resourceImportInput, resourceImportReceipt, type HostedResourceCollection, type ResourceImportDraft, type ResourceImportReceipt } from "./hosted-media-import-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
const initial = (): ResourceImportDraft => ({ collection: "", name: "", revision: 0 });
/** Own upload bytes and original operations, without reading any owner source. */
export class HostedResourceImports {
    private epoch = 0;
    private catalogVersion = 0;
    private verified = false;
    private key = "";
    private draft = initial();
    private collections: HostedResourceCollection[] = [];
    private unreadable?: string;
    private original?: File;
    private reading = false;
    private busy = false;
    private picker = ui("select");
    private name = ui("input");
    private file = ui("input");
    private message = ui("p", "hint");
    private details = ui("p", "hint");
    private records = ui("section");
    private prepareButton = action(uiMessage("hostedMediaImport.prepareCollectionImport"), () => this.run(() => this.prepare()));
    constructor(private host: HTMLElement, private request: Request, private changed: () => Promise<void>) { this.clear(false); }
    get hasPending() { return !!this.draft.pending && !resourceImportEnded(this.draft.pending.receipt?.status); }
    clear(removeCache = true) {
        this.epoch++;
        this.catalogVersion++;
        this.verified = false;
        this.key = "";
        this.draft = initial();
        this.collections = [];
        this.original = undefined;
        this.reading = false;
        this.busy = false;
        this.unreadable = undefined;
        disposeUiTree(this.host);
        this.host.replaceChildren();
        this.host.hidden = true;
        this.picker = ui("select");
        this.name = ui("input");
        this.file = ui("input");
        this.picker.disabled = this.name.disabled = this.file.disabled = false;
        setUiText(this.message, "");
        setUiText(this.details, "");
        this.records.replaceChildren();
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-resource-import:"))
                    sessionStorage.removeItem(key);
    }
    connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch;
        this.key = hostedCachePrefix("resource-import", deployment, actor, generation) + "state";
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw)
                this.draft = decodeResourceImportDraft(raw);
        }
        catch {
            this.unreadable = raw ?? "";
        }
        setUiAttr(this.picker, "aria-label", uiMessage("hostedMediaImport.collection"));
        setUiAttr(this.name, "aria-label", uiMessage("hostedMediaImport.name"));
        setUiAttr(this.file, "aria-label", uiMessage("hostedMediaImport.original"));
        setUiText(this.prepareButton, uiMessage("hostedMediaImport.prepareCollectionImport"));
        this.file.type = "file";
        this.picker.onchange = () => { if (epoch !== this.epoch || !this.picker.isConnected) return; this.draft.collection = this.picker.value; this.inputChanged(); };
        this.name.oninput = () => { if (epoch !== this.epoch || !this.name.isConnected) return; this.draft.name = this.name.value; this.inputChanged(); };
        this.file.onchange = () => {
            if (epoch !== this.epoch || !this.file.isConnected) return;
            const file = this.file.files?.[0];
            if (file)
                void this.chooseFile(file);
        };
        this.host.replaceChildren(ui("h3", "", uiMessage("hostedMediaImport.title")), ui("p", "hint", uiMessage("hostedMediaImport.description")), this.picker, this.name, this.file, this.details, this.prepareButton, this.message, this.records);
        this.render();
    }
    setCatalog(collections: HostedResourceCollection[]) {
        const decoded = collections.map(resourceCollection);
        if (JSON.stringify(decoded) !== JSON.stringify(this.collections)) {
            this.catalogVersion++;
            this.verified = false;
            setUiText(this.message, this.draft.pending ? uiMessage("hostedMediaImport.originalNeedsVerification") : "");
        }
        this.collections = decoded;
        if (!this.draft.collection && !this.draft.pending)
            this.draft.collection = collections.find(item => item.import && item.status === "ready")?.collection ?? "";
        replaceUiChildren(this.picker, ...decoded.map(item => { const option = ui("option", "", () => `${uiTextValue(item.title)} · ${uiTextValue(item.read ? uiMessage("hostedMediaImport.readAllowed") : uiMessage("hostedMediaImport.readDenied"))} / ${uiTextValue(item.download ? uiMessage("hostedMediaImport.downloadAllowed") : uiMessage("hostedMediaImport.downloadDenied"))} / ${uiTextValue(item.import ? uiMessage("hostedMediaImport.importAllowed") : uiMessage("hostedMediaImport.importDenied"))}${uiTextValue(item.status === "unavailable" ? uiMessage("hostedStructure.unavailableSuffix") : "")}${uiTextValue(item.lifecycle === "trash" ? uiMessage("hostedMediaImport.recycledReadOnlySuffix") : "")}`); option.value = item.collection; return option; }));
        if (this.draft.collection && !collections.some(item => item.collection === this.draft.collection)) {
            const old = ui("option", "", displayMessage("hostedMediaImport.originalCollectionUnavailable", { p0: uiDisplayParameter(this.draft.collection) }));
            old.value = this.draft.collection;
            this.picker.append(old);
        }
        this.render();
        const original = this.draft.pending, current = original && this.collections.find(item => item.collection === original.request.collection);
        if (this.key && original && !this.verified && !this.busy && this.unreadable === undefined && current?.import && current.status === "ready" && current.lifecycle === "live") void this.run(() => this.lookup());
    }
    private persist(value = this.draft) {
        if (!this.key || this.unreadable !== undefined)
            throw new UiError(uiMessage("hostedMediaImport.recoveryNotWritable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(value));
        }
        catch {
            throw new UiError(uiMessage("hostedMediaImport.storageUnavailable"));
        }
    }
    private keep() {
        try {
            this.persist();
        }
        catch (error) {
            setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("hostedMediaImport.resultUnconfirmed"));
        }
    }
    private inputChanged() { this.draft.revision++; this.keep(); this.render(); }
    private async chooseFile(file: File) {
        const epoch = this.epoch;
        this.original = file;
        this.reading = true;
        this.draft.bytesBase64 = undefined;
        this.draft.bytes = file.size;
        this.draft.fileName = file.name;
        if (!this.draft.name)
            this.draft.name = file.name;
        this.draft.revision++;
        const revision = this.draft.revision;
        this.keep();
        this.render();
        try {
            if (file.size > 16 * 1024 * 1024)
                throw new UiError(uiMessage("hostedMediaImport.originalSizeLimit"));
            const bytes = new Uint8Array(await file.arrayBuffer());
            let binary = "";
            for (let offset = 0; offset < bytes.length; offset += 32768)
                binary += String.fromCharCode(...bytes.subarray(offset, offset + 32768));
            if (epoch !== this.epoch || this.original !== file)
                return;
            this.draft.bytesBase64 = btoa(binary);
            this.draft.bytes = bytes.length;
            this.draft.revision++;
            this.keep();
            setUiText(this.message, revision !== this.draft.revision - 1 ? uiMessage("hostedMediaImport.inputChangedDuringRead") : uiMessage("hostedMediaImport.originalReadRequestPersistenceRequired"));
        }
        catch (error) {
            if (epoch === this.epoch && this.original === file)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("hostedMediaImport.resultUnconfirmed"));
        }
        finally {
            if (epoch === this.epoch && this.original === file) {
                this.reading = false;
                this.render();
            }
        }
    }
    private async run(work: () => Promise<void>) {
        if (this.busy)
            return;
        const epoch = this.epoch, version = this.catalogVersion;
        this.busy = true;
        this.render();
        try {
            await work();
        }
        catch (error) {
            if (epoch === this.epoch && version === this.catalogVersion) {
                this.verified = false;
                setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("hostedMediaImport.resultUnconfirmed"));
            }
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async prepare() {
        if (this.reading)
            throw new UiError(uiMessage("hostedMediaImport.readingKeepInput"));
        const collection = this.collections.find(item => item.collection === this.draft.collection), request = freezeResourceImport(this.draft, collection, crypto.randomUUID());
        const next = { ...this.draft, pending: { request, owner: collection!.owner, input: resourceImportInput(this.draft), revision: this.draft.revision } };
        this.persist(next);
        this.draft = next;
        this.verified = false;
        const epoch = this.epoch, version = this.catalogVersion, result = await this.request<ResourceImportReceipt>("media/import/prepare", request);
        if (epoch === this.epoch && version === this.catalogVersion)
            this.receipt(result);
    }
    private receipt(value: ResourceImportReceipt) {
        const pending = this.draft.pending;
        if (!pending)
            return;
        pending.receipt = resourceImportReceipt(pending.request, value, pending.receipt);
        this.verified = true;
        this.keep();
        setUiText(this.message, value.status === "committed" ? uiMessage("hostedMediaImport.committedLocatorNotAuthority") : value.status === "prepared" ? uiMessage("hostedMediaImport.planPrepared") : resourceImportEnded(value.status) ? uiMessage("hostedMediaImport.originalEndedNotSaved") : uiMessage("hostedMediaImport.originalNeedsVerification"));
    }
    private async lookup() {
        const pending = this.draft.pending;
        if (!pending) return;
        const epoch = this.epoch, version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import/operation", { collection: pending.request.collection, operationId: pending.request.operationId });
        if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) {
            this.receipt(value);
            if (value.status === "committed") await this.changed();
        }
    }
    private render() {
        this.host.hidden = !this.collections.length && !this.draft.pending && !this.draft.bytesBase64 && this.unreadable === undefined;
        if (this.name.value !== this.draft.name)
            this.name.value = this.draft.name;
        this.picker.value = this.draft.collection;
        setUiText(this.details, this.draft.fileName ? displayMessage("hostedMediaImport.originalSummary", { p0: uiDisplayParameter(this.draft.fileName), p1: uiDisplayParameter(this.draft.bytes ?? uiMessage("common.unknown")), p2: uiDisplayParameter(this.reading ? uiMessage("hostedMediaImport.readingSuffix") : "") }) : uiMessage("hostedMediaImport.selectOriginalHint"));
        const collection = this.collections.find(item => item.collection === this.draft.collection);
        this.prepareButton.disabled = this.busy || this.reading || !!this.draft.pending || !collection?.import || collection.status !== "ready" || typeof this.draft.bytesBase64 !== "string" || this.unreadable !== undefined;
        replaceUiChildren(this.records);
        if (this.unreadable !== undefined) {
            this.picker.disabled = this.name.disabled = this.file.disabled = true;
            this.records.append(ui("p", "hint", uiMessage("hostedMediaImport.recoveryUnparseable")), ui("pre", "", this.unreadable), action(uiMessage("hostedMediaImport.discardCorruptRecovery"), () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.picker.disabled = this.name.disabled = this.file.disabled = false; this.render(); }));
            return;
        }
        const pending = this.draft.pending;
        if (!pending)
            return;
        const receipt = this.verified ? pending.receipt : undefined, card = ui("section", "resource-import-pending");
        card.append(ui("h4", "", uiMessage("hostedMediaImport.originalOperation")), ui("p", "", displayMessage("hostedMediaImport.originalOwnershipSummary", { p0: uiDisplayParameter(pending.request.collection), p1: uiDisplayParameter(pending.owner), p2: uiDisplayParameter(pending.request.operationId) })), ui("p", "", displayMessage("hostedStructure.operationStatus", { p0: uiDisplayParameter(receipt?.status ?? "unknown") })));
        if (receipt?.preview) {
            const summary = receipt.preview.summary;
            if (summary) card.append(ui("p", "", summary));
            if (receipt.preview.name !== undefined) card.append(ui("p", "", displayMessage("office.outputFileSummary", { p0: receipt.preview.name, p1: uiDisplayParameter(receipt.preview.bytes ?? uiMessage("common.unknown")) })));
            if (receipt.preview.metadata?.kind === "unsupported") card.append(ui("p", "hint", descriptionUiText(receipt.preview.metadata)));
        }
        if (pending.revision !== this.draft.revision || pending.input !== resourceImportInput(this.draft))
            card.append(ui("p", "hint", uiMessage("hostedMediaImport.laterInputChangedBasis")));
        const recover = action(uiMessage("hostedMediaImport.queryOriginalOperation"), () => this.run(() => this.lookup()));
        recover.disabled = this.busy;
        const retry = action(uiMessage("hostedMediaImport.retryOriginalPreparation"), () => this.run(async () => {
            this.persist();
            const epoch = this.epoch, version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import/prepare", pending.request);
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) {
                this.receipt(value);
                if (value.status === "committed")
                    await this.changed();
            }
        }));
        retry.disabled = this.busy || resourceImportEnded(receipt?.status);
        const commit = action(uiMessage("hostedMediaImport.commitOriginalPlan"), () => this.run(async () => {
            if (!canCommitResourceImport(this.draft, collection, this.verified) || this.reading)
                throw new UiError(uiMessage("hostedMediaImport.inputOrAuthorityChanged"));
            this.persist();
            const epoch = this.epoch, version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import/commit", { collection: pending.request.collection, operationId: pending.request.operationId, planId: pending.receipt!.planId });
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) {
                this.receipt(value);
                if (value.status === "committed")
                    await this.changed();
            }
        }));
        commit.disabled = this.busy || this.reading || !canCommitResourceImport(this.draft, collection, this.verified);
        const settle = action(uiMessage("hostedMediaImport.endUninstalledOriginal"), () => this.run(async () => {
            const epoch = this.epoch, version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import/settle", { collection: pending.request.collection, operationId: pending.request.operationId });
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending)
                this.receipt(value);
        }));
        settle.disabled = this.busy || resourceImportEnded(receipt?.status);
        card.append(commit, recover, retry, settle);
        if (receipt?.status === "committed") {
            card.append(ui("p", "hint", uiMessage("hostedMediaImport.locatorRequiresIndependentAuthority")), ui("pre", "", (receipt.resourceRefs ?? []).join("\n")));
        }
        if (resourceImportEnded(receipt?.status))
            card.append(action(uiMessage("hostedMediaImport.clearCompletedReceiptKeepFile"), () => { const next = { ...this.draft, pending: undefined }; this.persist(next); this.draft = next; this.render(); }));
        this.records.append(card);
    }
}
