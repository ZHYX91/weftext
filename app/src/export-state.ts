import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { UiLocale } from "./ui-locale";
import { parseUiMessage, type UiMessage } from "./ui-locale";
import { decodeExchangePlan, decodeExchangeReceipt, decodeLocalOutput, decodeOutputLoss, invalidOfficeWire, outputFailureUiText, type ExchangePlan, type ExchangeReceipt, type LegacyCapabilities, type NativeLoss, type OutputJobFailure, type OutputLoss, type PreparedOutput, type TemplateInspection } from "./office-wire";
export type { ExchangePlan, ExchangeReceipt, LegacyCapabilities, OutputJobFailure, OutputLoss, PreparedOutput, TemplateAnchor, TemplateInspection } from "./office-wire";
import { DEFAULT_CONTENT_LOCALE } from "./content-language";
/** Output owns a frozen draft and one artifact ID. It never writes author data. */
import type { DocumentBasis } from "./data-panel";
export type ExportFormat = "docx" | "doc" | "wps" | "pdf" | "xlsx" | "native" | "bibtex" | "csl-json" | "ris" | "source";
export interface DocxImportOptions {
    contentHash: string;
    captionPairings: {
        captionBlock: number;
        objectBlock: number;
        kind: string;
        captionText: string;
        position: string;
    }[];
}
export interface DocxImportAnalysis {
    contentHash: string;
    captions: {
        blockIndex: number;
        sequence: string;
        text: string;
        suggestedText: string;
    }[];
    objects: {
        blockIndex: number;
        kind: string;
    }[];
}
export interface EmbeddedQuerySelection {
    occurrenceId: string;
    queryId: string;
}
export type ExportStylePolicy = "template" | "source";
export interface FrozenQuery {
    queryId: string;
    label: string | UiMessage;
    rowCount: number;
    coverage: "complete" | "partial";
    freshness: "current" | "stale";
    limited?: boolean;
}
export interface OutputFile {
    name: string;
    bytesBase64: string;
}
export interface OutputBinding {
    kind: "body" | "title" | "field" | "query" | "text";
    field?: string;
    text?: string;
}
export interface ExportDraft {
    outputLocale: UiLocale;
    format: ExportFormat;
    stylePolicy: ExportStylePolicy;
    template?: OutputFile;
    bindings: Record<string, OutputBinding>;
    query?: FrozenQuery;
    reviewMode?: "none" | "comments" | "changes";
    includeResolved?: boolean;
    legacyBackend?: string;
}
export interface ExportRequest {
    reviewMode?: "none" | "comments" | "changes";
    includeResolved?: boolean;
    outputLocale: UiLocale;
    requestId: string;
    sessionId: string;
    source: string;
    serial: number;
    baseId: string;
    format: ExportFormat;
    stylePolicy: ExportStylePolicy;
    template?: OutputFile;
    bindings?: Record<string, OutputBinding>;
    queryId?: string;
    embeddedQueries?: EmbeddedQuerySelection[];
    legacyBackend?: string;
}
export interface OutputTransport {
    id(): string;
    prepare(request: ExportRequest): Promise<unknown>;
    confirm(request: {
        id: string;
        acceptLosses: boolean;
    }): Promise<unknown>;
    download(request: {
        id: string;
    }): Promise<Blob>;
    digest?(blob: Blob): Promise<string>;
    cancel?(request: {
        requestId: string;
    }): Promise<unknown>;
}
export function uncertainOutput(error: unknown): boolean {
    const code = error && typeof error === "object" && "code" in error ? String(error.code) : "";
    return !code || ["unknown", "resultUnknown", "pending", "in-progress", "pending_reconciliation", "recoveryPending"].includes(code);
}
const malformed = (message: UiText): Error => Object.assign(new UiError(message), { code: "resultUnknown" });
const clone = <T>(value: T): T => structuredClone(value);
export class OutputController {
    readonly basis: Readonly<DocumentBasis>;
    draft: ExportDraft;
    prepared: PreparedOutput | null = null;
    confirmed = false;
    received = false;
    busy = false;
    uncertain: "prepare" | "confirm" | "download" | null = null;
    private pending: ExportRequest | null = null;
    private confirmRequest: {
        id: string;
        acceptLosses: boolean;
    } | null = null;
    readonly embeddedQueries: readonly EmbeddedQuerySelection[];
    constructor(readonly host: OutputTransport, basis: DocumentBasis, query?: FrozenQuery, embeddedQueries: readonly EmbeddedQuerySelection[] = []) {
        this.basis = Object.freeze(clone(basis));
        this.embeddedQueries = clone(embeddedQueries);
        const frozenQuery = query && clone(query);
        if (frozenQuery && typeof frozenQuery.label !== "string") frozenQuery.label = parseUiMessage(frozenQuery.label) ?? invalidOfficeWire();
        this.draft = { outputLocale: DEFAULT_CONTENT_LOCALE, format: query ? "xlsx" : "docx", stylePolicy: "template", bindings: {}, ...(frozenQuery ? { query: frozenQuery } : {}) };
    }
    get locked(): boolean { return this.busy || this.uncertain !== null; }
    get requestId(): string | null { return this.pending?.requestId ?? null; }
    async cancel(): Promise<void> {
        if (!this.pending || !this.host.cancel)
            throw new UiError(uiMessage("outputState.noCancellableConversion"));
        await this.host.cancel({ requestId: this.pending.requestId });
    }
    setDraft(value: Partial<ExportDraft>): void {
        if (this.locked)
            throw new UiError(uiMessage("outputState.originalOutputPending"));
        const next = { ...this.draft, ...clone(value) };
        if (next.query && typeof next.query.label !== "string") next.query.label = parseUiMessage(next.query.label) ?? invalidOfficeWire();
        if (next.format !== this.draft.format) {
            next.template = undefined;
            next.bindings = {};
        }
        this.draft = next;
        this.pending = null;
        this.confirmRequest = null;
        this.prepared = null;
        this.confirmed = false;
        this.received = false;
    }
    async prepare(): Promise<PreparedOutput> {
        if (this.busy || this.uncertain === "confirm" || this.uncertain === "download")
            throw new UiError(uiMessage("outputState.finishOriginalBeforeGenerating"));
        if (this.prepared && !this.pending)
            return this.prepared;
        if (!this.pending) {
            this.pending = {
                requestId: this.host.id(), sessionId: this.basis.sessionId, source: this.basis.source, serial: this.basis.serial, baseId: this.basis.baseId,
                format: this.draft.format, stylePolicy: this.draft.stylePolicy, outputLocale: this.draft.outputLocale,
                ...(["docx", "doc", "wps"].includes(this.draft.format) ? { reviewMode: this.draft.reviewMode ?? "none", includeResolved: this.draft.includeResolved ?? false } : {}),
                ...(["doc", "wps"].includes(this.draft.format) ? { legacyBackend: this.draft.legacyBackend ?? "libreoffice" } : {}),
                ...(this.draft.template ? { template: clone(this.draft.template), bindings: clone(this.draft.bindings) } : {}),
                ...(this.draft.query ? { queryId: this.draft.query.queryId } : {}),
                ...(["docx", "doc", "wps", "pdf"].includes(this.draft.format) ? { embeddedQueries: clone([...this.embeddedQueries]) } : {}),
            };
        }
        this.busy = true;
        try {
            const result = decodeLocalOutput(await this.host.prepare(clone(this.pending)));
            if (result.requestId !== this.pending.requestId)
                throw malformed(uiMessage("outputState.generationReceiptMismatch"));
            if (result.status === "failed")
                throw Object.assign(new UiError(outputFailureUiText(result)), { code: uncertainOutput({ code: result.code }) ? result.code : "outputFailed" });
            if (result.status === "pending")
                throw Object.assign(new UiError(uiMessage("outputState.outputProcessingKeepRequest")), { code: "pending" });
            if (!("filename" in result))
                throw malformed(uiMessage("outputState.fileStateUnrecognized"));
            if (!result.id || !["prepared", "confirmed"].includes(result.status) || !result.filename || !result.contentType || !Number.isSafeInteger(result.length) || result.length < 0 || !/^[a-f0-9]{64}$/i.test(result.digest) || !Array.isArray(result.losses))
                throw malformed(uiMessage("outputState.exactFileInformationMissing"));
            const saved = result;
            this.prepared = saved;
            this.pending = null;
            this.confirmed = result.status === "confirmed";
            this.received = false;
            this.uncertain = null;
            return saved;
        }
        catch (error) {
            if (uncertainOutput(error))
                this.uncertain = "prepare";
            else {
                this.pending = null;
                this.uncertain = null;
            }
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
    async confirm(acceptLosses: boolean): Promise<void> {
        if (!this.prepared || this.busy || this.uncertain === "prepare" || this.uncertain === "download")
            throw new UiError(uiMessage("outputState.viewExactPreviewFirst"));
        if (this.prepared.losses.length && !acceptLosses && !this.confirmRequest)
            throw new UiError(uiMessage("outputState.acceptOutputLossesFirst"));
        this.confirmRequest ??= { id: this.prepared.id, acceptLosses };
        this.busy = true;
        try {
            const result = decodeLocalOutput(await this.host.confirm({ ...this.confirmRequest }));
            if (result.status !== "confirmed" || result.id !== this.prepared.id || result.requestId !== this.prepared.requestId || result.digest !== this.prepared.digest || result.length !== this.prepared.length || result.filename !== this.prepared.filename || result.contentType !== this.prepared.contentType)
                throw malformed(uiMessage("outputState.confirmationPreviewMismatch"));
            this.confirmed = true;
            this.uncertain = null;
            this.confirmRequest = null;
        }
        catch (error) {
            this.uncertain = uncertainOutput(error) ? "confirm" : null;
            if (!this.uncertain)
                this.confirmRequest = null;
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
    async download(): Promise<Blob> {
        if (!this.prepared || !this.confirmed || this.busy || this.uncertain === "prepare" || this.uncertain === "confirm")
            throw new UiError(uiMessage("outputState.confirmSameFileFirst"));
        this.busy = true;
        try {
            const blob = await this.host.download({ id: this.prepared.id });
            if (blob.size !== this.prepared.length || this.host.digest && await this.host.digest(blob) !== this.prepared.digest)
                throw malformed(uiMessage("outputState.downloadBytesMismatch"));
            this.received = true;
            this.uncertain = null;
            return blob;
        }
        catch (error) {
            this.uncertain = uncertainOutput(error) ? "download" : null;
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
}
/** Independent template reads cannot replace a later file or format selection. */
export class OutputReadGate {
    private generation = 0;
    begin(): number { return ++this.generation; }
    accepts(generation: number): boolean { return generation === this.generation; }
    invalidate(): void { this.generation++; }
}
export type ExchangeFormat = "native" | "docx" | "doc" | "wps" | "bibtex" | "csl-json" | "ris";
export type ExchangeLoss = NativeLoss;
export interface BibliographyInspection {
    records: {
        conflicts: unknown[];
        [key: string]: unknown;
    }[];
    losses: OutputLoss[];
}
export function bibliographyConflictCount(result: BibliographyInspection): number {
    if (!Array.isArray(result.records) || !Array.isArray(result.losses) || result.records.some(record => !record || !Array.isArray(record.conflicts)))
        throw new UiError(uiMessage("outputState.bibliographyConflictStatusIncomplete"));
    return result.records.reduce((count, record) => count + record.conflicts.length, 0);
}
export function decodeBibliographyInspection(value: unknown): BibliographyInspection {
    if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).sort().join(",") !== "losses,records") invalidOfficeWire();
    const result = value as BibliographyInspection;
    bibliographyConflictCount(result);
    return { records: structuredClone(result.records), losses: result.losses.map(decodeOutputLoss) };
}
export interface ExchangeRequest {
    operationId: string;
    sessionId: string;
    baseId: string;
    root: string;
    parent: string;
    name: string;
    bytesBase64: string;
    format: ExchangeFormat;
    legacyBackend?: string;
    docxOptions?: DocxImportOptions;
}
export interface ExchangeTransport {
    id(): string;
    prepare(request: ExchangeRequest): Promise<unknown>;
    commit(request: {
        operationId: string;
        planId: string;
        acceptLosses: boolean;
    }): Promise<unknown>;
    cancel?(request: {
        requestId: string;
    }): Promise<unknown>;
}
/** An uncertain import keeps its uploaded original and reuses one Core operation. */
export class ExchangeController {
    input: Omit<ExchangeRequest, "operationId"> | null = null;
    plan: ExchangePlan | null = null;
    receipt: ExchangeReceipt | null = null;
    busy = false;
    uncertain: "prepare" | "commit" | null = null;
    private pending: ExchangeRequest | null = null;
    private commitRequest: {
        operationId: string;
        planId: string;
        acceptLosses: boolean;
    } | null = null;
    constructor(readonly host: ExchangeTransport) { }
    get locked(): boolean { return this.busy || this.uncertain !== null; }
    get requestId(): string | null { return this.pending?.operationId ?? null; }
    async cancel(): Promise<void> {
        if (!this.pending || !this.host.cancel)
            throw new UiError(uiMessage("outputState.noCancellableConversion"));
        await this.host.cancel({ requestId: this.pending.operationId });
    }
    setInput(input: Omit<ExchangeRequest, "operationId">): void {
        if (this.locked)
            throw new UiError(uiMessage("outputState.originalImportUnconfirmed"));
        this.input = clone(input);
        this.plan = null;
        this.pending = null;
        this.commitRequest = null;
        this.receipt = null;
    }
    async prepare(): Promise<ExchangePlan> {
        if (!this.input || this.busy || this.uncertain === "commit" || this.receipt)
            throw new UiError(uiMessage("outputState.selectOriginalLocationOrCheckResult"));
        if (this.plan && !this.pending)
            return this.plan;
        this.pending ??= { ...clone(this.input), operationId: this.host.id() };
        this.busy = true;
        try {
            const plan = decodeExchangePlan(await this.host.prepare(clone(this.pending)));
            if (plan.status !== "prepared" || plan.operationId !== this.pending.operationId)
                throw malformed(uiMessage("outputState.importPreviewMismatch"));
            this.plan = plan;
            this.pending = null;
            this.uncertain = null;
            return this.plan;
        }
        catch (error) {
            if (uncertainOutput(error))
                this.uncertain = "prepare";
            else {
                this.pending = null;
                this.uncertain = null;
            }
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
    async commit(acceptLosses: boolean): Promise<ExchangeReceipt> {
        if (!this.plan || this.busy || this.uncertain === "prepare")
            throw new UiError(uiMessage("outputState.viewImportPlanFirst"));
        if (this.receipt)
            return this.receipt;
        if (this.plan.losses.length && !acceptLosses && !this.commitRequest)
            throw new UiError(uiMessage("outputState.acceptImportLossesFirst"));
        this.commitRequest ??= { operationId: this.plan.operationId, planId: this.plan.planId, acceptLosses };
        this.busy = true;
        try {
            const result = decodeExchangeReceipt(await this.host.commit({ ...this.commitRequest }));
            if (result.status !== "committed" || result.operationId !== this.plan.operationId || result.planId !== this.plan.planId)
                throw malformed(uiMessage("outputState.actualImportReceiptMissing"));
            this.receipt = result;
            this.uncertain = null;
            this.commitRequest = null;
            return this.receipt;
        }
        catch (error) {
            this.uncertain = uncertainOutput(error) ? "commit" : null;
            if (!this.uncertain)
                this.commitRequest = null;
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
}
