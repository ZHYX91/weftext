import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { backupChunk, backupEnded, backupReceipt, canCommitBackup, canDownloadBackup, decodeBackupJob, type BackupCatalog, type BackupChunk, type BackupJob, type BackupReceipt } from "./hosted-owner-backup-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedOwnerBackupHost {
    identity(): HostedLinksIdentity | null;
    authorizationChanged(message: UiText, current: {
        actor: string;
        deployment: string;
        generation: number;
    }): void;
}
/** Recovery material delivery is separate from native content export. */
export class HostedOwnerBackup {
    private epoch = 0;
    private key = "";
    private job?: BackupJob;
    private catalog?: BackupCatalog;
    private busy = false;
    private verified = false;
    private unreadable?: UiText;
    private reviewed = false;
    private message = ui("p", "hint");
    private panel = ui("section");
    private url?: string;
    constructor(private host: HTMLElement, private request: Request, private owner: HostedOwnerBackupHost) { this.clear(false); }
    clear(removeCache = true): void {
        this.epoch++;
        this.key = "";
        this.job = undefined;
        this.catalog = undefined;
        this.busy = this.verified = this.reviewed = false;
        this.unreadable = undefined;
        setUiText(this.message, "");
        this.panel.replaceChildren();
        this.host.replaceChildren();
        this.host.hidden = true;
        this.releaseUrl();
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-owner-backup:"))
                    sessionStorage.removeItem(key);
    }
    private releaseUrl(): void {
        if (this.url)
            URL.revokeObjectURL(this.url);
        this.url = undefined;
    }
    async connect(deployment: string, actor: string, generation: number): Promise<void> {
        this.clear(false);
        const epoch = this.epoch, identity = this.owner.identity();
        if (!identity || identity.actor !== actor || identity.deployment !== deployment || identity.generation !== generation)
            return;
        this.key = hostedCachePrefix("owner-backup", deployment, actor, generation) + "state";
        try {
            const raw = sessionStorage.getItem(this.key);
            if (raw)
                this.job = decodeBackupJob(raw);
        }
        catch {
            this.unreadable = uiMessage("hostedBackup.recoveryUnparseable");
        }
        this.render();
        await this.refresh();
        if (epoch === this.epoch && this.job && !this.unreadable)
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
            this.owner.authorizationChanged(uiMessage("hostedBackup.authorizationChanged"), current);
            return false;
        }
        return true;
    }
    private persist(value: BackupJob | null | undefined = this.job): void {
        if (!this.key || this.unreadable)
            throw new UiError(uiMessage("hostedBackup.recoveryNotWritable"));
        try {
            if (value)
                sessionStorage.setItem(this.key, JSON.stringify(value));
            else
                sessionStorage.removeItem(this.key);
        }
        catch {
            throw new UiError(uiMessage("hostedBackup.requestNotDurable"));
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
        const epoch = this.epoch;
        this.busy = true;
        this.render();
        try {
            await work();
        }
        catch (e) {
            if (epoch === this.epoch) {
                this.verified = this.reviewed = false;
                this.releaseUrl();
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
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupCatalog>("owner-backup");
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        if (!value || typeof value.enabled !== "boolean" || value.scope !== "completeSelectedLibraryRecoveryMaterials" || value.automaticRestore !== false || value.rights && [value.rights.read, value.rights.export, value.rights.download].some(v => typeof v !== "boolean"))
            throw new UiError(uiMessage("hostedBackup.eligibilityUnconfirmed"));
        this.catalog = value;
        this.render();
    }
    private accept(value: BackupReceipt): void {
        if (!this.job)
            return;
        this.job.receipt = backupReceipt(this.job.request, value, this.job.receipt);
        this.job.phase = undefined;
        this.verified = true;
        this.reviewed = false;
        this.keep();
        setUiText(this.message, value.status === "committed" ? value.artifact ? uiMessage("hostedBackup.generatedOriginalArtifact") : uiMessage("hostedBackup.committedArtifactUnavailable") : value.status === "prepared" ? uiMessage("hostedBackup.planPrepared") : uiMessage("hostedBackup.factsCheckedNotFalseSuccess"));
    }
    private async prepare(): Promise<void> {
        if (this.job || !this.catalog?.enabled || !this.catalog.rights?.export)
            throw new UiError(uiMessage("hostedBackup.generationNotPermitted"));
        const job: BackupJob = { request: { requestId: crypto.randomUUID() }, phase: "prepare" };
        this.persist(job);
        this.job = job;
        this.verified = false;
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupReceipt>("owner-backup/prepare", job.request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async lookup(): Promise<void> {
        const job = this.job;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupReceipt>("owner-backup/status", job.request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async retry(): Promise<void> {
        const job = this.job;
        if (!job)
            return;
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupReceipt>("owner-backup/prepare", job.request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async commit(): Promise<void> {
        const job = this.job;
        if (!job || !this.reviewed || !canCommitBackup(job, this.verified, this.catalog))
            throw new UiError(uiMessage("hostedBackup.checkOriginalPlan"));
        job.commit ??= { requestId: job.request.requestId, planId: job.receipt!.planId! };
        job.phase = "commit";
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupReceipt>("owner-backup/commit", job.commit);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async settle(): Promise<void> {
        const job = this.job;
        if (!job)
            return;
        job.phase = "settle";
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<BackupReceipt>("owner-backup/settle", job.request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.accept(value);
    }
    private async close(): Promise<void> {
        const job = this.job;
        if (!job || !this.verified || job.receipt?.status !== "committed")
            return;
        job.phase = "close";
        this.persist();
        const epoch = this.epoch, identity = this.owner.identity();
        const value = await this.request<{
            requestId: string;
            delivery: string;
            originalOperationRetained: boolean;
        }>("owner-backup/close", job.request);
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        if (value.requestId !== job.request.requestId || value.delivery !== "closed" || value.originalOperationRetained !== true)
            throw new UiError(uiMessage("hostedBackup.closeReceiptUnconfirmed"));
        this.releaseUrl();
        await this.lookup();
    }
    private async download(): Promise<void> {
        const job = this.job;
        if (!job || !canDownloadBackup(job, this.verified, this.catalog))
            throw new UiError(uiMessage("hostedBackup.artifactNotAuthorizedForDownload"));
        const epoch = this.epoch, identity = this.owner.identity(), artifact = structuredClone(job.receipt!.artifact!), chunks: Uint8Array<ArrayBuffer>[] = [];
        this.persist();
        this.releaseUrl();
        let offset = 0;
        do {
            const value = await this.request<BackupChunk>("owner-backup/download", { requestId: job.request.requestId, digest: artifact.digest, offset });
            if (!await this.authority(identity) || epoch !== this.epoch || this.job !== job)
                return;
            const bytes = backupChunk(job.request, artifact, offset, value);
            chunks.push(bytes as Uint8Array<ArrayBuffer>);
            offset = value.nextOffset;
            if (value.done)
                break;
        } while (offset < artifact.length);
        const blob = new Blob(chunks, { type: artifact.contentType });
        if (blob.size !== artifact.length)
            throw new UiError(uiMessage("hostedBackup.downloadLengthIncomplete"));
        const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()))].map(v => v.toString(16).padStart(2, "0")).join("");
        if (digest !== artifact.digest)
            throw new UiError(uiMessage("hostedBackup.downloadDigestMismatch"));
        if (!await this.authority(identity) || epoch !== this.epoch || this.job !== job)
            return;
        this.url = URL.createObjectURL(blob);
        const link = ui("a");
        link.href = this.url;
        link.download = artifact.filename;
        document.body.append(link);
        link.click();
        link.remove();
        this.releaseUrl();
        setUiText(this.message, displayMessage("hostedBackup.verifiedDelivery", { p0: uiDisplayParameter(blob.size) }));
    }
    private render(): void {
        if (!this.key)
            return;
        this.host.hidden = !this.catalog?.enabled && !this.job && !this.unreadable;
        if (this.host.hidden)
            return;
        this.host.replaceChildren(ui("h2", "", uiMessage("hostedBackup.title")), ui("p", "hint", uiMessage("hostedBackup.description")), this.message, this.panel);
        this.panel.replaceChildren();
        if (this.unreadable) {
            this.panel.append(ui("p", "hint", this.unreadable), action(uiMessage("hostedBackup.discardCorruptRecovery"), () => { void this.run(async () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.job = undefined; }); }));
            return;
        }
        const prepare = action(uiMessage("hostedBackup.prepareBackup"), () => { void this.run(() => this.prepare()); });
        prepare.disabled = this.busy || !!this.job || !this.catalog?.rights?.export;
        const refresh = action(uiMessage("hostedBackup.refreshEligibility"), () => { void this.run(() => this.refresh()); });
        refresh.disabled = this.busy;
        this.panel.append(prepare, refresh);
        const job = this.job;
        if (!job)
            return;
        const receipt = this.verified ? job.receipt : undefined, card = ui("section", "owner-backup-pending");
        card.append(ui("h3", "", uiMessage("hostedBackup.originalRequest")), ui("code", "", job.request.requestId), ui("p", "", displayMessage("hostedStructure.operationStatus", { p0: uiDisplayParameter(receipt?.status ?? uiMessage("hostedBackup.idAwaitingAuthorization")) })));
        if (receipt?.summary)
            card.append(ui("p", "", receipt.summary));
        if (receipt?.status === "committed" && !receipt.artifact)
            card.append(ui("p", "hint", uiMessage("hostedBackup.committedArtifactUnavailableNoReplacement")));
        const consent = ui("input");
        consent.type = "checkbox";
        setUiAttr(consent, "aria-label", uiMessage("hostedBackup.consentAria"));
        consent.checked = this.reviewed;
        consent.disabled = this.busy || !canCommitBackup(job, this.verified, this.catalog);
        consent.onchange = () => { this.reviewed = consent.checked; this.render(); };
        const label = ui("label", "", uiMessage("hostedBackup.consentLabel"));
        label.prepend(consent);
        card.append(label);
        const commit = action(uiMessage("hostedBackup.generateOriginalPlan"), () => { void this.run(() => this.commit()); });
        commit.disabled = this.busy || !this.reviewed || !canCommitBackup(job, this.verified, this.catalog);
        const lookup = action(uiMessage("hostedBackup.queryOriginalRequest"), () => { void this.run(() => this.lookup()); });
        lookup.disabled = this.busy;
        const retry = action(uiMessage("hostedBackup.retryOriginalPreparation"), () => { void this.run(() => this.retry()); });
        retry.disabled = this.busy || backupEnded(receipt?.status);
        const settle = action(uiMessage("hostedBackup.endUninstalledPlan"), () => { void this.run(() => this.settle()); });
        settle.disabled = this.busy || backupEnded(receipt?.status) || !this.catalog?.rights?.export;
        card.append(commit, lookup, retry, settle);
        if (receipt?.artifact)
            card.append(ui("p", "", displayMessage("office.outputFileSummary", { p0: uiDisplayParameter(receipt.artifact.filename), p1: uiDisplayParameter(receipt.artifact.length) })), ui("code", "", receipt.artifact.digest));
        const download = action(uiMessage("hostedBackup.downloadOriginalBackup"), () => { void this.run(() => this.download()); });
        download.disabled = this.busy || !canDownloadBackup(job, this.verified, this.catalog);
        const close = action(uiMessage("hostedBackup.closeArtifact"), () => { void this.run(() => this.close()); });
        close.disabled = this.busy || receipt?.status !== "committed" || !receipt.artifact;
        card.append(download, close);
        const fresh = action(uiMessage("hostedBackup.endCompletedReceipt"), () => {
            void this.run(async () => {
                if (!backupEnded(this.verified ? this.job?.receipt?.status : undefined))
                    return;
                this.persist(null);
                this.job = undefined;
                this.verified = this.reviewed = false;
                this.releaseUrl();
            });
        });
        fresh.disabled = this.busy || !backupEnded(receipt?.status);
        card.append(fresh);
        this.panel.append(card);
    }
}
