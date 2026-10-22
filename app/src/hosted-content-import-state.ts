import { message as uiMessage, UiError, type UiMessage } from "./ui-locale";
import type { ErrorDescription } from "./ui-diagnostics";
import type { NativeValue } from "./native-values";
import type { OutputLoss } from "./office-wire";
import { contentFormats, decodeContentInspection, decodeContentImportReceipt, importKeys, importText, importInteger, invalidImportResponse, type PublicImportLoss } from "./hosted-import-wire";
export { contentFormats } from "./hosted-import-wire";
export type ContentFormat = typeof contentFormats[number];
export interface ContentTarget {
    id: string;
    title: string;
    formats: ContentFormat[];
    available: boolean;
}
export type ContentLoss = OutputLoss;
export interface ContentInspection {
    uploadDigest: string;
    recordCount?: number;
    records?: { suggestedKey: string | null; publication: NativeValue; originalRecord: unknown; unknownFields: unknown; sources: { path: string; originalField: string }[]; conflicts: { key: string; current: NativeValue; imported: NativeValue }[] }[];
    losses: ContentLoss[];
    canPrepare: boolean;
    reason?: ErrorDescription | null;
}
export interface ContentRequest {
    slot: string;
    operationId: string;
    format: ContentFormat;
    bytesBase64: string;
}
export interface ContentReceipt {
    slot: string;
    operationId: string;
    status: string;
    planId: string | null;
    preview: {
        summary?: UiMessage;
        losses?: PublicImportLoss[];
        recordCount?: number;
        sharedWrites?: string[];
    };
    nodeRefs: {
        workspaceId: string;
        nodeId: string;
    }[];
    commitAllowed: boolean;
}
export interface ContentDraft {
    slot: string;
    format: ContentFormat;
    revision: number;
    bytesBase64?: string;
    fileName?: string;
    bytes?: number;
    job?: {
        request: ContentRequest;
        fileName: string;
        revision: number;
        receipt?: ContentReceipt;
        phase?: "prepare" | "commit" | "settle";
        commit?: {
            slot: string;
            operationId: string;
            planId: string;
            acceptLosses: boolean;
        };
    };
}
export interface ContentMembers {
    scope: "configuredImportSlotMembers";
    coverage: "complete" | "partial";
    members: {
        resource: string;
        nodeRef: {
            workspaceId: string;
            nodeId: string;
        };
        title: string;
        status: "ready" | "unavailable";
        canOpen: boolean;
    }[];
}
export const contentEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export function contentBytes(value: string): Uint8Array {
    if (typeof value !== "string" || value.length > 22369624 || value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value))
        throw new UiError(uiMessage("hostedContentImportState.originalBytesInvalid"));
    const binary = atob(value);
    if (binary.length > 16 * 1024 * 1024 || btoa(binary) !== value)
        throw new UiError(uiMessage("hostedContentImportState.originalBytesIncomplete"));
    return Uint8Array.from(binary, c => c.charCodeAt(0));
}
export function contentInspection(value: unknown, digest: string): ContentInspection {
    const decoded = decodeContentInspection(value);
    if (decoded.uploadDigest !== digest || !/^[a-f0-9]{64}$/.test(digest))
        throw new UiError(uiMessage("hostedContentImportState.inspectionOriginalMismatch"));
    if (decoded.canPrepare && decoded.records?.some(record => record.conflicts.length > 0))
        throw new UiError(uiMessage("hostedContentImportState.inspectionFieldConflicts"));
    return decoded;
}
export function freezeContentImport(draft: ContentDraft, target: ContentTarget | undefined, inspected: {
    revision: number;
    value: ContentInspection;
} | undefined, operationId: string): ContentRequest {
    if (draft.job)
        throw new UiError(uiMessage("hostedContentImportState.originalOperationMustBeResolved"));
    if (!target || target.id !== draft.slot || !target.available || !target.formats.includes(draft.format))
        throw new UiError(uiMessage("hostedContentImportState.fixedTargetUnavailable"));
    if (typeof draft.bytesBase64 !== "string" || contentBytes(draft.bytesBase64).length !== draft.bytes || !inspected?.value.canPrepare || inspected.revision !== draft.revision)
        throw new UiError(uiMessage("hostedContentImportState.inspectExactCurrentOriginal"));
    return { slot: draft.slot, format: draft.format, bytesBase64: draft.bytesBase64, operationId };
}
export function contentReceipt(request: ContentRequest, incoming: unknown, previous?: ContentReceipt): ContentReceipt {
    const value = decodeContentImportReceipt(incoming);
    if (value.slot !== request.slot || value.operationId !== request.operationId)
        throw new UiError(uiMessage("hostedContentImportState.receiptMismatch"));
    if (value.commitAllowed && (value.status !== "prepared" || !value.planId || !value.preview.summary))
        throw new UiError(uiMessage("hostedContentImportState.planCommitEligibilityMissing"));
    if (contentEnded(previous?.status) && value.status !== previous?.status || previous?.planId && value.planId && previous.planId !== value.planId)
        throw new UiError(uiMessage("hostedContentImportState.originalFactsChanged"));
    return value; // Filtered status replaces old private preview; parsed messages keep their brands.
}
export function canCommitContent(draft: ContentDraft, verified: boolean, reviewedRevision: number | undefined): boolean {
    return verified && reviewedRevision === draft.revision && !!draft.job && draft.job.receipt?.status === "prepared" && draft.job.receipt.commitAllowed && !!draft.job.receipt.planId;
}
export function decodeContentDraft(raw: string): ContentDraft {
    const record = importKeys(JSON.parse(raw), ["slot", "format", "revision"], ["bytesBase64", "fileName", "bytes", "job"]);
    const value: ContentDraft = { slot: importText(record.slot), format: record.format as ContentFormat, revision: importInteger(record.revision) };
    if (!contentFormats.includes(value.format)) return invalidImportResponse();
    if (Object.hasOwn(record, "fileName")) value.fileName = importText(record.fileName);
    if (Object.hasOwn(record, "bytes")) value.bytes = importInteger(record.bytes);
    if (Object.hasOwn(record, "bytesBase64")) value.bytesBase64 = importText(record.bytesBase64);
    if (value.bytesBase64 !== undefined && (contentBytes(value.bytesBase64).length !== value.bytes || typeof value.fileName !== "string"))
        throw new UiError(uiMessage("hostedContentImportState.recoveryUnparseable"));
    if (Object.hasOwn(record, "job")) {
        const stored = importKeys(record.job, ["request", "fileName", "revision"], ["receipt", "phase", "commit"]), request = importKeys(stored.request, ["slot", "operationId", "format", "bytesBase64"]);
        const job: NonNullable<ContentDraft["job"]> = { request: { slot: importText(request.slot, true), operationId: importText(request.operationId, true), format: request.format as ContentFormat, bytesBase64: importText(request.bytesBase64) }, fileName: importText(stored.fileName), revision: importInteger(stored.revision) };
        if (!contentFormats.includes(job.request.format))
            throw new UiError(uiMessage("hostedContentImportState.originalRequestInvalid"));
        contentBytes(job.request.bytesBase64);
        if (Object.hasOwn(stored, "receipt")) job.receipt = contentReceipt(job.request, stored.receipt);
        if (Object.hasOwn(stored, "phase")) { if (!["prepare", "commit", "settle"].includes(String(stored.phase))) return invalidImportResponse(); job.phase = stored.phase as NonNullable<ContentDraft["job"]>["phase"]; }
        if (Object.hasOwn(stored, "commit")) {
            const commit = importKeys(stored.commit, ["slot", "operationId", "planId", "acceptLosses"]);
            if (commit.slot !== job.request.slot || commit.operationId !== job.request.operationId || typeof commit.acceptLosses !== "boolean") throw new UiError(uiMessage("hostedContentImportState.confirmationRequestInvalid"));
            job.commit = { slot: job.request.slot, operationId: job.request.operationId, planId: importText(commit.planId, true), acceptLosses: commit.acceptLosses };
        }
        value.job = job;
    }
    return value;
}
