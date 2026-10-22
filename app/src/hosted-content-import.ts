import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, uiTextValue, errorUiText, UiError, disposeUiTree, replaceUiChildren, type UiText } from "./ui-locale";
import { descriptionUiText } from "./ui-diagnostics";
import { decodeContentMembers, decodeContentTargets, importLossUiText } from "./hosted-import-wire";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { canCommitContent, contentBytes, contentEnded, contentFormats, contentInspection, contentReceipt, decodeContentDraft, freezeContentImport, type ContentDraft, type ContentFormat, type ContentInspection, type ContentMembers, type ContentReceipt, type ContentTarget } from "./hosted-content-import-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedContentImportHost {
    identity(): HostedLinksIdentity | null;
    authorizationChanged(message: UiText, current: {
        actor: string;
        deployment: string;
        generation: number;
    }): void;
    open(resource: string): Promise<void>;
}
const initial = (): ContentDraft => ({ slot: "", format: "native", revision: 0 });
const labels: Record<ContentFormat, UiText> = { native: uiMessage("hostedContentImport.nativePackage"), bibtex: "BibTeX", "csl-json": "CSL JSON", ris: "RIS" };
const sha = async (bytes: Uint8Array) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>))].map(v => v.toString(16).padStart(2, "0")).join("");
/** A content producer is independent of both the current source and future readers. */
export class HostedContentImport {
    private epoch = 0;
    private key = "";
    private draft = initial();
    private targets: ContentTarget[] = [];
    private verified = false;
    private busy = false;
    private reading = false;
    private original?: File;
    private unreadable?: UiText;
    private inspected?: {
        revision: number;
        value: ContentInspection;
    };
    private reviewedRevision?: number;
    private acceptedLosses = false;
    private members?: ContentMembers;
    private slot = ui("select", "data-input");
    private format = ui("select", "data-input");
    private file = ui("input", "data-input");
    private message = ui("p", "hint");
    private details = ui("p", "hint");
    private inspection = ui("section");
    private records = ui("section");
    private memberView = ui("section");
    private inspectButton = action(uiMessage("hostedContentImport.inspectOriginal"), () => { void this.run(() => this.inspect()); });
    private prepareButton = action(uiMessage("hostedContentImport.prepareFixedSlotImport"), () => { void this.run(() => this.prepare()); });
    constructor(private host: HTMLElement, private request: Request, private owner: HostedContentImportHost) { this.clear(false); }
    clear(removeCache = true): void {
        this.epoch++;
        this.key = "";
        this.draft = initial();
        this.targets = [];
        this.verified = false;
        this.busy = this.reading = false;
        this.original = undefined;
        this.unreadable = undefined;
        this.inspected = undefined;
        this.reviewedRevision = undefined;
        this.acceptedLosses = false;
        this.members = undefined;
        disposeUiTree(this.host);
        this.host.replaceChildren();
        this.host.hidden = true;
        this.slot = ui("select", "data-input");
        this.format = ui("select", "data-input");
        this.file = ui("input", "data-input");
        this.slot.disabled = this.format.disabled = this.file.disabled = false;
        setUiText(this.message, "");
        setUiText(this.details, "");
        this.records.replaceChildren();
        this.inspection.replaceChildren();
        this.memberView.replaceChildren();
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-content-import:"))
                    sessionStorage.removeItem(key);
    }
    async connect(deployment: string, actor: string, generation: number): Promise<void> {
        this.clear(false);
        const epoch = this.epoch, identity = this.owner.identity();
        if (!identity || identity.actor !== actor || identity.deployment !== deployment || identity.generation !== generation)
            return;
        this.key = hostedCachePrefix("content-import", deployment, actor, generation) + "state";
        try {
            const raw = sessionStorage.getItem(this.key);
            if (raw)
                this.draft = decodeContentDraft(raw);
        }
        catch {
            this.unreadable = uiMessage("hostedContentImport.recoveryUnparseable");
        }
        setUiAttr(this.slot, "aria-label", uiMessage("hostedContentImport.fixedSlot"));
        setUiAttr(this.format, "aria-label", uiMessage("hostedContentImport.format"));
        setUiAttr(this.file, "aria-label", uiMessage("hostedContentImport.original"));
        setUiText(this.inspectButton, uiMessage("hostedContentImport.inspectOriginal"));
        setUiText(this.prepareButton, uiMessage("hostedContentImport.prepareFixedSlotImport"));
        this.file.type = "file";
        this.slot.onchange = () => { if (epoch !== this.epoch || !this.slot.isConnected) return; this.draft.slot = this.slot.value; this.inputChanged(); };
        this.format.onchange = () => { if (epoch !== this.epoch || !this.format.isConnected) return; this.draft.format = this.format.value as ContentFormat; this.inputChanged(); };
        this.file.onchange = () => {
            if (epoch !== this.epoch || !this.file.isConnected) return;
            const file = this.file.files?.[0];
            if (file)
                void this.chooseFile(file);
        };
        this.host.replaceChildren(ui("h2", "", uiMessage("hostedContentImport.title")), ui("p", "hint", uiMessage("hostedContentImport.description")), action(uiMessage("hostedContentImport.refreshSlots"), () => { void this.run(() => this.refresh()); }), this.slot, this.format, this.file, this.details, this.inspectButton, this.prepareButton, this.message, this.inspection, this.records, this.memberView);
        this.render();
        await this.refresh();
        if (epoch === this.epoch && this.draft.job && !this.unreadable)
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
            this.clear(false);
            this.owner.authorizationChanged(uiMessage("hostedContentImport.authorizationChanged"), current);
            return false;
        }
        return true;
    }
    private persist(value = this.draft): void {
        if (!this.key || this.unreadable)
            throw new UiError(uiMessage("hostedContentImport.recoveryNotWritable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(value));
        }
        catch {
            throw new UiError(uiMessage("hostedContentImport.originalRequestNotDurable"));
        }
    }
    private keep(): void {
        try {
            this.persist();
        }
        catch (e) {
            setUiText(this.message, e instanceof Error ? errorUiText(e) : uiMessage("error.invalidResponse"));
        }
    }
    private inputChanged(): void { this.draft.revision++; this.inspected = undefined; this.reviewedRevision = undefined; this.acceptedLosses = false; this.members = undefined; this.keep(); this.render(); }
    private async chooseFile(file: File): Promise<void> {
        const epoch = this.epoch;
        this.original = file;
        this.reading = true;
        this.draft.fileName = file.name;
        this.draft.bytes = file.size;
        this.draft.bytesBase64 = undefined;
        this.inputChanged();
        try {
            if (file.size > 16 * 1024 * 1024)
                throw new UiError(uiMessage("hostedContentImport.originalSizeLimit"));
            const bytes = new Uint8Array(await file.arrayBuffer());
            if (epoch !== this.epoch || this.original !== file)
                return;
            let binary = "";
            for (let i = 0; i < bytes.length; i += 32768)
                binary += String.fromCharCode(...bytes.subarray(i, i + 32768));
            this.draft.bytesBase64 = btoa(binary);
            this.draft.bytes = bytes.length;
            this.inputChanged();
            setUiText(this.message, uiMessage("hostedContentImport.originalReadNotPrepared"));
        }
        catch (e) {
            if (epoch === this.epoch && this.original === file)
                setUiText(this.message, e instanceof Error ? errorUiText(e) : uiMessage("error.invalidResponse"));
        }
        finally {
            if (epoch === this.epoch && this.original === file) {
                this.reading = false;
                this.render();
            }
        }
    }
    private async run(work: () => Promise<void>): Promise<void> {
        if (this.busy)
            return;
        const epoch = this.epoch;
        this.busy = true;
        this.render();
        try {
            await work();
        }
        catch (e) {
            if (epoch === this.epoch) {
                this.verified = false;
                this.reviewedRevision = undefined;
                this.members = undefined;
                setUiText(this.message, e instanceof Error ? errorUiText(e) : uiMessage("error.invalidResponse"));
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
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<{
            scope: string;
            targets: ContentTarget[];
        }>("import");
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.members = undefined;
        this.targets = decodeContentTargets(value);
        const original = this.draft.job?.request;
        if (original && !this.targets.some(target => target.id === original.slot && target.formats.includes(original.format))) {
            this.verified = false;
            this.reviewedRevision = undefined;
            this.acceptedLosses = false;
        }
        this.render();
    }
    private async inspect(): Promise<void> {
        const revision = this.draft.revision, epoch = this.epoch, identity = this.owner.identity(), target = this.targets.find(t => t.id === this.draft.slot), bytesBase64 = this.draft.bytesBase64;
        if (!target?.formats.includes(this.draft.format) || !target.available || typeof bytesBase64 !== "string" || this.reading)
            throw new UiError(uiMessage("hostedContentImport.selectSlotFormatOriginal"));
        this.persist();
        const body = { slot: this.draft.slot, format: this.draft.format, bytesBase64 }, digest = await sha(contentBytes(bytesBase64));
        if (epoch !== this.epoch || revision !== this.draft.revision || !sameHostedLinksIdentity(identity!, this.owner.identity()))
            return;
        const value = await this.request<ContentInspection>("import/inspect", body);
        if (!await this.authority(identity) || epoch !== this.epoch || revision !== this.draft.revision)
            return;
        const inspected = contentInspection(value, digest);
        this.inspected = { revision, value: inspected };
        setUiText(this.message, inspected.canPrepare ? uiMessage("hostedContentImport.originalCanCreate") : inspected.reason ? descriptionUiText(inspected.reason) : uiMessage("hostedContentImport.originalConflictsOrUnmappedReferences"));
    }
    private accept(value: ContentReceipt): void {
        const job = this.draft.job;
        if (!job)
            return;
        job.receipt = contentReceipt(job.request, value, job.receipt);
        this.members = undefined;
        job.phase = undefined;
        this.verified = true;
        this.reviewedRevision = undefined;
        this.acceptedLosses = false;
        this.keep();
        setUiText(this.message, value.status === "committed" ? uiMessage("hostedContentImport.committedDraftRetained") : value.status === "prepared" ? uiMessage("hostedContentImport.uploadPlanPrepared") : uiMessage("hostedContentImport.factsCheckedNotFalseSuccess"));
    }
    private async prepare(): Promise<void> {
        const request = freezeContentImport(this.draft, this.targets.find(t => t.id === this.draft.slot), this.inspected, crypto.randomUUID()), epoch = this.epoch, identity = this.owner.identity();
        const next: ContentDraft = { ...this.draft, job: { request, fileName: this.draft.fileName!, revision: this.draft.revision, phase: "prepare" } };
        this.persist(next);
        this.draft = next;
        this.verified = false;
        const value = await this.request<ContentReceipt>("import/prepare", request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async lookup(): Promise<void> {
        const job = this.draft.job;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<ContentReceipt>("import/operation", { slot: job.request.slot, operationId: job.request.operationId });
        if (await this.authority(identity) && epoch === this.epoch) {
            this.accept(value);
            if (value.status === "committed")
                await this.refresh();
        }
    }
    private async retry(): Promise<void> {
        const job = this.draft.job;
        if (!job)
            return;
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<ContentReceipt>("import/prepare", job.request);
        if (await this.authority(identity) && epoch === this.epoch) {
            this.accept(value);
            if (value.status === "committed")
                await this.refresh();
        }
    }
    private async commit(): Promise<void> {
        const job = this.draft.job;
        if (!job || !canCommitContent(this.draft, this.verified, this.reviewedRevision) || !this.acceptedLosses)
            throw new UiError(uiMessage("hostedContentImport.checkOriginalUploadLosses"));
        job.commit ??= { slot: job.request.slot, operationId: job.request.operationId, planId: job.receipt!.planId!, acceptLosses: true };
        job.phase = "commit";
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<ContentReceipt>("import/commit", job.commit);
        if (await this.authority(identity) && epoch === this.epoch) {
            this.accept(value);
            if (value.status === "committed")
                await this.refresh();
        }
    }
    private async settle(): Promise<void> {
        const job = this.draft.job;
        if (!job)
            return;
        job.phase = "settle";
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<ContentReceipt>("import/settle", { slot: job.request.slot, operationId: job.request.operationId });
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async loadMembers(): Promise<void> {
        const job = this.draft.job;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<ContentMembers>("import/members", { slot: job.request.slot });
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.members = decodeContentMembers(value, job.request.slot);
    }
    private render(): void {
        if (!this.key)
            return;
        this.host.hidden = false;
        replaceUiChildren(this.slot);
        const selected = this.draft.slot, options: [
            string,
            UiText
        ][] = [["", uiMessage("hostedContentImport.selectAuthorizedEmptySlot")], ...this.targets.map<[string, UiText]>(t => [t.id, () => `${t.title}${uiTextValue(t.available ? "" : uiMessage("hostedContentImport.targetUnavailableSuffix"))}`])];
        if (selected && !this.targets.some(t => t.id === selected))
            options.push([selected, uiMessage("hostedContentImport.originalSlotUnavailable")]);
        for (const [id, title] of options) {
            const option = ui("option", "", title);
            option.value = id;
            this.slot.append(option);
        }
        this.slot.value = selected;
        replaceUiChildren(this.format, ...contentFormats.map(f => { const option = ui("option", "", labels[f]); option.value = f; return option; }));
        this.format.value = this.draft.format;
        this.slot.disabled = this.format.disabled = this.file.disabled = !!this.unreadable;
        setUiText(this.details, this.draft.fileName ? displayMessage("hostedContentImport.currentOriginalSummary", { p0: uiDisplayParameter(this.draft.fileName), p1: uiDisplayParameter(this.draft.bytes ?? uiMessage("common.unknown")), p2: uiDisplayParameter(this.reading ? uiMessage("hostedContentImport.readingSuffix") : "") }) : uiMessage("hostedContentImport.selectOriginalHint"));
        const target = this.targets.find(t => t.id === selected);
        this.inspectButton.disabled = this.busy || this.reading || !!this.unreadable || !target?.available || !target.formats.includes(this.draft.format) || typeof this.draft.bytesBase64 !== "string";
        this.prepareButton.disabled = this.inspectButton.disabled || !!this.draft.job || this.inspected?.revision !== this.draft.revision || !this.inspected?.value.canPrepare;
        replaceUiChildren(this.inspection);
        if (this.unreadable)
            this.inspection.append(ui("p", "hint", this.unreadable), action(uiMessage("hostedContentImport.discardCorruptRecovery"), () => { void this.run(async () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.file.value = ""; this.original = undefined; this.inspected = undefined; this.verified = false; }); }));
        const inspected = this.inspected?.value;
        if (inspected) {
            this.inspection.append(ui("h3", "", uiMessage("hostedContentImport.exactOriginalInspection")), ui("p", "", displayMessage("hostedContentImport.originalInspectionSummary", { p0: uiDisplayParameter(inspected.uploadDigest), p1: uiDisplayParameter(inspected.recordCount ?? inspected.records?.length ?? uiMessage("common.unknown")), p2: uiDisplayParameter(inspected.canPrepare ? uiMessage("hostedContentImport.canPrepareCreation") : uiMessage("hostedContentImport.cannotPrepare")) })));
            if (inspected.reason)
                this.inspection.append(ui("p", "hint", descriptionUiText(inspected.reason)));
            if (inspected.records) this.inspection.append(ui("pre", "", JSON.stringify(inspected.records, null, 2)));
            for (const loss of inspected.losses) this.inspection.append(ui("code", "", loss.code), ui("p", "hint", importLossUiText(loss)));
        }
        replaceUiChildren(this.records);
        replaceUiChildren(this.memberView);
        const job = this.draft.job;
        if (!job)
            return;
        const receipt = this.verified ? job.receipt : undefined, card = ui("section", "content-import-pending");
        card.append(ui("h3", "", uiMessage("hostedContentImport.originalOperation")), ui("code", "", job.request.operationId), ui("p", "", displayMessage("hostedContentImport.originalUploadSummary", { p0: uiDisplayParameter(job.request.slot), p1: uiDisplayParameter(labels[job.request.format]), p2: uiDisplayParameter(job.fileName), p3: contentBytes(job.request.bytesBase64).length })), ui("p", "", displayMessage("hostedStructure.operationStatus", { p0: uiDisplayParameter(receipt?.status ?? uiMessage("hostedContentImport.idRetainedAwaitingAuthorization")) })));
        if (job.revision !== this.draft.revision)
            card.append(ui("p", "hint", uiMessage("hostedContentImport.laterInputRetainedOriginalPlanOnly")));
        if (receipt) {
            if (receipt.preview.summary)
                card.append(ui("p", "", receipt.preview.summary));
            if (receipt.preview.recordCount !== undefined)
                card.append(ui("p", "", displayMessage("hostedContentImport.planRecordCount", { p0: uiDisplayParameter(receipt.preview.recordCount) })));
            if (receipt.preview.losses)
                for (const loss of receipt.preview.losses) card.append(ui("code", "", loss.code), ui("p", "hint", importLossUiText(loss, true)));
            if (receipt.preview.sharedWrites?.length) card.append(ui("pre", "", receipt.preview.sharedWrites.join("\n")));
        }
        const consent = ui("input");
        consent.type = "checkbox";
        setUiAttr(consent, "aria-label", uiMessage("hostedContentImport.lossConsentAria"));
        consent.checked = this.reviewedRevision === this.draft.revision && this.acceptedLosses;
        consent.disabled = this.busy || !receipt?.commitAllowed;
        consent.onchange = () => { this.reviewedRevision = consent.checked ? this.draft.revision : undefined; this.acceptedLosses = consent.checked; this.render(); };
        const consentLabel = ui("label", "", uiMessage("hostedContentImport.lossConsentLabel"));
        consentLabel.prepend(consent);
        card.append(consentLabel);
        const commit = action(uiMessage("hostedContentImport.commitOriginalPlan"), () => { void this.run(() => this.commit()); });
        commit.disabled = this.busy || !canCommitContent(this.draft, this.verified, this.reviewedRevision) || !this.acceptedLosses;
        const lookup = action(uiMessage("hostedContentImport.queryOriginalOperation"), () => { void this.run(() => this.lookup()); });
        lookup.disabled = this.busy;
        const retry = action(uiMessage("hostedContentImport.retryCompleteOriginalRequest"), () => { void this.run(() => this.retry()); });
        retry.disabled = this.busy || contentEnded(receipt?.status);
        const settle = action(uiMessage("hostedContentImport.endUninstalledOriginal"), () => { void this.run(() => this.settle()); });
        settle.disabled = this.busy || contentEnded(receipt?.status);
        card.append(commit, lookup, retry, settle);
        if (receipt?.status === "committed") {
            card.append(ui("p", "hint", uiMessage("hostedContentImport.locatorNotReadPermission")), ui("pre", "", JSON.stringify(receipt.nodeRefs, null, 2)), action(uiMessage("hostedContentImport.viewAuthorizedMembers"), () => { void this.run(() => this.loadMembers()); }));
        }
        const fresh = action(uiMessage("hostedContentImport.endCompletedReceiptKeepOriginal"), () => {
            void this.run(async () => {
                if (!contentEnded(this.verified ? this.draft.job?.receipt?.status : undefined))
                    return;
                const next = { ...this.draft, job: undefined };
                this.persist(next);
                this.draft = next;
                this.verified = false;
                this.reviewedRevision = undefined;
                this.members = undefined;
            });
        });
        fresh.disabled = this.busy || !contentEnded(receipt?.status);
        card.append(fresh);
        this.records.append(card);
        if (this.members) {
            this.memberView.append(ui("h3", "", uiMessage("hostedContentImport.authorizedMembers")), ui("p", "hint", this.members.coverage === "partial" ? uiMessage("hostedContentImport.membersPartial") : uiMessage("hostedContentImport.membersScopeDescription")));
            for (const member of this.members.members) {
                const button = action(displayMessage("hostedContentImport.openMember", { p0: uiDisplayParameter(member.title ?? member.resource) }), () => {
                    void this.run(async () => {
                        const epoch = this.epoch, identity = this.owner.identity();
                        if (member.canOpen && member.status === "ready" && await this.authority(identity) && epoch === this.epoch)
                            await this.owner.open(member.resource);
                    });
                });
                button.disabled = this.busy || !member.canOpen || member.status !== "ready";
                this.memberView.append(button);
            }
        }
    }
}
