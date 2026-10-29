import { message as uiMessage, UiError, type UiMessage } from "./ui-locale";
import type { ResourceMetadata } from "./media-wire";
import { decodeImportCollection, decodeResourceImportReceipt, importInteger, importKeys, importText } from "./hosted-import-wire";
export interface HostedResourceCollection { collection: string; owner: string; title: string; lifecycle: "live" | "trash"; read: boolean; download: boolean; import: boolean; status: "ready" | "unavailable" }
export const hostedImportFileBytes = 16 * 1024 * 1024;
export const hostedImportBatchBytes = 32 * 1024 * 1024;
export const hostedImportMaxFiles = 64;
export interface ResourceImportFile { name: string; bytesBase64: string }
export interface ResourceImportRequest { collection: string; operationId: string; files: ResourceImportFile[] }
export interface ResourceImportFileInput { name: string; fileName: string; bytes: number; sha256?: string; bytesBase64?: string }
export interface ResourceImportLocator { collection: string; operationId: string; files: { name: string; bytes: number; sha256: string }[]; wireDigest: string }
export interface ResourceImportReceipt {
    collection: string; owner: string; operationId: string; status: string; planId: string | null;
    preview: { summary?: UiMessage; files?: { name: string; bytes: number; contentType: string; metadata: ResourceMetadata }[]; totalBytes?: number };
    resourceRefs: string[]; assets: string[]; results: { name: string; resourceRef: string; exactRevision: string }[]; commitAllowed: boolean;
}
export interface ResourceImportDraft {
    version: 2; collection: string; files: ResourceImportFileInput[]; revision: number;
    pending?: { request: ResourceImportLocator; owner: string; input: string; revision: number; receipt?: ResourceImportReceipt };
}
const shaPattern = /^[a-f0-9]{64}$/;
export const resourceImportEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export const resourceImportInput = (draft: ResourceImportDraft) => JSON.stringify([draft.collection, draft.files.map(file => [file.name, file.fileName, file.bytes, file.sha256 ?? null])]);
export function resourceCollection(value: unknown): HostedResourceCollection { return decodeImportCollection(value); }
export async function resourceImportDigest(bytes: Uint8Array): Promise<string> {
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", Uint8Array.from(bytes).buffer))].map(byte => byte.toString(16).padStart(2, "0")).join("");
}
function importBytes(bytesBase64: string): number {
    if (typeof bytesBase64 !== "string" || bytesBase64.length > Math.ceil(hostedImportFileBytes / 3) * 4 || bytesBase64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(bytesBase64)) throw new UiError(uiMessage("hostedMediaImportState.originalBytesIncomplete"));
    const binary = atob(bytesBase64);
    if (btoa(binary) !== bytesBase64) throw new UiError(uiMessage("hostedMediaImportState.originalBytesIncomplete"));
    if (binary.length > hostedImportFileBytes) throw new UiError(uiMessage("hostedMediaImportState.sizeLimitExceeded"));
    return binary.length;
}
export function checkResourceImportSizes(files: readonly { bytes: number }[]): void {
    if (!files.length || files.length > hostedImportMaxFiles) throw new UiError(uiMessage("hostedMediaImport.batchCountLimit"));
    let total = 0;
    for (const file of files) {
        if (!Number.isSafeInteger(file.bytes) || file.bytes < 0 || file.bytes > hostedImportFileBytes) throw new UiError(uiMessage("hostedMediaImportState.sizeLimitExceeded"));
        total += file.bytes;
        if (total > hostedImportBatchBytes) throw new UiError(uiMessage("hostedMediaImport.batchTotalLimit"));
    }
}
function validName(name: string): boolean { return !!name && new TextEncoder().encode(name).length <= 240 && !/[\\/:\u0000-\u001f\u007f-\u009f]/.test(name) && !/[. ]$/.test(name) && ![".", ".."].includes(name); }
export function freezeResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined, operationId: string): ResourceImportRequest {
    if (!collection || collection.collection !== draft.collection || collection.status !== "ready" || !collection.import || collection.lifecycle !== "live") throw new UiError(uiMessage("hostedMediaImportState.collectionImportNotPermitted"));
    if (draft.pending) throw new UiError(uiMessage("hostedMediaImportState.originalOperationMustBeResolved"));
    checkResourceImportSizes(draft.files);
    const files = draft.files.map(file => {
        if (!validName(file.name) || !file.sha256 || !shaPattern.test(file.sha256) || typeof file.bytesBase64 !== "string" || importBytes(file.bytesBase64) !== file.bytes) throw new UiError(uiMessage("hostedMediaImportState.resourceNameOrBytesIncomplete"));
        return { name: file.name, bytesBase64: file.bytesBase64 };
    });
    return { collection: draft.collection, operationId, files };
}
export async function resourceImportLocator(request: ResourceImportRequest): Promise<ResourceImportLocator> {
    const files = await Promise.all(request.files.map(async file => {
        const binary = atob(file.bytesBase64), bytes = Uint8Array.from(binary, char => char.charCodeAt(0));
        return { name: file.name, bytes: bytes.length, sha256: await resourceImportDigest(bytes) };
    }));
    return { collection: request.collection, operationId: request.operationId, files, wireDigest: await resourceImportDigest(new TextEncoder().encode(JSON.stringify(request))) };
}
export function resourceImportReceipt(request: ResourceImportLocator, value: unknown, previous?: ResourceImportReceipt): ResourceImportReceipt {
    const incoming = decodeResourceImportReceipt(value);
    if (incoming.collection !== request.collection || incoming.operationId !== request.operationId) throw new UiError(uiMessage("hostedMediaImportState.receiptMismatch"));
    if (previous?.planId && incoming.planId && previous.planId !== incoming.planId || resourceImportEnded(previous?.status) && incoming.status !== previous?.status) throw new UiError(uiMessage("hostedMediaImportState.originalFactsChanged"));
    if (incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId || !incoming.preview.summary || incoming.preview.files?.length !== request.files.length || incoming.preview.files.some((file, index) => file.name !== request.files[index].name || file.bytes !== request.files[index].bytes))) throw new UiError(uiMessage("hostedMediaImportState.previewOriginalMissing"));
    const refs = new Set(incoming.resourceRefs);
    if (incoming.results.length && (incoming.status !== "committed" || incoming.results.length !== request.files.length || refs.size !== incoming.results.length || incoming.results.some((file, index) => file.name !== request.files[index].name || !refs.has(file.resourceRef)) || new Set(incoming.results.map(file => file.resourceRef)).size !== refs.size || incoming.assets.length !== request.files.length)) throw new UiError(uiMessage("hostedMediaImportState.receiptMismatch"));
    return incoming;
}
export function canCommitResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined, verified: boolean): boolean {
    const pending = draft.pending;
    return verified && !!pending && !!collection && collection.collection === pending.request.collection && collection.owner === pending.owner && collection.import && collection.status === "ready" && collection.lifecycle === "live" && pending.receipt?.status === "prepared" && pending.receipt.commitAllowed && !!pending.receipt.planId && pending.revision === draft.revision && pending.input === resourceImportInput(draft);
}
/** The only stored state is the original locator and own file metadata, never upload bytes. */
export function encodeResourceImportDraft(draft: ResourceImportDraft): string {
    return JSON.stringify({ ...draft, files: draft.files.map(({ bytesBase64: _bytes, ...file }) => file) });
}
export function decodeResourceImportDraft(raw: string): ResourceImportDraft {
    const stored = importKeys(JSON.parse(raw), ["version", "collection", "files", "revision"], ["pending"]);
    if (stored.version !== 2 || !Array.isArray(stored.files)) throw new UiError(uiMessage("hostedMediaImportState.recoveryUnparseable"));
    const draft: ResourceImportDraft = { version: 2, collection: importText(stored.collection), files: stored.files.map(value => {
        const file = importKeys(value, ["name", "fileName", "bytes"], ["sha256"]), sha256 = file.sha256 === undefined ? undefined : importText(file.sha256);
        if (sha256 !== undefined && !shaPattern.test(sha256)) throw new UiError(uiMessage("hostedMediaImportState.recoveryUnparseable"));
        return { name: importText(file.name), fileName: importText(file.fileName), bytes: importInteger(file.bytes), ...(sha256 === undefined ? {} : { sha256 }) };
    }), revision: importInteger(stored.revision) };
    if (Object.hasOwn(stored, "pending")) {
        const pending = importKeys(stored.pending, ["request", "owner", "input", "revision"], ["receipt"]), request = importKeys(pending.request, ["collection", "operationId", "files", "wireDigest"]);
        if (!Array.isArray(request.files) || !shaPattern.test(importText(request.wireDigest))) throw new UiError(uiMessage("hostedMediaImportState.recoveryUnparseable"));
        const locator: ResourceImportLocator = { collection: importText(request.collection, true), operationId: importText(request.operationId, true), wireDigest: request.wireDigest as string, files: request.files.map(value => {
            const file = importKeys(value, ["name", "bytes", "sha256"]), name = importText(file.name), sha256 = importText(file.sha256);
            if (!validName(name) || !shaPattern.test(sha256)) throw new UiError(uiMessage("hostedMediaImportState.recoveryUnparseable"));
            return { name, bytes: importInteger(file.bytes), sha256 };
        }) };
        checkResourceImportSizes(locator.files);
        const p: NonNullable<ResourceImportDraft["pending"]> = { request: locator, owner: importText(pending.owner, true), input: importText(pending.input), revision: importInteger(pending.revision) };
        if (Object.hasOwn(pending, "receipt")) p.receipt = resourceImportReceipt(locator, pending.receipt);
        draft.pending = p;
    }
    return draft;
}
