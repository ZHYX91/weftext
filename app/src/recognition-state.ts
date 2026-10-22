import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { RecognitionEvidence, RecognitionRect, RecognitionScheme } from "./recognition-review";
import { parseUiMessage, type UiMessage } from "./ui-locale";
import { exactKeys } from "./ui-diagnostics";
import { decodePublicImportLoss, type PublicImportLoss } from "./hosted-import-wire";
export type RecognitionPurpose = "resource" | "text" | "note";
export type RecognitionTarget = {
    sessionId: string;
    parent?: string;
    name?: string;
} | {
    slot: string;
};
export interface RecognitionOptions {
    purpose: RecognitionPurpose;
    operation?: "inspect" | "recognize";
    languages: string[];
    pages: {
        page: number;
        regions?: {
            id: string;
            rect: RecognitionRect;
            mode: "auto" | "text" | "ocr";
            rotation: 0 | 90 | 180 | 270;
            segmentation: "auto" | "sparse" | "block" | "line";
            ocrScale?: number;
        }[];
    }[];
    dpi: number;
    scheme: string;
    language?: string;
    script?: string;
    region?: string;
    numbering: "keep" | "remove" | "rebuild";
}
export interface RecognitionCapabilities {
    version: 1;
    available: boolean;
    executor: "local" | "selfHosted";
    formats: string[];
    models: {
        id: string;
        languages: string[];
    }[];
    schemes: {
        id: string;
        title: UiMessage;
    }[];
    limits: {
        inputBytes: number;
        pages: number;
        resultBytes: number;
        jobSeconds: number;
        pageSeconds: number;
        concurrentExecutions: number;
        retainedBytes: number;
    };
    implementationDigest: string;
    targets?: {
        id: string;
        title: string;
        available: boolean;
    }[];
}
export interface RecognitionJob {
    version: 1;
    jobId: string;
    status: "pending" | "running" | "complete" | "partial" | "failed" | "cancelled" | "interrupted";
    inputSha256: string;
    optionsDigest: string;
    resultDigest?: string;
    result?: RecognitionEvidence;
    scheme?: RecognitionScheme | null;
    expiresAt: number;
    confirmationExpired?: boolean;
}
export interface RecognitionAuthorReview {
    purpose: RecognitionPurpose;
    title: string;
    blocks: {
        id: string;
        text: string;
        kind?: "paragraph" | "heading" | "table";
        headingLevel?: number;
        table?: string[][];
    }[];
    fields: {
        key: string;
        value: string;
        confirmed: boolean;
        candidateIndex?: number;
        manualCorrection?: boolean;
        targetField?: string;
        targetType?: string;
    }[];
    acceptPartial: boolean;
    numbering?: "keep" | "remove" | "rebuild";
    selectedStructureIds?: string[];
}
export interface RecognitionReceipt {
    operationId: string;
    status: "notRegistered" | "prepared" | "committed" | "recoveryPending" | "resolved" | "cancelled";
    planId: string | null;
    commitAllowed: boolean;
    preview: {
        summary?: UiMessage;
        source?: string;
        losses?: PublicImportLoss[];
        fields?: unknown[];
        coverage?: unknown;
    };
    nodeRefs: {
        workspaceId: string;
        nodeId: string;
    }[];
    resourceRefs: string[];
}
export interface RecognitionLocator {
    version: 1;
    identity: string;
    target: RecognitionTarget;
    operationId: string;
    planId?: string;
}
export interface RecognitionTransport {
    request<T>(path: string, body?: unknown): Promise<T>;
    identity(): string | null;
    target?(): RecognitionTarget | null;
    persist(locator: RecognitionLocator): void;
    remove(): void;
    id(): string;
    committed?(identity: string): void;
}
function invalidRecognitionResponse(): never { throw new UiError(uiMessage("error.invalidResponse")); }
function recognitionObject(value: unknown): Record<string, unknown> {
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalidRecognitionResponse();
    return value as Record<string, unknown>;
}
function recognitionText(value: unknown): string { if (typeof value !== "string") return invalidRecognitionResponse(); return value; }
function recognitionInteger(value: unknown): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) return invalidRecognitionResponse(); return value; }
function recognitionBoolean(value: unknown): boolean { if (typeof value !== "boolean") return invalidRecognitionResponse(); return value; }
function recognitionArray<T>(value: unknown, decode: (item: unknown) => T): T[] { if (!Array.isArray(value)) return invalidRecognitionResponse(); return value.map(decode); }
/** Built-in scheme labels are product messages; configured destination titles are authored text. */
export function decodeRecognitionCapabilities(value: unknown): RecognitionCapabilities {
    const item = recognitionObject(value);
    if (!exactKeys(item, ["version", "available", "executor", "formats", "models", "schemes", "limits", "implementationDigest"], ["targets"]) || item.version !== 1 || !["local", "selfHosted"].includes(String(item.executor))) return invalidRecognitionResponse();
    const limits = recognitionObject(item.limits);
    const limitKeys = ["inputBytes", "pages", "resultBytes", "jobSeconds", "pageSeconds", "concurrentExecutions", "retainedBytes"] as const;
    if (!exactKeys(limits, limitKeys)) return invalidRecognitionResponse();
    return {
        version: 1, available: recognitionBoolean(item.available), executor: item.executor as "local" | "selfHosted",
        formats: recognitionArray(item.formats, recognitionText), implementationDigest: recognitionText(item.implementationDigest),
        models: recognitionArray(item.models, value => { const model = recognitionObject(value); if (!exactKeys(model, ["id", "languages"])) return invalidRecognitionResponse(); return { id: recognitionText(model.id), languages: recognitionArray(model.languages, recognitionText) }; }),
        schemes: recognitionArray(item.schemes, value => { const scheme = recognitionObject(value), title = parseUiMessage(scheme.title); if (!exactKeys(scheme, ["id", "title"]) || !title) return invalidRecognitionResponse(); return { id: recognitionText(scheme.id), title }; }),
        limits: Object.fromEntries(limitKeys.map(key => [key, recognitionInteger(limits[key])])) as RecognitionCapabilities["limits"],
        ...(Object.hasOwn(item, "targets") ? { targets: recognitionArray(item.targets, value => { const target = recognitionObject(value); if (!exactKeys(target, ["id", "title", "available"])) return invalidRecognitionResponse(); return { id: recognitionText(target.id), title: recognitionText(target.title), available: recognitionBoolean(target.available) }; }) } : {}),
    };
}
/** Decode display slots after transport/clone; the worker evidence remains original evidence. */
export function decodeRecognitionJob(value: unknown): RecognitionJob {
    const item = recognitionObject(value);
    if (!exactKeys(item, ["version", "jobId", "status", "inputSha256", "optionsDigest", "expiresAt"], ["resultDigest", "result", "scheme", "confirmationExpired"]) || item.version !== 1 || !["pending", "running", "complete", "partial", "failed", "cancelled", "interrupted"].includes(String(item.status)) || !/^[a-f0-9]{64}$/.test(recognitionText(item.inputSha256))) return invalidRecognitionResponse();
    const cloned = structuredClone(item) as unknown as RecognitionJob;
    recognitionText(item.jobId); recognitionText(item.optionsDigest); recognitionInteger(item.expiresAt);
    if (Object.hasOwn(item, "resultDigest")) recognitionText(item.resultDigest);
    if (Object.hasOwn(item, "confirmationExpired")) recognitionBoolean(item.confirmationExpired);
    if (Object.hasOwn(item, "result")) {
        const evidence = recognitionObject(item.result), input = recognitionObject(evidence.input);
        if (input.sha256 !== item.inputSha256) return invalidRecognitionResponse();
    }
    return cloned;
}
export function decodeRecognitionReceipt(value: unknown): RecognitionReceipt {
    const item = recognitionObject(value);
    if (!exactKeys(item, ["operationId", "status", "planId", "commitAllowed", "preview", "nodeRefs", "resourceRefs"]) || !["notRegistered", "prepared", "committed", "recoveryPending", "resolved", "cancelled"].includes(String(item.status))) return invalidRecognitionResponse();
    if (item.commitAllowed === true && (item.status !== "prepared" || typeof item.planId !== "string" || !item.planId)) return invalidRecognitionResponse();
    const preview = recognitionObject(item.preview), cloned = structuredClone(preview) as RecognitionReceipt["preview"];
    if (Object.hasOwn(preview, "summary")) cloned.summary = parseUiMessage(preview.summary) ?? invalidRecognitionResponse();
    if (Object.hasOwn(preview, "losses")) cloned.losses = recognitionArray(preview.losses, decodePublicImportLoss);
    if (Object.hasOwn(preview, "source")) cloned.source = recognitionText(preview.source);
    if (Object.hasOwn(preview, "fields") && !Array.isArray(preview.fields)) return invalidRecognitionResponse();
    return { operationId: recognitionText(item.operationId), status: item.status as RecognitionReceipt["status"], planId: item.planId === null ? null : recognitionText(item.planId), commitAllowed: recognitionBoolean(item.commitAllowed), preview: cloned,
        nodeRefs: recognitionArray(item.nodeRefs, value => { const ref = recognitionObject(value); if (!exactKeys(ref, ["workspaceId", "nodeId"])) return invalidRecognitionResponse(); return { workspaceId: recognitionText(ref.workspaceId), nodeId: recognitionText(ref.nodeId) }; }),
        resourceRefs: recognitionArray(item.resourceRefs, recognitionText),
    };
}
function validateRecognitionReceipt(value: RecognitionReceipt, locator: RecognitionLocator): void {
    if (!value || value.operationId !== locator.operationId || !["notRegistered", "prepared", "committed", "recoveryPending", "resolved", "cancelled"].includes(value.status) || value.commitAllowed && !value.planId || locator.planId && value.planId && value.planId !== locator.planId || !value.preview || !Array.isArray(value.nodeRefs))
        throw new UiError(uiMessage("recognitionState.receiptPlanMismatch"));
}
export const recognitionTerminal = (job?: RecognitionJob): boolean => !!job && !["pending", "running"].includes(job.status);
export const recognitionActionable = (job?: RecognitionJob): boolean => !!job && ["complete", "partial"].includes(job.status) && !job.confirmationExpired && job.result?.workerFinalValidation !== false;
export function decodeRecognitionLocator(raw: string, identity: string): RecognitionLocator {
    const value = JSON.parse(raw) as RecognitionLocator;
    if (!value || value.version !== 1 || value.identity !== identity || !value.operationId || !value.target || !("slot" in value.target ? typeof value.target.slot === "string" : typeof value.target.sessionId === "string"))
        throw new UiError(uiMessage("recognitionState.recoveryUnconfirmed"));
    return value;
}
/** Freeze once, query unknown writes by original identity, never replay prepare/commit. */
export class RecognitionController {
    private epoch = 0;
    private notifiedCommit: string | null = null;
    pendingJobId?: string;
    job?: RecognitionJob;
    locator?: RecognitionLocator;
    receipt?: RecognitionReceipt;
    uncertain = false;
    busy = false;
    constructor(private readonly host: RecognitionTransport) { }
    private stamp(): {
        epoch: number;
        identity: string;
    } {
        const identity = this.host.identity();
        if (!identity)
            throw new UiError(uiMessage("recognitionState.currentIdentityUnavailable"));
        return { epoch: this.epoch, identity };
    }
    private current(stamp: {
        epoch: number;
        identity: string;
    }): boolean { return stamp.epoch === this.epoch && stamp.identity === this.host.identity(); }
    retire(): void { this.epoch++; this.job = undefined; this.pendingJobId = undefined; this.busy = false; }
    revoke(): void { this.retire(); this.locator = undefined; this.receipt = undefined; this.uncertain = false; this.host.remove(); }
    detach(): void { this.retire(); this.locator = undefined; this.receipt = undefined; this.uncertain = false; }
    hide(): void { this.retire(); this.receipt = undefined; this.uncertain = !!this.locator; }
    restore(locator: RecognitionLocator): void { this.locator = structuredClone(locator); this.uncertain = true; this.receipt = undefined; }
    private operationTarget(): RecognitionTarget {
        if (!this.locator || this.locator.identity !== this.host.identity())
            throw new UiError(uiMessage("recognitionState.originalIdentityUnavailable"));
        const current = this.host.target?.();
        if (current && "sessionId" in current && "sessionId" in this.locator.target && current.sessionId !== this.locator.target.sessionId) {
            this.locator = { ...this.locator, target: { ...this.locator.target, sessionId: current.sessionId } };
            this.host.persist(this.locator);
        }
        return structuredClone(this.locator.target);
    }
    private acceptJob(value: RecognitionJob, jobId: string): void {
        value = decodeRecognitionJob(value);
        if (!value || value.version !== 1 || value.jobId !== jobId || !["pending", "running", "complete", "partial", "failed", "cancelled", "interrupted"].includes(value.status) || !/^[a-f0-9]{64}$/.test(value.inputSha256) || value.result && value.result.input.sha256 !== value.inputSha256)
            throw new UiError(uiMessage("recognitionState.responseOriginalMismatch"));
        this.job = value;
    }
    async start(target: RecognitionTarget, format: string, bytesBase64: string, options: RecognitionOptions): Promise<void> {
        if (this.busy || this.locator)
            throw new UiError(uiMessage("recognitionState.checkOriginalImportFirst"));
        this.retire();
        const stamp = this.stamp(), jobId = this.host.id();
        this.pendingJobId = jobId;
        this.busy = true;
        try {
            const value = await this.host.request<RecognitionJob>("/start", structuredClone({ jobId, target, format, bytesBase64, options }));
            if (this.current(stamp))
                this.acceptJob(value, jobId);
        }
        finally {
            if (this.current(stamp))
                this.busy = false;
        }
    }
    async status(): Promise<void> {
        const jobId = this.job?.jobId ?? this.pendingJobId;
        if (!jobId)
            return;
        const stamp = this.stamp();
        const value = await this.host.request<RecognitionJob>("/status", { jobId });
        if (this.current(stamp))
            this.acceptJob(value, jobId);
    }
    async cancel(): Promise<void> {
        const jobId = this.job?.jobId ?? this.pendingJobId;
        if (!jobId)
            return;
        const stamp = this.stamp();
        this.epoch++;
        const next = { ...stamp, epoch: this.epoch };
        this.job = undefined;
        this.busy = false;
        const value = await this.host.request<RecognitionJob>("/cancel", { jobId });
        if (this.current(next))
            this.acceptJob(value, jobId);
    }
    private acceptReceipt(value: RecognitionReceipt): void {
        if (!this.locator)
            throw new UiError(uiMessage("recognitionState.originalImportUnavailable"));
        value = decodeRecognitionReceipt(value);
        validateRecognitionReceipt(value, this.locator);
        this.locator = { ...this.locator, ...(value.planId ? { planId: value.planId } : {}) };
        this.receipt = value;
        this.uncertain = false;
        this.host.persist(this.locator);
    }
    private receiveReceipt(value: RecognitionReceipt, locator: RecognitionLocator, stamp: {
        epoch: number;
        identity: string;
    }): void {
        value = decodeRecognitionReceipt(value);
        validateRecognitionReceipt(value, locator);
        const key = JSON.stringify([locator.identity, locator.operationId]);
        try {
            if (this.current(stamp))
                this.acceptReceipt(value);
        }
        finally {
            if (value.status === "committed" && this.notifiedCommit !== key) {
                this.notifiedCommit = key;
                this.host.committed?.(locator.identity);
            }
        }
    }
    async prepare(target: RecognitionTarget, review: RecognitionAuthorReview): Promise<void> {
        if (this.busy || this.locator || !recognitionActionable(this.job))
            throw new UiError(uiMessage("recognitionState.finishReviewAndOperationFirst"));
        const stamp = this.stamp(), locator: RecognitionLocator = { version: 1, identity: stamp.identity, target: structuredClone(target), operationId: this.host.id() };
        this.host.persist(locator);
        this.locator = locator;
        this.receipt = undefined;
        this.uncertain = true;
        this.busy = true;
        const request = structuredClone({ jobId: this.job!.jobId, operationId: locator.operationId, target, review });
        try {
            const value = await this.host.request<RecognitionReceipt>("/prepare", request);
            this.receiveReceipt(value, locator, stamp);
        }
        finally {
            if (this.current(stamp))
                this.busy = false;
        }
    }
    async commit(acceptLosses: boolean): Promise<void> {
        if (this.busy || this.uncertain || !this.locator?.planId || !this.receipt?.commitAllowed || !acceptLosses)
            throw new UiError(uiMessage("recognitionState.queryPlanAcceptLossesFirst"));
        const stamp = this.stamp(), body = structuredClone({ target: this.operationTarget(), operationId: this.locator.operationId, planId: this.locator.planId, acceptLosses });
        this.host.persist(this.locator);
        const locator = structuredClone(this.locator!);
        this.busy = this.uncertain = true;
        try {
            const value = await this.host.request<RecognitionReceipt>("/commit", body);
            this.receiveReceipt(value, locator, stamp);
        }
        finally {
            if (this.current(stamp))
                this.busy = false;
        }
    }
    async lookup(): Promise<void> {
        if (!this.locator || this.busy)
            return;
        const stamp = this.stamp(), body = structuredClone({ target: this.operationTarget(), operationId: this.locator.operationId });
        const locator = structuredClone(this.locator!);
        this.busy = true;
        try {
            const value = await this.host.request<RecognitionReceipt>("/operation", body);
            this.receiveReceipt(value, locator, stamp);
        }
        finally {
            if (this.current(stamp))
                this.busy = false;
        }
    }
    async settle(cancel = false): Promise<void> {
        if (!this.locator || this.busy)
            return;
        const stamp = this.stamp(), body = structuredClone({ target: this.operationTarget(), operationId: this.locator.operationId });
        this.host.persist(this.locator);
        const locator = structuredClone(this.locator!);
        this.busy = this.uncertain = true;
        try {
            const value = await this.host.request<RecognitionReceipt>(cancel ? "/cancel-import" : "/settle", body);
            this.receiveReceipt(value, locator, stamp);
        }
        finally {
            if (this.current(stamp))
                this.busy = false;
        }
    }
    finish(): void {
        if (this.uncertain || !this.receipt || !["committed", "resolved", "cancelled"].includes(this.receipt.status))
            throw new UiError(uiMessage("recognitionState.originalOperationNotEnded"));
        this.host.remove();
        this.locator = undefined;
        this.receipt = undefined;
    }
}
