import { message as uiMessage, UiError, type UiMessage } from "./ui-locale";
import type { ResourceMetadata } from "./media-wire";
import { decodeImportCollection, decodeResourceImportReceipt, importInteger, importKeys, importText } from "./hosted-import-wire";
export interface HostedResourceCollection {
    collection: string;
    owner: string;
    title: string;
    lifecycle: "live" | "trash";
    read: boolean;
    download: boolean;
    import: boolean;
    status: "ready" | "unavailable";
}
export interface ResourceImportRequest {
    collection: string;
    operationId: string;
    name: string;
    bytesBase64: string;
}
export interface ResourceImportReceipt {
    collection: string;
    owner: string;
    operationId: string;
    status: string;
    planId: string | null;
    preview: {
        summary?: UiMessage;
        name?: string;
        bytes?: number;
        contentType?: string;
        metadata?: ResourceMetadata;
    };
    resourceRefs: string[];
    assets: string[];
    commitAllowed: boolean;
}
export interface ResourceImportDraft {
    collection: string;
    name: string;
    bytesBase64?: string;
    fileName?: string;
    bytes?: number;
    revision: number;
    pending?: {
        request: ResourceImportRequest;
        owner: string;
        input: string;
        revision: number;
        receipt?: ResourceImportReceipt;
    };
}
export const resourceImportEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export const resourceImportInput = (draft: ResourceImportDraft) => JSON.stringify([draft.collection, draft.name, draft.bytesBase64 ?? null]);
export function resourceCollection(value: unknown): HostedResourceCollection { return decodeImportCollection(value); }
function importBytes(bytesBase64: string): number {
    if (typeof bytesBase64 !== "string" || bytesBase64.length > 22369624 || bytesBase64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(bytesBase64))
        throw new UiError(uiMessage("hostedMediaImportState.originalBytesIncomplete"));
    const binary = atob(bytesBase64), length = binary.length;
    if (btoa(binary) !== bytesBase64) throw new UiError(uiMessage("hostedMediaImportState.originalBytesIncomplete"));
    if (length > 16 * 1024 * 1024)
        throw new UiError(uiMessage("hostedMediaImportState.sizeLimitExceeded"));
    return length;
}
export function freezeResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined, operationId: string): ResourceImportRequest {
    if (!collection || collection.collection !== draft.collection || collection.status !== "ready" || !collection.import || collection.lifecycle !== "live")
        throw new UiError(uiMessage("hostedMediaImportState.collectionImportNotPermitted"));
    if (draft.pending)
        throw new UiError(uiMessage("hostedMediaImportState.originalOperationMustBeResolved"));
    if (!draft.name || new TextEncoder().encode(draft.name).length > 240 || /[\\/:\u0000-\u001f\u007f-\u009f]/.test(draft.name) || /[. ]$/.test(draft.name) || [".", ".."].includes(draft.name) || typeof draft.bytesBase64 !== "string" || importBytes(draft.bytesBase64) !== draft.bytes)
        throw new UiError(uiMessage("hostedMediaImportState.resourceNameOrBytesIncomplete"));
    return { collection: draft.collection, operationId, name: draft.name, bytesBase64: draft.bytesBase64 };
}
export function resourceImportReceipt(request: ResourceImportRequest, value: unknown, previous?: ResourceImportReceipt): ResourceImportReceipt {
    const incoming = decodeResourceImportReceipt(value);
    if (incoming.collection !== request.collection || incoming.operationId !== request.operationId)
        throw new UiError(uiMessage("hostedMediaImportState.receiptMismatch"));
    if (previous?.planId && incoming.planId && previous.planId !== incoming.planId || resourceImportEnded(previous?.status) && incoming.status !== previous?.status)
        throw new UiError(uiMessage("hostedMediaImportState.originalFactsChanged"));
    if (incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId || !incoming.preview.summary || incoming.preview.name !== request.name || incoming.preview.bytes !== importBytes(request.bytesBase64)))
        throw new UiError(uiMessage("hostedMediaImportState.previewOriginalMissing"));
    // Incoming permission/config filtering replaces, rather than revives, any
    // old preview or locator. A committed fact survives without its content.
    return incoming;
}
export function canCommitResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined, verified: boolean): boolean {
    const pending = draft.pending;
    return verified && !!pending && !!collection && collection.collection === pending.request.collection && collection.owner === pending.owner && collection.import && collection.status === "ready" && collection.lifecycle === "live" && pending.receipt?.status === "prepared" && pending.receipt.commitAllowed && !!pending.receipt.planId && pending.revision === draft.revision && pending.input === resourceImportInput(draft);
}
export function decodeResourceImportDraft(raw: string): ResourceImportDraft {
    const stored = importKeys(JSON.parse(raw), ["collection", "name", "revision"], ["bytesBase64", "fileName", "bytes", "pending"]);
    const draft: ResourceImportDraft = { collection: importText(stored.collection), name: importText(stored.name), revision: importInteger(stored.revision) };
    if (Object.hasOwn(stored, "fileName")) draft.fileName = importText(stored.fileName);
    if (Object.hasOwn(stored, "bytes")) draft.bytes = importInteger(stored.bytes);
    if (Object.hasOwn(stored, "bytesBase64")) draft.bytesBase64 = importText(stored.bytesBase64);
    if (draft.bytesBase64 !== undefined && (importBytes(draft.bytesBase64) !== draft.bytes || typeof draft.fileName !== "string"))
        throw new UiError(uiMessage("hostedMediaImportState.recoveryUnparseable"));
    if (Object.hasOwn(stored, "pending")) {
        const pending = importKeys(stored.pending, ["request", "owner", "input", "revision"], ["receipt"]), request = importKeys(pending.request, ["collection", "operationId", "name", "bytesBase64"]);
        const p: NonNullable<ResourceImportDraft["pending"]> = { request: { collection: importText(request.collection, true), operationId: importText(request.operationId, true), name: importText(request.name), bytesBase64: importText(request.bytesBase64) }, owner: importText(pending.owner, true), input: importText(pending.input), revision: importInteger(pending.revision) };
        importBytes(p.request.bytesBase64);
        if (Object.hasOwn(pending, "receipt")) p.receipt = resourceImportReceipt(p.request, pending.receipt);
        draft.pending = p;
    }
    return draft;
}
