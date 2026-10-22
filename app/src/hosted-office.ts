import { contentLanguageControl, DEFAULT_CONTENT_LOCALE, isContentLocale } from "./content-language";
import { disposeUiTree, joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, formatNativeValue, ui, type Cell } from "./data-panel";
import { OutputReadGate, type DocxImportAnalysis, type LegacyCapabilities, type OutputFile, type TemplateInspection } from "./export-state";
import { freezeOfficeImportRequest, importReceipt, officeChunk, officeFormats, officeOutputPresentation, outputReceipt, type OfficeBinding, type OfficeDraft, type OfficeFormat, type OfficeImportFormat, type OfficeImportReceipt, type OfficeImportRequest, type OfficeOutputReceipt, type OfficeOutputRequest, type OfficeOutputVerification } from "./hosted-office-state";
import { backendLabel, captionKinds, kindLabel, legacyBackendChoices, selectedCaptionOptions, templateInspectionFormat, validDocxAnalysis, type CaptionChoice } from "./office-import-options";
import { hostedCachePrefix } from "./hosted-state";
import { nodeSourceHash } from "./hosted-navigation-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import type { CitationWork } from "./hosted-citations-state";
import type { EmbeddedQueryBasis } from "./embedded-query";
import { descriptionUiText } from "./ui-diagnostics";
import { decodeFrozenOfficeArtifact, decodeLegacyCapabilities, decodeTemplateInspection, exportErrorUiText, legacyMetadataUiText, officeLossUiText, officeOutputUiText, officePreviewFacts, officePreviewNotices, type FrozenOfficeArtifact, type LegacyMetadata, type OfficeLoss, type OfficePreview } from "./office-wire";
export type HostedOfficeContext = {
    resource: string;
    source: string;
    sourceHash?: string;
    dirty: boolean;
    pending: boolean;
    sessionId?: string;
    baseId?: string;
    serial?: number;
    paused?: boolean;
};
export interface HostedOfficeHost {
    identity(): HostedLinksIdentity | null;
    basis(): Promise<EmbeddedQueryBasis>;
    embeddedQueries(): {
        occurrenceId: string;
        queryId: string;
    }[];
    authorizationChanged(message: UiText, current: {
        actor: string;
        deployment: string;
        generation: number;
    }): void;
}
export type HostedOfficeQuery = {
    queryId: string;
    columns: string[];
    rowCount: number;
    coverage: string;
};
type Resource = {
    resource: string;
    title: string;
    export: boolean;
    download: boolean;
    formats?: OfficeFormat[];
    downloads?: OfficeFormat[];
};
type Target = {
    resource: string;
    title: string;
    available?: boolean;
    formats?: OfficeImportFormat[];
};
type NativeCapability = {
    enabled: boolean;
    read: boolean;
    export: boolean;
    download: boolean;
    scope: "savedWholeLibrary";
};
type Request = <T>(path: string, body?: unknown) => Promise<T>;
type OutputJob = {
    request: OfficeOutputRequest;
    receipt?: OfficeOutputReceipt;
    frozenArtifact?: FrozenOfficeArtifact;
    confirm?: {
        requestId: string;
        digest: string;
        acceptLosses: boolean;
    };
    acceptLosses?: boolean;
    phase?: "prepare" | "confirm" | "download";
    query?: HostedOfficeQuery;
    received?: boolean;
};
type ImportJob = {
    request: OfficeImportRequest;
    receipt?: OfficeImportReceipt;
    /** Original plan identity is a recovery locator, never current commit authority. */
    planId?: string;
    name: string;
    commit?: {
        resource: string;
        operationId: string;
        planId: string;
        acceptLosses: boolean;
    };
    acceptLosses?: boolean;
    phase?: "prepare" | "commit";
    notified?: boolean;
};
type Cache = {
    draft: OfficeDraft;
    output?: OutputJob;
    importDraft: {
        resource: string;
        file?: OutputFile;
        format?: OfficeImportFormat;
        legacyBackend?: string;
        analysis?: DocxImportAnalysis;
        captionChoices?: CaptionChoice[];
    };
    importJob?: ImportJob;
};
const initial = (): Cache => ({ draft: { outputLocale: DEFAULT_CONTENT_LOCALE, resource: "", format: "docx", reviewMode: "none", includeResolved: false, queryId: "", stylePolicy: "template", bindings: {} }, importDraft: { resource: "" } });
const terminalImport = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
const outputDone = (status?: string) => ["cancelled", "failed", "expired", "notRegistered"].includes(status ?? "");
const isLegacy = (format: string): boolean => ["doc", "wps"].includes(format);
const isWord = (format: string): boolean => ["docx", "doc", "wps"].includes(format);
function select(label: UiText, options: [
    string,
    UiText
][], current: string) {
    const control = ui("select", "data-input");
    setUiAttr(control, "aria-label", label);
    for (const [value, text] of options) {
        const item = ui("option", "", text);
        item.value = value;
        control.append(item);
    }
    control.value = current;
    return control;
}
function checkbox(label: UiText, checked: boolean, changed: (value: boolean) => void) {
    const input = ui("input");
    input.type = "checkbox";
    input.checked = checked;
    setUiAttr(input, "aria-label", label);
    input.onchange = () => changed(input.checked);
    const host = ui("label", "data-label", label);
    host.prepend(input);
    return host;
}
async function fileBytes(file: File, limit: number): Promise<OutputFile> {
    if (file.size > limit)
        throw new UiError(displayMessage("office.fileBudgetExceeded", { p0: uiDisplayParameter(limit / 1024 / 1024) }));
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onerror = () => reject(new UiError(uiMessage("office.fileReadFailed")));
        reader.onload = () => typeof reader.result === "string" && reader.result.includes(",") ? resolve({ name: file.name, bytesBase64: reader.result.slice(reader.result.indexOf(",") + 1) }) : reject(new UiError(uiMessage("office.fileBytesIncomplete")));
        reader.readAsDataURL(file);
    });
}
async function sha256(blob: Blob) { return [...new Uint8Array(await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()))].map(b => b.toString(16).padStart(2, "0")).join(""); }
function losses(items: OfficeLoss[]) {
    const host = ui("section", "output-losses");
    host.append(ui("h4", "", items.length ? displayMessage("office.lossCount", { p0: items.length }) : uiMessage("office.noReportedLosses")));
    const list = ui("ul");
    for (const item of items)
        list.append(ui("li", "", officeLossUiText(item)));
    host.append(list);
    return host;
}
function conversionDetails(value: LegacyMetadata): HTMLElement {
    const section = ui("details", "output-detail");
    section.append(ui("summary", "", uiMessage("office.viewServerConversionDetails")));
    legacyMetadataUiText(value).forEach(notice => section.append(ui("p", "hint", notice)));
    const { explanation, diagnostics, causes, details, features, losses, ...facts } = value;
    section.append(ui("pre", "output-source", JSON.stringify(facts, null, 2)));
    return section;
}
/** Only text and typed cells are drawn; no returned markup is executed. */
function preview(value: OfficePreview | null, columns: string[] = []) {
    const section = ui("details", "output-detail");
    section.append(ui("summary", "", uiMessage("office.viewExactOutputPreview")));
    if (!value || typeof value !== "object") {
        if (typeof value === "string")
            section.append(ui("pre", "", value));
        return section;
    }
    officePreviewNotices(value).forEach(notice => section.append(ui("p", "hint", notice)));
    const record = officePreviewFacts(value) as Record<string, unknown>;
    for (const [key, label] of [["title", uiMessage("office.previewTitle")], ["body", uiMessage("office.previewBody")], ["source", uiMessage("office.previewNativeSource")], ["summary", uiMessage("office.previewDescription")]] as [
        string,
        UiText
    ][]) {
        const text = record[key];
        if (typeof text === "string")
            section.append(ui("p", "", label), ui("pre", "", text));
        else if (key === "source" && text && typeof text === "object") {
            const pair = text as {
                before?: string;
                after?: string;
            };
            if (typeof pair.before === "string")
                section.append(ui("p", "", uiMessage("office.previewOriginal")), ui("pre", "", pair.before));
            if (typeof pair.after === "string")
                section.append(ui("p", "", uiMessage("office.previewProposed")), ui("pre", "", pair.after));
        }
    }
    const rows = Array.isArray(record.rows) ? record.rows : [];
    const order = Array.isArray(record.columns) && record.columns.every(c => typeof c === "string") ? record.columns as string[] : columns;
    const query = record.query as {
        rowCount?: number;
        coverage?: string;
    } | undefined;
    if (query && typeof query.rowCount === "number")
        section.append(ui("p", "hint", displayMessage("office.frozenQuerySummary", { p0: query.rowCount, p1: uiDisplayParameter(query.coverage === "partial" ? uiMessage("office.queryIncompleteSuffix") : "") })));
    if (rows.length || order.length) {
        const table = ui("table");
        setUiAttr(table, "aria-label", uiMessage("office.frozenQueryPreviewAria"));
        const names = order.length ? order : Object.keys((rows[0] as {
            cells?: Record<string, Cell>;
        } | undefined)?.cells ?? {});
        const header = ui("tr");
        for (const name of names)
            header.append(ui("th", "", name));
        table.append(header);
        for (const row of rows) {
            const cells = (row as {
                cells?: Record<string, Cell>;
            }).cells ?? {};
            const tr = ui("tr");
            for (const name of names) {
                const cell = cells[name];
                tr.append(ui("td", "", cell?.value ? formatNativeValue(cell.value) : ({ absent: uiMessage("office.cellAbsent"), null: uiMessage("office.cellNone"), invalid: uiMessage("office.cellInvalid"), unavailable: uiMessage("office.cellUnavailable"), value: "" })[cell?.state ?? "unavailable"]));
            }
            table.append(tr);
        }
        section.append(table);
    }
    if (value.legacyConversion) section.append(conversionDetails(value.legacyConversion));
    if (value.legacy) section.append(conversionDetails(value.legacy));
    return section;
}
/** Frozen output jobs and actor import transactions survive this tab's reload. */
export class HostedOffice {
    private epoch = 0;
    private key = "";
    private cache: Cache = initial();
    private resources: Resource[] = [];
    private queries: HostedOfficeQuery[] = [];
    private targets: Target[] = [];
    private works: CitationWork[] = [];
    private native?: NativeCapability;
    private unreadable?: string;
    private outputVerified?: OfficeOutputVerification;
    private importVerified?: {
        operationId: string;
        epoch: number;
    };
    private legacyCapabilities: LegacyCapabilities | null = null;
    private legacyError: UiText = uiMessage("office.serverCapabilitiesMissing");
    private cancelling = false;
    private busy = false;
    private original?: File;
    private templateGate = new OutputReadGate();
    private importGate = new OutputReadGate();
    private message = ui("p", "hint");
    private editor = ui("section");
    private activeUrl?: string;
    constructor(private host: HTMLElement, private request: Request, private context: () => HostedOfficeContext | null, private flush: () => Promise<void>, private changed?: (resource: string) => void, private owner?: HostedOfficeHost) { this.clear(false); }
    clear(removeCache = true) {
        this.epoch++;
        this.templateGate.invalidate();
        this.importGate.invalidate();
        this.busy = false;
        this.cache = initial();
        this.resources = [];
        this.queries = [];
        this.targets = [];
        this.works = [];
        this.native = undefined;
        this.original = undefined;
        this.unreadable = undefined;
        this.outputVerified = undefined;
        this.importVerified = undefined;
        this.legacyCapabilities = null;
        this.legacyError = uiMessage("office.serverCapabilitiesMissing");
        this.cancelling = false;
        if (this.activeUrl)
            URL.revokeObjectURL(this.activeUrl);
        this.activeUrl = undefined;
        disposeUiTree(this.host);
        disposeUiTree(this.editor);
        disposeUiTree(this.message);
        this.host.replaceChildren();
        this.host.hidden = true;
        this.editor.replaceChildren();
        setUiText(this.message, "");
        this.key = "";
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-office:"))
                    sessionStorage.removeItem(key);
        this.key = "";
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch, identity = this.owner?.identity();
        this.key = hostedCachePrefix("office", deployment, actor, generation) + "state";
        if (this.owner && (!identity || identity.deployment !== deployment || identity.actor !== actor || identity.generation !== generation)) {
            this.clear(false);
            return;
        }
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw) {
                const stored = JSON.parse(raw) as Cache;
                if (!stored?.draft || !stored.importDraft || !officeFormats.includes(stored.draft.format) || !isContentLocale(stored.draft.outputLocale) || !stored.draft.bindings)
                    throw new Error();
                if (stored.draft.inspection) stored.draft.inspection = decodeTemplateInspection(stored.draft.inspection, templateInspectionFormat(stored.draft.format));
                if (stored.output?.receipt)
                    stored.output.receipt = outputReceipt(stored.output.request.requestId, stored.output.receipt);
                if (stored.output?.frozenArtifact) {
                    stored.output.frozenArtifact = decodeFrozenOfficeArtifact(stored.output.frozenArtifact);
                    if (stored.output.frozenArtifact.requestId !== stored.output.request.requestId) throw new Error();
                    if (stored.output.receipt) stored.output.receipt = outputReceipt(stored.output.request.requestId, stored.output.receipt, stored.output.frozenArtifact);
                }
                if (stored.output?.confirm && (stored.output.confirm.requestId !== stored.output.request.requestId || stored.output.confirm.digest !== stored.output.frozenArtifact?.digest || typeof stored.output.confirm.acceptLosses !== "boolean")) throw new Error();
                if (stored.importJob) stored.importJob.request = freezeOfficeImportRequest(stored.importJob.request);
                if (stored.importJob?.planId !== undefined && (typeof stored.importJob.planId !== "string" || !stored.importJob.planId)) throw new Error();
                if (stored.importJob?.receipt) {
                    stored.importJob.receipt = importReceipt(stored.importJob.request, stored.importJob.receipt, undefined, stored.importJob.planId);
                    if (stored.importJob.receipt.planId) stored.importJob.planId = stored.importJob.receipt.planId;
                }
                if (stored.importJob?.commit) {
                    const job = stored.importJob, commit = job.commit!;
                    if (commit.resource !== job.request.resource || commit.operationId !== job.request.operationId || typeof commit.planId !== "string" || !commit.planId || typeof commit.acceptLosses !== "boolean" || job.planId && commit.planId !== job.planId) throw new Error();
                    job.planId = commit.planId;
                }
                this.cache = stored;
            }
        }
        catch {
            this.unreadable = raw ?? "";
        }
        this.mount();
        setUiText(this.message, uiMessage("office.checkingAudienceAndCatalog"));
        const results = await Promise.allSettled([this.request<{
                resources: Resource[];
                queries?: HostedOfficeQuery[];
                native?: NativeCapability;
            }>("export"), this.request<{
                targets: Target[];
            }>("office/import-targets"), this.request<{
                works: CitationWork[];
            }>("citations"), this.request<LegacyCapabilities>("office/legacy-capabilities")]);
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        if (results[0].status === "fulfilled") {
            this.resources = results[0].value.resources;
            this.queries = results[0].value.queries ?? [];
            this.native = results[0].value.native;
        }
        if (results[1].status === "fulfilled")
            this.targets = results[1].value.targets;
        if (results[2].status === "fulfilled")
            this.works = results[2].value.works;
        if (results[3].status === "fulfilled")
            this.installLegacyCapabilities(results[3].value);
        const draft = this.cache.draft;
        if (!draft.resource)
            draft.resource = this.context()?.resource ?? this.resources[0]?.resource ?? "";
        if (!this.cache.importDraft.resource)
            this.cache.importDraft.resource = this.targets[0]?.resource ?? "";
        if (results.some(r => r.status === "rejected"))
            setUiText(this.message, uiMessage("office.catalogIncomplete"));
        else if (this.cache.output || this.cache.importJob)
            setUiText(this.message, uiMessage("office.requestRecovered"));
        this.render();
    }
    setQuery(queryId: string, metadata?: Omit<HostedOfficeQuery, "queryId">) {
        if (!this.key || this.unreadable !== undefined)
            return;
        const previous = this.queries.find(q => q.queryId === queryId);
        const item = { queryId, columns: metadata?.columns ?? previous?.columns ?? [], rowCount: metadata?.rowCount ?? previous?.rowCount ?? 0, coverage: metadata?.coverage ?? previous?.coverage ?? "unknown" };
        const existing = this.queries.findIndex(q => q.queryId === queryId);
        if (existing < 0)
            this.queries.push(item);
        else
            this.queries[existing] = item;
        if (!this.cache.output && !this.busy) {
            this.cache.draft.format = "xlsx";
            this.cache.draft.queryId = queryId;
            this.cache.draft.template = undefined;
            this.cache.draft.inspection = undefined;
            this.cache.draft.bindings = {};
            this.keep();
        }
        this.host.hidden = false;
        this.render();
    }
    private mount() { this.host.hidden = false; this.host.replaceChildren(ui("h2", "", uiMessage("office.panelTitle")), ui("p", "hint", uiMessage("office.panelHelp")), this.message, this.editor); }
    private persist() {
        if (!this.key || this.unreadable !== undefined)
            throw new UiError(uiMessage("office.recoveryNotWritable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(this.cache));
        }
        catch {
            throw new UiError(uiMessage("office.recoveryQuotaFailure"));
        }
    }
    private keep() {
        try {
            this.persist();
            return true;
        }
        catch (error) {
            setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("office.recoveryWritePaused"));
            return false;
        }
    }
    private async run(work: () => Promise<void>) {
        if (this.busy)
            return;
        this.busy = true;
        const epoch = this.epoch;
        this.render();
        try {
            await work();
        }
        catch (error) {
            if (epoch === this.epoch) {
                this.outputVerified = undefined;
                this.importVerified = undefined;
                setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("office.operationIncomplete"));
            }
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async authority(identity: HostedLinksIdentity | null | undefined): Promise<boolean> {
        if (!this.owner)
            return true;
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
            this.owner.authorizationChanged(uiMessage("office.authorizationChanged"), current);
            return false;
        }
        return true;
    }
    private allowed(resource: Resource | undefined, format: OfficeFormat): boolean { return !!resource && (resource.formats?.includes(format) ?? (resource.export && format === "docx")); }
    private outputPath(suffix: string): string { return `${["docx", "doc", "wps", "xlsx"].includes(this.cache.output?.request.format ?? this.cache.draft.format) ? "office/export" : "export"}/${suffix}`; }
    private installLegacyCapabilities(result: unknown): void {
        try {
            this.legacyCapabilities = decodeLegacyCapabilities(result, "server");
            this.legacyError = "";
        }
        catch (error) {
            this.legacyCapabilities = null;
            this.legacyError = errorUiText(error);
        }
    }
    private legacyAvailable(format: string, backend: string | undefined, importing: boolean): boolean { return !isLegacy(format) || legacyBackendChoices(this.legacyCapabilities, format, importing).some(choice => choice[0] === backend); }
    private importAllowed(resource: string, format: OfficeImportFormat): boolean { const target = this.targets.find(item => item.resource === resource && item.available !== false); return !!target && (target.formats?.includes(format) ?? format === "docx"); }
    private async reloadLegacyCapabilities(): Promise<void> {
        const epoch = this.epoch, identity = this.owner?.identity();
        const result = await this.request<LegacyCapabilities>("office/legacy-capabilities");
        if (await this.authority(identity) && epoch === this.epoch) {
            this.installLegacyCapabilities(result);
            setUiText(this.message, this.legacyError || uiMessage("office.serverSoftwareRechecked"));
        }
    }
    private renderLegacy(format: string, backend: string | undefined, importing: boolean, changed: (value: string) => void): HTMLElement {
        const section = ui("div", "output-legacy");
        section.append(ui("p", "hint", uiMessage("office.serverExecutionHelp")));
        const choices = legacyBackendChoices(this.legacyCapabilities, format, importing);
        const control = select(importing ? uiMessage("office.importBackendLabel") : uiMessage("office.outputBackendLabel"), [["", uiMessage("office.chooseServerBackend")], ...choices], backend ?? "");
        control.onchange = () => changed(control.value);
        control.disabled = !choices.length;
        section.append(control);
        if (!choices.length)
            section.append(ui("p", "data-warning", this.legacyError || uiMessage("office.noServerBackend")));
        if (backend && !choices.some(item => item[0] === backend))
            section.append(ui("p", "data-warning", uiMessage("office.selectedBackendUnavailable")));
        const direction = format === "doc" ? importing ? "docToDocx" : "docxToDoc" : importing ? "wpsToDocx" : "docxToWps";
        for (const value of this.legacyCapabilities?.backends ?? []) {
            const capability = value.directions.find(item => item.direction === direction);
            section.append(ui("p", "hint", joinUiText([backendLabel(value.id), value.version ? " " + value.version : "", ": ", capability?.available ? displayMessage("office.backendCallable", { p0: uiDisplayParameter(capability.contentRoundtripVerified ? "" : uiMessage("office.backendRoundtripUnverifiedSuffix")) }) : uiMessage("office.cellUnavailable"), ...(capability ? [" (", descriptionUiText(capability), ")"] : [])])));
        }
        section.append(ui("p", "hint", importing ? uiMessage("office.legacyImportIntermediateHelp") : uiMessage("office.legacyOutputIntermediateHelp")), action(uiMessage("office.recheckServerSoftware"), () => this.run(() => this.reloadLegacyCapabilities())));
        return section;
    }
    private async cancelLegacy(importing: boolean): Promise<void> {
        const output = this.cache.output, imported = this.cache.importJob;
        if (this.cancelling || (importing ? !imported || !isLegacy(imported.request.format ?? "docx") : !output || !isLegacy(output.request.format)))
            return;
        const epoch = this.epoch, identity = this.owner?.identity();
        this.cancelling = true;
        this.render();
        try {
            await this.request(importing ? "office/import/cancel" : "office/export/cancel", importing ? { resource: imported!.request.resource, operationId: imported!.request.operationId } : { requestId: output!.request.requestId });
            if (await this.authority(identity) && epoch === this.epoch)
                setUiText(this.message, uiMessage("office.conversionCancellationRequested"));
        }
        catch (error) {
            if (epoch === this.epoch)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("office.cancellationUnconfirmed"));
        }
        finally {
            if (epoch === this.epoch) {
                this.cancelling = false;
                this.render();
            }
        }
    }
    private acceptOutput(result: OfficeOutputReceipt) {
        const job = this.cache.output;
        if (!job)
            return;
        const previous = job.receipt && ["prepared", "confirmed"].includes(job.receipt.status) ? job.receipt : job.frozenArtifact ?? job.receipt;
        const receipt = outputReceipt(job.request.requestId, result, previous);
        job.receipt = receipt;
        this.outputVerified = { requestId: job.request.requestId, epoch: this.epoch };
        if (["prepared", "confirmed"].includes(receipt.status))
            job.frozenArtifact = decodeFrozenOfficeArtifact({ requestId: receipt.requestId, status: receipt.status, filename: receipt.filename, contentType: receipt.contentType, length: receipt.length, digest: receipt.digest });
        job.phase = ["pending", "unknown"].includes(receipt.status) ? "prepare" : undefined;
        if (receipt.status === "confirmed")
            delete job.confirm;
        if (this.keep())
            setUiText(this.message, ({ pending: uiMessage("office.outputStillGenerating"), unknown: uiMessage("office.outputUnconfirmed"), cancelled: uiMessage("office.outputCancelled"), prepared: uiMessage("office.outputPrepared"), confirmed: uiMessage("office.outputConfirmed"), failed: uiMessage("office.outputFailed"), expired: uiMessage("office.outputInvalidated"), notRegistered: uiMessage("office.outputNotRegistered") })[receipt.status]);
    }
    private async prepareOutput() {
        if (this.cache.output)
            return;
        const epoch = this.epoch, identity = this.owner?.identity(), draft = structuredClone(this.cache.draft);
        const bib = ["bibtex", "csl-json", "ris"].includes(draft.format), citationDraft = bib && (draft.bibliographyScope ?? "citationWorks") === "citationWorks" && draft.bibliographySource !== "saved";
        if (!this.legacyAvailable(draft.format, draft.legacyBackend, false))
            throw new UiError(uiMessage("office.outputBackendUnavailable"));
        const body = isWord(draft.format) && (!draft.template || draft.reviewMode !== "none" || Object.values(draft.bindings).some(b => b.kind === "body" || b.kind === "title")) || draft.format === "pdf" || citationDraft;
        if (body)
            await this.flush();
        if (epoch !== this.epoch)
            return;
        const resource = this.resources.find(r => r.resource === draft.resource);
        const office = isWord(draft.format) || draft.format === "xlsx";
        const request: OfficeOutputRequest = { requestId: crypto.randomUUID(), outputLocale: draft.outputLocale, format: draft.format, reviewMode: isWord(draft.format) ? draft.reviewMode : "none", includeResolved: isWord(draft.format) && draft.includeResolved, stylePolicy: draft.stylePolicy, bindings: office ? structuredClone(draft.bindings) : {}, ...(office && draft.template ? { template: structuredClone(draft.template) } : {}), ...(isLegacy(draft.format) ? { legacyBackend: draft.legacyBackend } : {}) };
        if (body) {
            const context = this.context();
            if (!this.allowed(resource, draft.format) || !context || context.paused || context.resource !== draft.resource)
                throw new UiError(uiMessage("office.openAuthorizedBodyRequired"));
            if (isWord(draft.format) && draft.reviewMode !== "none" && (context.pending || context.dirty))
                throw new UiError(uiMessage("office.savedBodyRequiredForReview"));
            request.resource = draft.resource;
            request.source = context.source;
            if (draft.format === "pdf" || citationDraft || draft.includeEmbedded) {
                if (!this.owner || !context.sessionId || !context.baseId || context.serial === undefined)
                    throw new UiError(uiMessage("office.draftSessionUnavailable"));
                const basis = await this.owner.basis(), current = this.context();
                if (!current || context.resource !== current.resource || context.sessionId !== current.sessionId || context.baseId !== current.baseId || context.serial !== current.serial || context.source !== current.source || basis.sessionId !== context.sessionId || basis.serial !== context.serial || basis.source !== context.source)
                    throw new UiError(uiMessage("office.bodyChangedDuringRegistration"));
                Object.assign(request, { sessionId: context.sessionId, baseId: context.baseId, serial: context.serial, sourceHash: await nodeSourceHash(context.source) });
                if (draft.includeEmbedded && ["docx", "doc", "wps", "pdf"].includes(draft.format)) {
                    const selections = this.owner.embeddedQueries();
                    if (!selections.length)
                        throw new UiError(uiMessage("office.embeddedQueryResultMissing"));
                    request.embeddedQueries = structuredClone(selections);
                }
                const afterHash = this.context();
                if (!afterHash || afterHash.source !== context.source || afterHash.serial !== context.serial || afterHash.sessionId !== context.sessionId)
                    throw new UiError(uiMessage("office.inputChangedDuringFreeze"));
            }
            if (isWord(draft.format) && Object.values(draft.bindings).some(binding => binding.kind === "query")) {
                if (!draft.queryId)
                    throw new UiError(uiMessage("office.templateQueryRequired"));
                request.queryId = draft.queryId;
            }
        }
        else if (isWord(draft.format)) {
            if (!this.allowed(resource, draft.format))
                throw new UiError(uiMessage("office.templateFieldTargetUnavailable"));
            request.resource = draft.resource;
            if (draft.includeEmbedded)
                throw new UiError(uiMessage("office.fieldOnlyTemplateHasNoBody"));
            if (Object.values(draft.bindings).some(b => b.kind === "query")) {
                if (!draft.queryId)
                    throw new UiError(uiMessage("office.templateFrozenQueryRequired"));
                request.queryId = draft.queryId;
            }
        }
        else if (draft.format === "xlsx") {
            if (draft.queryId)
                request.queryId = draft.queryId;
            else if (!Object.values(draft.bindings).some(b => b.kind === "field"))
                throw new UiError(uiMessage("office.fieldQueryOrBindingRequired"));
            if (Object.values(draft.bindings).some(b => b.kind === "field")) {
                if (!this.allowed(resource, "xlsx"))
                    throw new UiError(uiMessage("office.fieldTargetUnavailable"));
                request.resource = draft.resource;
            }
        }
        else if (bib) {
            if ((draft.bibliographyScope ?? "citationWorks") === "selectedWorks") {
                const ids = draft.works ?? [];
                if (!ids.length || ids.some(id => !this.works.some(w => w.resource === id && w.status === "ready" && w.export)))
                    throw new UiError(uiMessage("office.authorizedWorkRequired"));
                request.works = [...ids];
            }
            else {
                if (!this.allowed(resource, draft.format))
                    throw new UiError(uiMessage("office.bibliographyTargetUnavailable"));
                request.resource = draft.resource;
            }
        }
        else if (draft.format === "native" && !(this.native?.enabled && this.native.read && this.native.export)) {
            throw new UiError(uiMessage("office.wholeLibraryAuthorizationRequired"));
        }
        if (bib)
            request.bibliographyScope = draft.bibliographyScope ?? "citationWorks";
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.cache.output = { request, phase: "prepare", query: this.queries.find(q => q.queryId === draft.queryId) };
        this.persist(); // No request can begin without its recoverable exact identity/input.
        this.render();
        const result = await this.request<OfficeOutputReceipt>(this.outputPath("prepare"), request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptOutput(result);
    }
    private async recoverOutput() {
        const job = this.cache.output;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner?.identity();
        const result = await this.request<OfficeOutputReceipt>(this.outputPath("status"), { requestId: job.request.requestId });
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptOutput(result);
    }
    private canDownload(request: OfficeOutputRequest): boolean {
        if (request.format === "native")
            return !!this.native?.enabled && this.native.read && this.native.export && this.native.download;
        if (request.bibliographyScope === "selectedWorks")
            return !!request.works?.length && request.works.every(id => this.works.some(w => w.resource === id && w.download));
        if (request.resource) {
            const resource = this.resources.find(r => r.resource === request.resource);
            return !!resource && (resource.downloads?.includes(request.format) ?? (!isLegacy(request.format) && resource.download));
        }
        return request.format === "xlsx" && !!request.queryId;
    }
    private async confirmOutput(acceptLosses: boolean) {
        const job = this.cache.output, receipt = job && officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch);
        if (!job || receipt?.status !== "prepared" || !receipt.digest)
            return;
        if (receipt.losses?.length && !acceptLosses && !job.confirm)
            throw new UiError(uiMessage("office.acceptOutputLossesRequired"));
        if (job.confirm && (job.confirm.requestId !== job.request.requestId || job.confirm.digest !== receipt.digest || typeof job.confirm.acceptLosses !== "boolean"))
            throw Object.assign(new UiError(uiMessage("hostedOfficeState.originalOutputFileChanged")), { code: "resultUnknown" });
        const epoch = this.epoch, identity = this.owner?.identity();
        job.confirm ??= { requestId: job.request.requestId, digest: receipt.digest, acceptLosses };
        job.phase = "confirm";
        this.persist();
        const result = await this.request<OfficeOutputReceipt>(this.outputPath("confirm"), job.confirm);
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptOutput(result);
    }
    private async downloadOutput() {
        const job = this.cache.output, receipt = job && officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch);
        if (!job || receipt?.status !== "confirmed" || !receipt.digest || receipt.length === undefined)
            return;
        const epoch = this.epoch, identity = this.owner?.identity();
        job.phase = "download";
        this.persist();
        const chunks: Uint8Array<ArrayBuffer>[] = [];
        for (let offset = 0; offset < receipt.length;) {
            const result = await this.request<Parameters<typeof officeChunk>[0]>(this.outputPath("chunk"), { requestId: job.request.requestId, offset });
            if (epoch !== this.epoch)
                return;
            const bytes = officeChunk(result, offset, receipt.length, receipt.digest);
            chunks.push(new Uint8Array(bytes));
            offset += bytes.length;
        }
        const blob = new Blob(chunks, { type: receipt.contentType });
        if (blob.size !== receipt.length || await sha256(blob) !== receipt.digest)
            throw new UiError(uiMessage("office.downloadDigestMismatch"));
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.download(blob, receipt.filename!);
        job.phase = undefined;
        job.received = true;
        this.keep();
        setUiText(this.message, uiMessage("office.downloadComplete"));
    }
    private download(blob: Blob, name: string) {
        const url = URL.createObjectURL(blob);
        this.activeUrl = url;
        const link = ui("a");
        link.href = url;
        link.download = name;
        link.click();
        setTimeout(() => {
            URL.revokeObjectURL(url);
            if (this.activeUrl === url)
                this.activeUrl = undefined;
        }, 1000);
    }
    private acceptImport(result: OfficeImportReceipt) {
        const job = this.cache.importJob;
        if (!job)
            return;
        const receipt = importReceipt(job.request, result, job.receipt, job.planId);
        job.receipt = receipt;
        if (receipt.planId) job.planId = receipt.planId;
        this.importVerified = { operationId: job.request.operationId, epoch: this.epoch };
        job.phase = ["pending", "notRegistered", "notCommitted", "recoveryPending"].includes(receipt.status) ? job.phase ?? "prepare" : undefined;
        const retained = this.keep();
        if (receipt.status === "committed") {
            const target = this.targets.find(target => target.resource === job.request.resource);
            if (target)
                target.available = false;
            if (retained)
                setUiText(this.message, uiMessage("office.wordImportCommitted"));
            if (!job.notified) {
                job.notified = true;
                this.keep();
                this.changed?.(job.request.resource);
            }
        }
        else if (retained)
            setUiText(this.message, receipt.status === "prepared" ? uiMessage("office.wordImportPrepared") : terminalImport(receipt.status) ? uiMessage("office.wordImportEnded") : uiMessage("office.wordImportUnconfirmed"));
    }
    private async prepareImport() {
        if (this.cache.importJob)
            return;
        const draft = this.cache.importDraft, file = draft.file, resource = draft.resource, format = draft.format ?? "docx";
        if (!file || !this.importAllowed(resource, format))
            throw new UiError(uiMessage("office.emptyWordTargetRequired"));
        if (!this.legacyAvailable(format, draft.legacyBackend, true))
            throw new UiError(uiMessage("office.importBackendUnavailable"));
        const docxOptions = format === "docx" ? selectedCaptionOptions(draft.analysis ?? null, draft.captionChoices ?? []) : undefined;
        const epoch = this.epoch, identity = this.owner?.identity();
        const request = freezeOfficeImportRequest({ resource, operationId: crypto.randomUUID(), bytesBase64: file.bytesBase64, format, ...(isLegacy(format) ? { legacyBackend: draft.legacyBackend } : {}), ...(docxOptions ? { docxOptions } : {}) });
        if (!await this.authority(identity) || epoch !== this.epoch)
            return;
        this.cache.importJob = { request, name: file.name, phase: "prepare" };
        delete this.cache.importDraft.file;
        this.persist();
        this.render();
        const result = await this.request<OfficeImportReceipt>("office/import/prepare", request);
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptImport(result);
    }
    private async inspectImport(): Promise<void> {
        const draft = this.cache.importDraft, uploaded = draft.file;
        if (!uploaded || (draft.format ?? "docx") !== "docx" || this.cache.importJob)
            return;
        if (!this.importAllowed(draft.resource, "docx"))
            throw new UiError(uiMessage("office.docxImportUnauthorized"));
        const epoch = this.epoch, identity = this.owner?.identity(), generation = this.importGate.begin(), resource = draft.resource;
        delete draft.analysis;
        delete draft.captionChoices;
        this.keep();
        try {
            const analysis = await this.request<DocxImportAnalysis>("office/import/inspect", { resource, format: "docx", bytesBase64: uploaded.bytesBase64 });
            if (!await this.authority(identity) || epoch !== this.epoch || !this.importGate.accepts(generation) || draft !== this.cache.importDraft || uploaded !== draft.file || resource !== draft.resource)
                return;
            if (!validDocxAnalysis(analysis))
                throw new UiError(uiMessage("office.captionAnalysisIncomplete"));
            draft.analysis = analysis;
            draft.captionChoices = analysis.captions.map(caption => ({ enabled: false, captionBlock: caption.blockIndex, objectBlock: "", kind: "", captionText: caption.suggestedText, position: "" }));
            if (this.keep())
                setUiText(this.message, uiMessage("office.captionsInspected"));
        }
        catch (error) {
            if (epoch === this.epoch && this.importGate.accepts(generation))
                setUiText(this.message, displayMessage("office.captionFallbackHelp", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : uiMessage("office.captionInspectionIncomplete")) }));
        }
    }
    private async recoverImport() {
        const job = this.cache.importJob;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner?.identity();
        const request = { resource: job.request.resource, operationId: job.request.operationId };
        let result = await this.request<OfficeImportReceipt>("office/import/operation", request);
        if (epoch !== this.epoch)
            return;
        result = importReceipt(job.request, result, job.receipt, job.planId);
        if (isLegacy(job.request.format ?? "docx") && result.status === "recoveryPending") {
            if (!await this.authority(identity) || epoch !== this.epoch)
                return;
            result = await this.request<OfficeImportReceipt>("office/import/prepare", structuredClone(job.request));
        }
        else if (["notRegistered", "notCommitted"].includes(result.status))
            result = await this.request<OfficeImportReceipt>("office/import/settle", { ...request, format: job.request.format ?? "docx" });
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptImport(result);
    }
    private async endImport(): Promise<void> {
        const job = this.cache.importJob;
        if (!job)
            return;
        const epoch = this.epoch, identity = this.owner?.identity();
        const result = await this.request<OfficeImportReceipt>("office/import/settle", { resource: job.request.resource, operationId: job.request.operationId, format: job.request.format ?? "docx" });
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptImport(result);
    }
    private async commitImport(acceptLosses: boolean) {
        const job = this.cache.importJob, receipt = job?.receipt;
        if (!job || this.importVerified?.operationId !== job.request.operationId || this.importVerified.epoch !== this.epoch || receipt?.status !== "prepared" || !receipt.planId || receipt.commitAllowed === false)
            return;
        if (receipt.preview?.losses?.length && !acceptLosses && !job.commit)
            throw new UiError(uiMessage("office.acceptImportLossesRequired"));
        if (job.planId && job.planId !== receipt.planId || job.commit && (job.commit.resource !== job.request.resource || job.commit.operationId !== job.request.operationId || job.commit.planId !== receipt.planId || typeof job.commit.acceptLosses !== "boolean"))
            throw Object.assign(new UiError(uiMessage("hostedOfficeState.originalImportPlanChanged")), { code: "resultUnknown" });
        const epoch = this.epoch, identity = this.owner?.identity();
        job.commit ??= { resource: job.request.resource, operationId: job.request.operationId, planId: receipt.planId, acceptLosses };
        job.phase = "commit";
        this.persist();
        const result = await this.request<OfficeImportReceipt>("office/import/commit", job.commit);
        if (await this.authority(identity) && epoch === this.epoch)
            this.acceptImport(result);
    }
    private render() {
        this.editor.replaceChildren();
        if (this.unreadable !== undefined) {
            this.editor.append(ui("p", "hint", uiMessage("office.recoveryUnreadable")), ui("pre", "", this.unreadable), action(uiMessage("office.discardUnreadableRecovery"), () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.cache = initial(); this.render(); }));
            return;
        }
        this.renderOutput();
        this.renderImport();
    }
    private renderOutput() {
        const draft = this.cache.draft, job = this.cache.output;
        const form = ui("fieldset");
        form.disabled = this.busy || !!job;
        form.append(ui("legend", "", uiMessage("office.outputSectionTitle")));
        const formats: [
            string,
            UiText
        ][] = [["docx", uiMessage("office.formatDocx")], ["doc", uiMessage("office.formatDoc")], ["wps", uiMessage("office.formatWps")], ["xlsx", uiMessage("office.formatXlsx")], ["pdf", uiMessage("office.formatPdfDraft")], ["bibtex", uiMessage("office.formatBibtex")], ["csl-json", uiMessage("office.formatCslJson")], ["ris", uiMessage("office.formatRis")]];
        if (this.native?.enabled && this.native.read && this.native.export)
            formats.push(["native", uiMessage("office.formatSavedWholeLibrary")]);
        else if (draft.format === "native")
            formats.push(["native", uiMessage("office.originalWholeLibrarySelectionUnavailable")]);
        const format = select(uiMessage("office.outputFormatAria"), formats, draft.format);
        format.onchange = () => { draft.format = format.value as OfficeDraft["format"]; delete draft.template; delete draft.inspection; draft.bindings = {}; this.templateGate.invalidate(); this.keep(); this.render(); };
        const contentLanguage = contentLanguageControl("output", draft.outputLocale, value => { draft.outputLocale = value; this.keep(); });
        form.append(format, contentLanguage);
        if (isLegacy(draft.format))
            form.append(this.renderLegacy(draft.format, draft.legacyBackend, false, value => { draft.legacyBackend = value; this.keep(); this.render(); }));
        if (isWord(draft.format)) {
            const options: [
                string,
                UiText
            ][] = this.resources.filter(r => this.allowed(r, draft.format)).map(r => [r.resource, r.title || r.resource]);
            if (!options.some(([id]) => id === draft.resource) && draft.resource)
                options.push([draft.resource, uiMessage("office.originalSelectionOutsideCatalog")]);
            const resource = select(uiMessage("office.wordOutputTargetAria"), options, draft.resource);
            resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); };
            form.append(resource);
            const mode = select(uiMessage("office.wordReviewModeAria"), [["none", uiMessage("office.reviewBodyOnly")], ["comments", uiMessage("office.reviewWithComments")], ["changes", uiMessage("office.reviewWithPendingRevisions")]], draft.reviewMode);
            mode.onchange = () => { draft.reviewMode = mode.value as OfficeDraft["reviewMode"]; this.keep(); };
            form.append(mode, checkbox(uiMessage("office.includeResolvedComments"), draft.includeResolved, checked => { draft.includeResolved = checked; this.keep(); }), ui("p", "hint", uiMessage("office.reviewSavedBodyHelp")));
            if (Object.values(draft.bindings).some(binding => binding.kind === "query")) {
                const query = select(uiMessage("office.templateFrozenQueryAria"), [["", uiMessage("office.chooseDisplayedAuthorizedQuery")], ...this.queries.map(q => [q.queryId, () => `${uiTextValue(q.columns.join("、") || uiMessage("office.authorizedFieldQuery"))}${uiTextValue(q.coverage === "partial" ? uiMessage("office.queryIncompleteDotSuffix") : "")}`] as [
                        string,
                        UiText
                    ])], draft.queryId);
                query.onchange = () => { draft.queryId = query.value; this.keep(); this.render(); };
                form.append(query);
            }
        }
        else if (draft.format === "xlsx") {
            const options: [
                string,
                UiText
            ][] = [["", uiMessage("office.chooseDisplayedFieldQuery")]];
            for (const q of this.queries)
                options.push([q.queryId, () => `${uiTextValue(q.columns.join("、") || uiMessage("office.authorizedFieldQuery"))} · ${uiTextValue(q.coverage === "unknown" ? uiMessage("office.originalResult") : displayMessage("office.queryRowCount", { p0: q.rowCount }))}${uiTextValue(q.coverage === "partial" ? uiMessage("office.queryIncompleteDotSuffix") : "")}`]);
            if (draft.queryId && !options.some(([id]) => id === draft.queryId))
                options.push([draft.queryId, uiMessage("office.originalQueryResultOption")]);
            const query = select(uiMessage("office.excelFrozenQueryAria"), options, draft.queryId);
            query.onchange = () => { draft.queryId = query.value; this.keep(); this.render(); };
            form.append(query);
        }
        else if (draft.format === "native") {
            form.append(ui("p", "hint", uiMessage("office.wholeLibrarySavedScopeHelp")));
            if (!(this.native?.enabled && this.native.read && this.native.export))
                form.append(ui("p", "hint", uiMessage("office.wholeLibraryExportUnauthorized")));
        }
        else {
            const bib = draft.format !== "pdf";
            if (bib) {
                const scope = select(uiMessage("office.bibliographyScopeAria"), [["citationWorks", uiMessage("office.bibliographyCitedWorks")], ["selectedWorks", uiMessage("office.bibliographyChooseAuthorizedWorks")]], draft.bibliographyScope ?? "citationWorks");
                scope.onchange = () => { draft.bibliographyScope = scope.value as "citationWorks" | "selectedWorks"; this.keep(); this.render(); };
                form.append(scope);
            }
            if (bib && draft.bibliographyScope === "selectedWorks") {
                const known = new Set(this.works.map(w => w.resource));
                for (const work of this.works) {
                    const choice = checkbox(`${work.title} · ${work.resource}`, draft.works?.includes(work.resource) ?? false, checked => {
                        const selected = new Set(draft.works ?? []);
                        if (checked)
                            selected.add(work.resource);
                        else
                            selected.delete(work.resource);
                        draft.works = [...selected];
                        this.keep();
                        this.render();
                    });
                    const control = choice.querySelector("input")!;
                    control.disabled = work.status !== "ready" || !work.export;
                    form.append(choice);
                }
                for (const id of (draft.works ?? []).filter(id => !known.has(id)))
                    form.append(ui("p", "hint", displayMessage("office.originalWorkUnavailable", { p0: uiDisplayParameter(id) })), action(displayMessage("office.removeOriginalWork", { p0: uiDisplayParameter(id) }), () => { draft.works = draft.works?.filter(value => value !== id); this.keep(); this.render(); }));
                form.append(ui("p", "hint", uiMessage("office.bibliographyPublicationOnlyHelp")));
            }
            else {
                const options: [
                    string,
                    UiText
                ][] = this.resources.filter(r => this.allowed(r, draft.format)).map(r => [r.resource, r.title]);
                if (draft.resource && !options.some(([id]) => id === draft.resource))
                    options.push([draft.resource, uiMessage("office.originalTargetUnavailable")]);
                const resource = select(uiMessage("office.bodyTargetAria"), options, draft.resource);
                resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); };
                form.append(resource);
                if (bib) {
                    const source = select(uiMessage("office.bibliographyBodyBasisAria"), [["draft", uiMessage("office.bodyBasisCurrentDraft")], ["saved", uiMessage("office.bodyBasisSaved")]], draft.bibliographySource ?? "draft");
                    source.onchange = () => { draft.bibliographySource = source.value as "draft" | "saved"; this.keep(); };
                    form.append(source);
                }
            }
        }
        if (["docx", "doc", "wps", "pdf"].includes(draft.format))
            form.append(checkbox(uiMessage("office.includeFrozenEmbeddedQueries"), draft.includeEmbedded ?? false, checked => { draft.includeEmbedded = checked; this.keep(); }), ui("p", "hint", uiMessage("office.embeddedQueryFreezeHelp")));
        if (["docx", "doc", "wps", "xlsx"].includes(draft.format)) {
            const style = select(uiMessage("office.stylePolicyAria"), [["template", uiMessage("office.styleTemplate")], ["source", uiMessage("office.styleSource")]], draft.stylePolicy);
            style.onchange = () => { draft.stylePolicy = style.value as OfficeDraft["stylePolicy"]; this.keep(); };
            form.append(style);
            const template = ui("input");
            template.type = "file";
            template.accept = draft.format === "xlsx" ? ".xlsx" : ".docx";
            setUiAttr(template, "aria-label", uiMessage("office.templateUploadAria"));
            template.onchange = () => {
                const file = template.files?.[0];
                if (!file)
                    return;
                const epoch = this.epoch, generation = this.templateGate.begin(), format = templateInspectionFormat(draft.format);
                void this.run(async () => {
                    const bytes = await fileBytes(file, 16 * 1024 * 1024);
                    if (epoch !== this.epoch || !this.templateGate.accepts(generation))
                        return;
                    const incoming = await this.request<unknown>("office/template", { format, bytesBase64: bytes.bytesBase64 });
                    if (epoch !== this.epoch || !this.templateGate.accepts(generation))
                        return;
                    draft.template = bytes;
                    draft.inspection = decodeTemplateInspection(incoming, format);
                    draft.bindings = {};
                    if (this.keep())
                        setUiText(this.message, uiMessage("office.templateInspected"));
                });
            };
            form.append(template);
            if (draft.template)
                form.append(ui("p", "", displayMessage("office.templateName", { p0: uiDisplayParameter(draft.template.name) })), action(uiMessage("office.removeTemplate"), () => { delete draft.template; delete draft.inspection; draft.bindings = {}; this.templateGate.invalidate(); this.keep(); this.render(); }));
            for (const slot of draft.inspection?.slots ?? []) {
                const options: [
                    string,
                    UiText
                ][] = [["", uiMessage("office.templateKeepOriginal")], ...(isWord(draft.format) ? [["body", uiMessage("office.previewBody")], ["title", uiMessage("office.previewTitle")]] as [
                        string,
                        UiText
                    ][] : []), ["text", uiMessage("office.templateFixedText")], ["query", uiMessage("office.templateFrozenQuery")], ["field", uiMessage("office.templateAuthorizedField")]];
                const binding = select(displayMessage("office.templateSlotAria", { p0: uiDisplayParameter(slot.name) }), options, draft.bindings[slot.name]?.kind ?? "");
                binding.disabled = slot.supported === false;
                binding.onchange = () => {
                    if (!binding.value)
                        delete draft.bindings[slot.name];
                    else
                        draft.bindings[slot.name] = { kind: binding.value as OfficeBinding["kind"], ...(binding.value === "text" ? { text: draft.bindings[slot.name]?.text ?? "" } : binding.value === "field" ? { field: draft.bindings[slot.name]?.field ?? "" } : {}) };
                    this.keep();
                    this.render();
                };
                form.append(ui("p", "", slot.label ?? slot.name), binding);
                if (slot.description) form.append(ui("p", "hint", slot.description));
                if (slot.supported === false)
                    form.append(ui("p", "hint", slot.reason ? exportErrorUiText(slot.reason) : uiMessage("office.templateSlotUnsupported")));
                if (draft.bindings[slot.name]?.kind === "text") {
                    const text = ui("textarea");
                    text.value = draft.bindings[slot.name].text ?? "";
                    setUiAttr(text, "aria-label", displayMessage("office.templateTextAria", { p0: uiDisplayParameter(slot.name) }));
                    text.oninput = () => { draft.bindings[slot.name].text = text.value; this.keep(); };
                    form.append(text);
                }
                if (draft.bindings[slot.name]?.kind === "field") {
                    const field = ui("input", "data-input");
                    field.value = draft.bindings[slot.name].field ?? "";
                    setUiAttr(field, "aria-label", displayMessage("office.templateFieldAria", { p0: uiDisplayParameter(slot.name) }));
                    field.oninput = () => { draft.bindings[slot.name].field = field.value; this.keep(); };
                    form.append(field, ui("p", "hint", uiMessage("office.templateFieldHelp")));
                }
            }
            if (Object.values(draft.bindings).some(b => b.kind === "field") && draft.format === "xlsx") {
                const resource = select(uiMessage("office.fieldOutputTargetAria"), this.resources.filter(r => this.allowed(r, "xlsx")).map(r => [r.resource, r.title]), draft.resource);
                resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); };
                form.append(resource);
            }
        }
        const prepare = action(uiMessage("office.prepareOutput"), () => this.run(() => this.prepareOutput()));
        prepare.disabled = draft.format === "native" ? !(this.native?.enabled && this.native.read && this.native.export) : draft.format === "xlsx" ? !draft.queryId && !Object.values(draft.bindings).some(b => b.kind === "field") : ["bibtex", "csl-json", "ris"].includes(draft.format) && draft.bibliographyScope === "selectedWorks" ? !(draft.works?.length) : !this.resources.some(r => r.resource === draft.resource && this.allowed(r, draft.format));
        prepare.disabled ||= !this.legacyAvailable(draft.format, draft.legacyBackend, false);
        form.append(prepare);
        this.editor.append(form);
        if (!job)
            return;
        const card = ui("section", "output-status"), receipt = officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch);
        card.append(ui("h3", "", uiMessage("office.originalOutputTitle")), ui("p", "", job.request.requestId), ui("p", "", receipt ? officeOutputUiText(receipt) ?? uiMessage("office.cachedOutputHelp") : uiMessage("office.cachedOutputHelp")));
        if (receipt?.filename)
            card.append(ui("p", "", displayMessage("office.outputFileSummary", { p0: uiDisplayParameter(receipt.filename), p1: uiDisplayParameter(receipt.length) })));
        if (receipt?.losses)
            card.append(losses(receipt.losses));
        if (receipt?.preview)
            card.append(preview(receipt.preview, job.query?.columns));
        const accepted = checkbox(uiMessage("office.acceptOutputLosses"), job.confirm?.acceptLosses ?? job.acceptLosses ?? false, value => { job.acceptLosses = value; this.keep(); });
        const check = accepted.querySelector("input")!;
        check.disabled = this.busy || !!job.phase || !!job.confirm || receipt?.status !== "prepared";
        const confirm = action(uiMessage("office.confirmOutput"), () => this.run(() => this.confirmOutput(check.checked)));
        confirm.disabled = this.busy || receipt?.status !== "prepared" || !!job.phase;
        const recover = action(uiMessage("office.inspectOriginalOutput"), () => this.run(() => this.recoverOutput()));
        recover.disabled = this.busy;
        const download = action(uiMessage("office.downloadConfirmedOutput"), () => this.run(() => this.downloadOutput()));
        download.disabled = this.busy || receipt?.status !== "confirmed" || !!job.phase || !this.canDownload(job.request);
        if (!this.canDownload(job.request))
            card.append(ui("p", "hint", uiMessage("office.outputDownloadUnauthorized")));
        card.append(accepted, confirm, recover, download);
        if (isLegacy(job.request.format) && (job.phase === "prepare" || receipt?.status === "pending")) {
            const cancel = action(uiMessage("office.cancelExternalOutput"), () => this.cancelLegacy(false));
            cancel.disabled = this.cancelling;
            card.append(cancel);
        }
        const fresh = action(uiMessage("office.prepareNewOutput"), () => { delete this.cache.output; this.outputVerified = undefined; this.keep(); this.render(); });
        fresh.disabled = this.busy || !receipt || !!job.phase || !outputDone(receipt?.status) && !job.received;
        card.append(fresh);
        this.editor.append(card);
    }
    private renderImport() {
        const draft = this.cache.importDraft, job = this.cache.importJob;
        const form = ui("fieldset");
        form.disabled = this.busy || !!job;
        form.append(ui("legend", "", uiMessage("office.wordImportTitle")));
        const formatValue = draft.format ?? "docx";
        const format = select(uiMessage("office.wordImportFormatAria"), [["docx", uiMessage("office.formatDocxImport")], ["doc", uiMessage("office.formatDoc")], ["wps", uiMessage("office.formatWps")]], formatValue);
        format.onchange = () => {
            draft.format = format.value as OfficeImportFormat;
            delete draft.analysis;
            delete draft.captionChoices;
            delete draft.legacyBackend;
            this.importGate.invalidate();
            this.keep();
            this.render();
            if (draft.file && draft.format === "docx")
                void this.run(() => this.inspectImport());
        };
        form.append(format);
        if (isLegacy(formatValue))
            form.append(this.renderLegacy(formatValue, draft.legacyBackend, true, value => { draft.legacyBackend = value; this.keep(); this.render(); }));
        const options: [
            string,
            UiText
        ][] = this.targets.map(t => [t.resource, () => (t.title || t.resource) + uiTextValue(t.available === false ? uiMessage("office.existingDocumentSuffix") : "")]);
        if (draft.resource && !options.some(([id]) => id === draft.resource))
            options.push([draft.resource, uiMessage("office.originalImportTargetOutsideCatalog")]);
        const target = select(uiMessage("office.wordImportTargetAria"), options, draft.resource);
        target.onchange = () => {
            draft.resource = target.value;
            delete draft.analysis;
            delete draft.captionChoices;
            this.importGate.invalidate();
            this.keep();
            this.render();
            if (draft.file && formatValue === "docx")
                void this.run(() => this.inspectImport());
        };
        form.append(target);
        if (!this.importAllowed(draft.resource, formatValue))
            form.append(ui("p", "data-warning", uiMessage("office.wordTargetUnauthorized")));
        const file = ui("input");
        file.type = "file";
        file.accept = `.${formatValue}`;
        setUiAttr(file, "aria-label", uiMessage("office.wordOriginalAria"));
        file.onchange = () => {
            const selected = file.files?.[0];
            if (!selected)
                return;
            const epoch = this.epoch, generation = this.importGate.begin();
            this.original = selected;
            delete draft.file;
            delete draft.analysis;
            delete draft.captionChoices;
            this.keep();
            void this.run(async () => {
                const bytes = await fileBytes(selected, 16 * 1024 * 1024);
                if (epoch !== this.epoch || !this.importGate.accepts(generation))
                    return;
                draft.file = bytes;
                if (this.keep())
                    setUiText(this.message, uiMessage("office.wordOriginalRead"));
                if ((draft.format ?? "docx") === "docx")
                    await this.inspectImport();
            });
        };
        form.append(file);
        if (draft.file || this.original)
            form.append(ui("p", "", displayMessage("office.retainedOriginalName", { p0: uiDisplayParameter(draft.file?.name ?? this.original?.name) })));
        let invalidCaption = false;
        if (formatValue === "docx" && draft.file) {
            const inspect = action(uiMessage("office.inspectWordCaptions"), () => this.run(() => this.inspectImport()));
            inspect.disabled = !this.importAllowed(draft.resource, "docx");
            form.append(inspect, this.renderCaptionPairings());
            try {
                selectedCaptionOptions(draft.analysis ?? null, draft.captionChoices ?? []);
            }
            catch (error) {
                invalidCaption = true;
                form.append(ui("p", "data-warning", error instanceof Error ? errorUiText(error) : uiMessage("office.captionChoicesIncomplete")));
            }
        }
        const prepare = action(uiMessage("office.prepareWordImport"), () => this.run(() => this.prepareImport()));
        prepare.disabled = !draft.file || !this.importAllowed(draft.resource, formatValue) || !this.legacyAvailable(formatValue, draft.legacyBackend, true) || invalidCaption;
        form.append(prepare);
        this.editor.append(form);
        if (!job)
            return;
        const card = ui("section", "output-status");
        card.append(ui("h3", "", uiMessage("office.originalWordImportTitle")), ui("p", "", displayMessage("office.wordOriginalName", { p0: uiDisplayParameter(job.name) })));
        const verified = this.importVerified?.operationId === job.request.operationId && this.importVerified.epoch === this.epoch;
        const receipt = verified ? job.receipt : undefined;
        const summary = receipt?.preview?.summary;
        if (summary)
            card.append(ui("p", "", summary));
        if (!verified)
            card.append(ui("p", "hint", uiMessage("office.cachedImportHelp")));
        if (receipt?.preview?.losses)
            card.append(losses(receipt.preview.losses));
        if (receipt?.preview?.legacyConversion !== undefined)
            card.append(conversionDetails(receipt.preview.legacyConversion));
        const accepted = checkbox(uiMessage("office.acceptWordImportLosses"), job.commit?.acceptLosses ?? job.acceptLosses ?? false, value => { job.acceptLosses = value; this.keep(); });
        const check = accepted.querySelector("input")!;
        check.disabled = this.busy || !!job.phase || !!job.commit || receipt?.status !== "prepared" || receipt.commitAllowed === false;
        const commit = action(uiMessage("office.commitWordImport"), () => this.run(() => this.commitImport(check.checked)));
        commit.disabled = this.busy || !!job.phase || receipt?.status !== "prepared" || !receipt.planId || receipt.commitAllowed === false;
        const recover = action(uiMessage("office.inspectOriginalWordImport"), () => this.run(() => this.recoverImport()));
        recover.disabled = this.busy;
        const end = action(uiMessage("office.finishOriginalWordImport"), () => this.run(() => this.endImport()));
        end.disabled = this.busy || terminalImport(receipt?.status);
        card.append(accepted, commit, recover, end);
        if (isLegacy(job.request.format ?? "docx") && (job.phase === "prepare" || receipt?.status === "pending")) {
            const cancel = action(uiMessage("office.cancelExternalImport"), () => this.cancelLegacy(true));
            cancel.disabled = this.cancelling;
            card.append(cancel);
        }
        const fresh = action(uiMessage("office.prepareAnotherWordImport"), () => { this.cache.importDraft.file = { name: job.name, bytesBase64: job.request.bytesBase64 }; this.cache.importDraft.format = job.request.format ?? "docx"; this.cache.importDraft.legacyBackend = job.request.legacyBackend; delete this.cache.importDraft.analysis; delete this.cache.importDraft.captionChoices; delete this.cache.importJob; this.importVerified = undefined; this.keep(); this.render(); });
        fresh.disabled = this.busy || !terminalImport(receipt?.status);
        card.append(fresh);
        this.editor.append(card);
    }
    private renderCaptionPairings(): HTMLElement {
        const section = ui("section", "output-caption-options"), draft = this.cache.importDraft, analysis = draft.analysis;
        section.append(ui("p", "hint", uiMessage("office.captionMappingHelp")));
        if (!analysis || !validDocxAnalysis(analysis))
            return section;
        section.append(ui("p", "hint", displayMessage("office.captionCandidateCount", { p0: analysis.captions.length })));
        draft.captionChoices ??= [];
        analysis.captions.forEach((caption, index) => {
            let choice = draft.captionChoices![index];
            if (!choice || choice.captionBlock !== caption.blockIndex) {
                choice = { enabled: false, captionBlock: caption.blockIndex, objectBlock: "", kind: "", captionText: caption.suggestedText, position: "" };
                draft.captionChoices![index] = choice;
            }
            const row = ui("fieldset", "output-caption-pairing");
            row.append(ui("legend", "", displayMessage("office.captionBlockSummary", { p0: caption.blockIndex + 1, p1: uiDisplayParameter(caption.sequence) })), ui("p", "output-preview-value", caption.text));
            const object = select(displayMessage("office.captionObjectAria", { p0: caption.blockIndex + 1 }), [["", uiMessage("office.chooseCaptionObject")], ...analysis.objects.filter(item => item.blockIndex !== caption.blockIndex).map(item => [String(item.blockIndex), displayMessage("office.captionObjectChoice", { p0: item.blockIndex + 1, p1: uiDisplayParameter(kindLabel(item.kind)) })] as [
                    string,
                    UiText
                ])], choice.objectBlock);
            const kind = select(displayMessage("office.captionKindAria", { p0: caption.blockIndex + 1 }), [["", uiMessage("office.chooseCaptionKind")], ...captionKinds], choice.kind);
            const position = select(displayMessage("office.captionPositionAria", { p0: caption.blockIndex + 1 }), [["", uiMessage("office.chooseCaptionPosition")], ["top", uiMessage("office.captionAboveObject")], ["bottom", uiMessage("office.captionBelowObject")]], choice.position);
            const text = ui("textarea", "data-input");
            text.value = choice.captionText;
            text.rows = 2;
            setUiAttr(text, "aria-label", displayMessage("office.captionTextAria", { p0: caption.blockIndex + 1 }));
            const update = (rerender = true): void => {
                Object.assign(choice, { objectBlock: object.value, kind: kind.value, captionText: text.value, position: position.value });
                this.keep();
                if (rerender)
                    this.render();
            };
            object.onchange = () => update();
            kind.onchange = () => update();
            position.onchange = () => update();
            text.oninput = () => update(false);
            text.onchange = () => update();
            row.append(object, kind, position, text, checkbox(displayMessage("office.confirmCaptionMapping", { p0: caption.blockIndex + 1 }), choice.enabled, value => { choice.enabled = value; update(); }));
            section.append(row);
        });
        return section;
    }
}
