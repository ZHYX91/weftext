import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, uiTextValue, errorUiText, UiError, disposeUiTree, replaceUiChildren } from "./ui-locale";
import { descriptionUiText } from "./ui-diagnostics";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { canCommitResourceImport, checkResourceImportSizes, decodeResourceImportDraft, encodeResourceImportDraft, freezeResourceImport, resourceCollection, resourceImportDigest, resourceImportEnded, resourceImportInput, resourceImportLocator, resourceImportReceipt, type HostedResourceCollection, type ResourceImportDraft, type ResourceImportFileInput, type ResourceImportReceipt, type ResourceImportRequest } from "./hosted-media-import-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
const initial = (): ResourceImportDraft => ({ version: 2, collection: "", files: [], revision: 0 });
interface FileRow { element: HTMLElement; name: HTMLInputElement; summary: HTMLElement }
/** One batch owner for single/multiple files. Stored locators never contain upload bytes. */
export class HostedResourceImports {
    private epoch = 0;
    private catalogVersion = 0;
    private verified = false;
    private key = "";
    private draft = initial();
    private collections: HostedResourceCollection[] = [];
    private unreadable?: string;
    private originals: File[] = [];
    private originalRequest?: ResourceImportRequest;
    private readVersion = 0;
    private reading = false;
    private busy = false;
    private picker = ui("select");
    private file = ui("input");
    private files = ui("ol", "resource-import-files");
    private readonly rows = new Map<ResourceImportFileInput, FileRow>();
    private message = ui("p", "hint");
    private details = ui("p", "hint");
    private records = ui("section");
    private prepareButton = action(uiMessage("hostedMediaImport.prepareCollectionImport"), () => this.run(() => this.prepare()));
    constructor(private host: HTMLElement, private request: Request, private changed: () => Promise<void>) { this.clear(false); }
    get hasPending() { return !!this.draft.pending && !resourceImportEnded(this.draft.pending.receipt?.status); }
    clear(removeCache = true) {
        this.epoch++; this.catalogVersion++; this.readVersion++;
        this.verified = false; this.key = ""; this.draft = initial(); this.collections = [];
        this.originals = []; this.originalRequest = undefined; this.reading = false; this.busy = false; this.unreadable = undefined;
        disposeUiTree(this.host); this.host.replaceChildren(); this.host.hidden = true;
        this.picker = ui("select"); this.file = ui("input"); this.files = ui("ol", "resource-import-files"); this.rows.clear();
        this.picker.disabled = this.file.disabled = false;
        setUiText(this.message, ""); setUiText(this.details, ""); this.records.replaceChildren();
        if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-resource-import:")) sessionStorage.removeItem(key);
    }
    connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch;
        this.key = hostedCachePrefix("resource-import", deployment, actor, generation) + "state";
        let raw: string | null = null;
        try { raw = sessionStorage.getItem(this.key); if (raw) this.draft = decodeResourceImportDraft(raw); }
        catch { this.unreadable = raw ?? ""; }
        setUiAttr(this.picker, "aria-label", uiMessage("hostedMediaImport.collection"));
        setUiAttr(this.file, "aria-label", uiMessage("hostedMediaImport.original"));
        this.file.type = "file"; this.file.multiple = true;
        this.picker.onchange = () => {
            if (epoch !== this.epoch || !this.picker.isConnected) return;
            this.draft.collection = this.picker.value; this.verified = false; this.inputChanged();
        };
        this.file.onchange = () => { if (epoch === this.epoch && this.file.isConnected) void this.chooseFiles(Array.from(this.file.files ?? [])); };
        this.host.replaceChildren(ui("h3", "", uiMessage("hostedMediaImport.title")), ui("p", "hint", uiMessage("hostedMediaImport.batchDescription")), this.picker, this.file, ui("p", "hint", uiMessage("hostedMediaImport.batchLimits")), this.details, this.files, this.prepareButton, this.message, this.records);
        this.render();
    }
    setCatalog(collections: HostedResourceCollection[]) {
        const decoded = collections.map(resourceCollection);
        if (JSON.stringify(decoded) !== JSON.stringify(this.collections)) {
            this.catalogVersion++; this.verified = false;
            setUiText(this.message, this.draft.pending ? uiMessage("hostedMediaImport.originalNeedsVerification") : "");
        }
        this.collections = decoded;
        if (!this.draft.collection && !this.draft.pending) this.draft.collection = decoded.find(item => item.import && item.status === "ready")?.collection ?? "";
        replaceUiChildren(this.picker, ...decoded.map(item => {
            const option = ui("option", "", () => [uiTextValue(item.title), uiTextValue(item.read ? uiMessage("hostedMediaImport.readAllowed") : uiMessage("hostedMediaImport.readDenied")), uiTextValue(item.download ? uiMessage("hostedMediaImport.downloadAllowed") : uiMessage("hostedMediaImport.downloadDenied")), uiTextValue(item.import ? uiMessage("hostedMediaImport.importAllowed") : uiMessage("hostedMediaImport.importDenied")), uiTextValue(item.status === "unavailable" ? uiMessage("hostedStructure.unavailableSuffix") : ""), uiTextValue(item.lifecycle === "trash" ? uiMessage("hostedMediaImport.recycledReadOnlySuffix") : "")].filter(Boolean).join(" · "));
            option.value = item.collection; return option;
        }));
        if (this.draft.collection && !decoded.some(item => item.collection === this.draft.collection)) { const old = ui("option", "", displayMessage("hostedMediaImport.originalCollectionUnavailable", { p0: uiDisplayParameter(this.draft.collection) })); old.value = this.draft.collection; this.picker.append(old); }
        this.render();
        const original = this.draft.pending, current = original && decoded.find(item => item.collection === original.request.collection);
        if (this.key && original && !this.verified && !this.busy && this.unreadable === undefined && current?.import && current.status === "ready" && current.lifecycle === "live") void this.run(() => this.lookup());
    }
    private persist(value = this.draft) {
        if (!this.key || this.unreadable !== undefined) throw new UiError(uiMessage("hostedMediaImport.recoveryNotWritable"));
        try {
            const raw = encodeResourceImportDraft(value);
            sessionStorage.setItem(this.key, raw);
            if (sessionStorage.getItem(this.key) !== raw) throw new Error("Original import locator was not read back");
        } catch { throw new UiError(uiMessage("hostedMediaImport.storageUnavailable")); }
    }
    private keep(): boolean {
        try { this.persist(); return true; }
        catch (error) { setUiText(this.message, errorUiText(error)); return false; }
    }
    private inputChanged() { this.draft.revision++; this.keep(); this.render(); }
    private async chooseFiles(files: File[]) {
        const epoch = this.epoch, readVersion = ++this.readVersion;
        this.originals = files; this.reading = true;
        this.draft.files = files.map(file => ({ name: file.name, fileName: file.name, bytes: file.size }));
        this.draft.revision++; this.keep(); this.render();
        try {
            checkResourceImportSizes(this.draft.files);
            const read: { bytesBase64: string; sha256: string; bytes: number }[] = [];
            for (const file of files) {
                const bytes = new Uint8Array(await file.arrayBuffer());
                if (epoch !== this.epoch || readVersion !== this.readVersion) return;
                let binary = "";
                for (let offset = 0; offset < bytes.length; offset += 32768) binary += String.fromCharCode(...bytes.subarray(offset, offset + 32768));
                read.push({ bytesBase64: btoa(binary), sha256: await resourceImportDigest(bytes), bytes: bytes.length });
            }
            if (epoch !== this.epoch || readVersion !== this.readVersion) return;
            this.draft.files.forEach((file, index) => Object.assign(file, read[index]));
            this.draft.revision++; this.keep();
            setUiText(this.message, uiMessage("hostedMediaImport.originalReadRequestPersistenceRequired"));
        } catch (error) {
            if (epoch === this.epoch && readVersion === this.readVersion) setUiText(this.message, errorUiText(error));
        } finally { if (epoch === this.epoch && readVersion === this.readVersion) { this.reading = false; this.render(); } }
    }
    private async run(work: () => Promise<void>) {
        if (this.busy) return;
        const epoch = this.epoch, version = this.catalogVersion;
        this.busy = true; this.render();
        try { await work(); }
        catch (error) { if (epoch === this.epoch && version === this.catalogVersion) { this.verified = false; setUiText(this.message, errorUiText(error)); } }
        finally { if (epoch === this.epoch) { this.busy = false; this.render(); } }
    }
    private async prepare() {
        if (this.reading) throw new UiError(uiMessage("hostedMediaImport.readingKeepInput"));
        const epoch = this.epoch, version = this.catalogVersion, revision = this.draft.revision, input = resourceImportInput(this.draft);
        const collection = this.collections.find(item => item.collection === this.draft.collection), upload = freezeResourceImport(this.draft, collection, crypto.randomUUID()), locator = await resourceImportLocator(upload);
        if (epoch !== this.epoch || version !== this.catalogVersion || revision !== this.draft.revision || input !== resourceImportInput(this.draft)) return;
        const next = { ...this.draft, pending: { request: locator, owner: collection!.owner, input, revision } };
        this.persist(next); // The exact original locator must survive before any effect.
        this.draft = next; this.originalRequest = upload; this.verified = false;
        const result = await this.request<ResourceImportReceipt>("media/import-batch/prepare", upload);
        if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === next.pending) this.receipt(result);
    }
    private receipt(value: ResourceImportReceipt) {
        const pending = this.draft.pending;
        if (!pending) return;
        pending.receipt = resourceImportReceipt(pending.request, value, pending.receipt);
        this.verified = true;
        if (this.keep()) setUiText(this.message, value.status === "committed" ? uiMessage("hostedMediaImport.batchCommittedBodyUnchanged") : value.status === "prepared" ? uiMessage("hostedMediaImport.planPrepared") : resourceImportEnded(value.status) ? uiMessage("hostedMediaImport.originalEndedNotSaved") : uiMessage("hostedMediaImport.originalNeedsVerification"));
    }
    private async lookup() {
        const pending = this.draft.pending;
        if (!pending) return;
        const epoch = this.epoch, version = this.catalogVersion, selected = this.draft.collection;
        const value = await this.request<ResourceImportReceipt>("media/import-batch/operation", { collection: pending.request.collection, operationId: pending.request.operationId });
        if (epoch === this.epoch && version === this.catalogVersion && selected === this.draft.collection && this.draft.pending === pending) { this.receipt(value); if (value.status === "committed") await this.changed(); }
    }
    private renderFiles() {
        const keep = new Set(this.draft.files);
        for (const [file, row] of this.rows) if (!keep.has(file)) { disposeUiTree(row.element); row.element.remove(); this.rows.delete(file); }
        this.draft.files.forEach((file, index) => {
            let row = this.rows.get(file);
            if (!row) {
                const element = ui("li"), name = ui("input"), summary = ui("p", "hint"), epoch = this.epoch;
                name.value = file.name;
                setUiAttr(name, "aria-label", index === 0 ? uiMessage("hostedMediaImport.name") : displayMessage("hostedMediaImport.batchName", { p0: uiDisplayParameter(index + 1) }));
                name.oninput = () => { if (epoch !== this.epoch || !name.isConnected || !this.draft.files.includes(file)) return; file.name = name.value; this.inputChanged(); };
                element.append(summary, name); row = { element, name, summary }; this.rows.set(file, row); this.files.append(element);
            }
            row.name.disabled = this.unreadable !== undefined;
            setUiText(row.summary, displayMessage("hostedMediaImport.originalSummary", { p0: uiDisplayParameter(file.fileName), p1: uiDisplayParameter(file.bytes), p2: uiDisplayParameter(this.reading ? uiMessage("hostedMediaImport.readingSuffix") : typeof file.bytesBase64 !== "string" ? uiMessage("hostedMediaImport.locatorOnlySuffix") : "") }));
        });
    }
    private render() {
        this.host.hidden = !this.collections.length && !this.draft.pending && !this.draft.files.length && this.unreadable === undefined;
        this.picker.value = this.draft.collection;
        this.renderFiles();
        setUiText(this.details, this.draft.files.length ? displayMessage("hostedMediaImport.batchSummary", { p0: uiDisplayParameter(this.draft.files.length), p1: uiDisplayParameter(this.draft.files.reduce((sum, file) => sum + file.bytes, 0)) }) : uiMessage("hostedMediaImport.selectOriginalHint"));
        const collection = this.collections.find(item => item.collection === this.draft.collection);
        this.prepareButton.disabled = this.busy || this.reading || !!this.draft.pending || !collection?.import || collection.status !== "ready" || collection.lifecycle !== "live" || !this.draft.files.length || this.draft.files.some(file => typeof file.bytesBase64 !== "string") || this.unreadable !== undefined;
        replaceUiChildren(this.records);
        if (this.unreadable !== undefined) {
            this.picker.disabled = this.file.disabled = true;
            this.records.append(ui("p", "hint", uiMessage("hostedMediaImport.recoveryUnparseable")), ui("pre", "", this.unreadable), action(uiMessage("hostedMediaImport.discardCorruptRecovery"), () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.picker.disabled = this.file.disabled = false; this.render(); }));
            return;
        }
        const pending = this.draft.pending;
        if (!pending) return;
        const epoch = this.epoch, receipt = this.verified ? pending.receipt : undefined, card = ui("section", "resource-import-pending");
        card.append(ui("h4", "", uiMessage("hostedMediaImport.originalOperation")), ui("p", "", displayMessage("hostedMediaImport.originalOwnershipSummary", { p0: uiDisplayParameter(pending.request.collection), p1: uiDisplayParameter(pending.owner), p2: uiDisplayParameter(pending.request.operationId) })), ui("p", "", displayMessage("hostedStructure.operationStatus", { p0: uiDisplayParameter(receipt?.status ?? "unknown") })));
        if (receipt?.preview && pending.request.collection === this.draft.collection) {
            if (receipt.preview.summary) card.append(ui("p", "", receipt.preview.summary));
            if (receipt.preview.files) card.append(ui("p", "hint", displayMessage("hostedMediaImport.batchSummary", { p0: uiDisplayParameter(receipt.preview.files.length), p1: uiDisplayParameter(receipt.preview.totalBytes ?? 0) })));
            const preview = ui("ol");
            for (const file of receipt.preview.files ?? []) {
                const row = ui("li"); row.append(ui("p", "", displayMessage("office.outputFileSummary", { p0: file.name, p1: uiDisplayParameter(file.bytes) })), ui("p", "hint", file.contentType));
                if (file.metadata.kind === "unsupported") row.append(ui("p", "hint", descriptionUiText(file.metadata)));
                preview.append(row);
            }
            card.append(preview);
        }
        if (pending.revision !== this.draft.revision || pending.input !== resourceImportInput(this.draft)) card.append(ui("p", "hint", uiMessage("hostedMediaImport.laterInputChangedBasis")));
        const original = (work: () => Promise<void>) => this.run(async () => { if (epoch !== this.epoch || this.draft.pending !== pending) throw new UiError(uiMessage("hostedMediaImport.inputOrAuthorityChanged")); await work(); });
        const recover = action(uiMessage("hostedMediaImport.queryOriginalOperation"), () => original(() => this.lookup())); recover.disabled = this.busy;
        const retry = action(uiMessage("hostedMediaImport.retryOriginalPreparation"), () => original(async () => {
            if (!this.originalRequest || this.originalRequest.operationId !== pending.request.operationId) throw new UiError(uiMessage("hostedMediaImport.originalUploadUnavailable"));
            this.persist(); const version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import-batch/prepare", this.originalRequest);
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) { this.receipt(value); if (value.status === "committed") await this.changed(); }
        })); retry.disabled = this.busy || !this.originalRequest || resourceImportEnded(receipt?.status);
        const commit = action(uiMessage("hostedMediaImport.commitOriginalPlan"), () => original(async () => {
            if (!canCommitResourceImport(this.draft, collection, this.verified) || this.reading) throw new UiError(uiMessage("hostedMediaImport.inputOrAuthorityChanged"));
            this.persist(); const version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import-batch/commit", { collection: pending.request.collection, operationId: pending.request.operationId, planId: pending.receipt!.planId });
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) { this.receipt(value); if (value.status === "committed") await this.changed(); }
        })); commit.disabled = this.busy || this.reading || !canCommitResourceImport(this.draft, collection, this.verified);
        const settle = action(uiMessage("hostedMediaImport.endUninstalledOriginal"), () => original(async () => {
            const version = this.catalogVersion, value = await this.request<ResourceImportReceipt>("media/import-batch/settle", { collection: pending.request.collection, operationId: pending.request.operationId });
            if (epoch === this.epoch && version === this.catalogVersion && this.draft.pending === pending) this.receipt(value);
        })); settle.disabled = this.busy || resourceImportEnded(receipt?.status);
        card.append(commit, recover, retry, settle);
        if (!this.originalRequest && !resourceImportEnded(receipt?.status)) card.append(ui("p", "hint", uiMessage("hostedMediaImport.originalUploadUnavailable")));
        if (receipt?.status === "committed" && pending.request.collection === this.draft.collection) card.append(ui("p", "hint", uiMessage("hostedMediaImport.locatorRequiresIndependentAuthority")), ui("pre", "", receipt.results.map(file => [file.name, file.resourceRef, file.exactRevision].join(" · ")).join("\n")));
        if (resourceImportEnded(receipt?.status)) card.append(action(uiMessage("hostedMediaImport.clearCompletedReceiptKeepFile"), () => {
            if (epoch !== this.epoch || this.draft.pending !== pending) return;
            const next = { ...this.draft, pending: undefined }; this.persist(next); this.draft = next; this.originalRequest = undefined; this.render();
        }));
        this.records.append(card);
    }
}
