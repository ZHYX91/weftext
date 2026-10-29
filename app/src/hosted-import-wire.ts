import { decodeNativeValue } from "./native-values";
import { decodeOutputLoss, officeLossUiText, type OutputLoss } from "./office-wire";
import { decodeErrorDescription, exactKeys } from "./ui-diagnostics";
import { message, parseUiMessage, UiError, type UiText } from "./ui-locale";
import { decodeResourceMetadata } from "./media-wire";
import type { ContentInspection, ContentMembers, ContentReceipt, ContentTarget } from "./hosted-content-import-state";
import type { HostedResourceCollection, ResourceImportReceipt } from "./hosted-media-import-state";

export const contentFormats = ["native", "bibtex", "csl-json", "ris"] as const;
export const importStatuses = ["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"] as const;
const safeLossCodes = ["word-layout", "word-list-numbering", "word-format-revision", "word-comment-range", "word-comment-unlocated", "word-nested-revision", "word-link", "word-image-format", "word-image-layout", "word-image-missing", "word-field", "word-element", "word-block", "word-table-layout", "docx-import-loss", "generatedBibliographyNotInstalled", "resourceModuleContextPreserved", "settingsNotApplied", "unregisteredFileOmitted", "content-import-loss", "structure-copy-loss"] as const;
export type PublicImportLossCode = typeof safeLossCodes[number];
export interface PublicImportLoss extends Omit<OutputLoss, "code" | "location"> { code: PublicImportLossCode }
export function invalidImportResponse(): never { throw new UiError(message("error.invalidResponse")); }
export function importRecord(value: unknown): Record<string, unknown> {
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalidImportResponse();
    return value as Record<string, unknown>;
}
export function importKeys(value: unknown, required: readonly string[], optional: readonly string[] = []): Record<string, unknown> {
    const item = importRecord(value);
    if (!exactKeys(item, required, optional)) return invalidImportResponse();
    return item;
}
export function importText(value: unknown, nonempty = false): string {
    if (typeof value !== "string" || nonempty && !value) return invalidImportResponse();
    return value;
}
export function importInteger(value: unknown): number {
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) return invalidImportResponse();
    return value;
}
function bool(value: unknown): boolean { if (typeof value !== "boolean") return invalidImportResponse(); return value; }
function array<T>(value: unknown, decode: (value: unknown) => T): T[] { if (!Array.isArray(value)) return invalidImportResponse(); return value.map(decode); }
function strings(value: unknown): string[] { return array(value, value => importText(value)); }
function nodeRef(value: unknown): { workspaceId: string; nodeId: string } { const item = importKeys(value, ["workspaceId", "nodeId"]); return { workspaceId: importText(item.workspaceId, true), nodeId: importText(item.nodeId, true) }; }
function status(value: unknown): string { if (!importStatuses.includes(value as typeof importStatuses[number])) return invalidImportResponse(); return value as string; }
function format(value: unknown): typeof contentFormats[number] { if (!contentFormats.includes(value as typeof contentFormats[number])) return invalidImportResponse(); return value as typeof contentFormats[number]; }
function summary(value: unknown) { return parseUiMessage(value) ?? invalidImportResponse(); }

/** Public loss meanings are already selected by Core; no code-to-text translation happens here. */
export function decodePublicImportLoss(value: unknown): PublicImportLoss {
    const item = importKeys(value, ["code", "explanation", "diagnostics", "causes", "productDiagnostics"]);
    if (!safeLossCodes.includes(importText(item.code) as PublicImportLossCode)) return invalidImportResponse();
    return { ...decodeOutputLoss(item), code: item.code as PublicImportLossCode };
}
export function importLossUiText(value: unknown, publicLoss = false): UiText { return officeLossUiText(publicLoss ? decodePublicImportLoss(value) : decodeOutputLoss(value)); }

export function decodeContentTargets(value: unknown): ContentTarget[] {
    const item = importKeys(value, ["scope", "targets"]);
    if (item.scope !== "configuredImportSlots") return invalidImportResponse();
    return array(item.targets, value => { const target = importKeys(value, ["id", "title", "formats", "available"]); return { id: importText(target.id, true), title: importText(target.title), formats: array(target.formats, format), available: bool(target.available) }; });
}

/** Only the bibliography adapter's declared fields are interpreted; original JSON objects stay author data. */
export function decodeContentInspection(value: unknown): ContentInspection {
    const item = importRecord(value), uploadDigest = importText(item.uploadDigest);
    if (!/^[a-f0-9]{64}$/.test(uploadDigest)) return invalidImportResponse();
    const losses = array(item.losses, decodeOutputLoss), canPrepare = bool(item.canPrepare);
    if (Object.hasOwn(item, "recordCount")) {
        importKeys(item, ["uploadDigest", "recordCount", "losses", "canPrepare"]);
        if (!canPrepare || losses.length) return invalidImportResponse();
        return { uploadDigest, recordCount: importInteger(item.recordCount), losses, canPrepare };
    }
    importKeys(item, ["uploadDigest", "records", "losses", "canPrepare", "reason"]);
    const reason = item.reason === null ? null : decodeErrorDescription(item.reason) ?? invalidImportResponse();
    const records = array(item.records, value => {
        const record = importKeys(value, ["suggestedKey", "publication", "originalRecord", "unknownFields", "sources", "conflicts"]);
        const publication = decodeNativeValue(record.publication) ?? invalidImportResponse();
        const sources = array(record.sources, value => { const source = importKeys(value, ["path", "originalField"]); return { path: importText(source.path), originalField: importText(source.originalField) }; });
        const conflicts = array(record.conflicts, value => { const conflict = importKeys(value, ["key", "current", "imported"]); return { key: importText(conflict.key), current: decodeNativeValue(conflict.current) ?? invalidImportResponse(), imported: decodeNativeValue(conflict.imported) ?? invalidImportResponse() }; });
        return { suggestedKey: record.suggestedKey === null ? null : importText(record.suggestedKey), publication, originalRecord: record.originalRecord, unknownFields: record.unknownFields, sources, conflicts };
    });
    if (canPrepare !== (reason === null) || canPrepare && records.some(record => record.conflicts.length)) return invalidImportResponse();
    return { uploadDigest, records, losses, canPrepare, reason };
}
export function decodeContentImportReceipt(value: unknown): ContentReceipt {
    const item = importKeys(value, ["slot", "operationId", "status", "planId", "preview", "nodeRefs", "commitAllowed"]), preview = importRecord(item.preview);
    let decoded: ContentReceipt["preview"] = {};
    if (Object.keys(preview).length) {
        importKeys(preview, ["summary", "recordCount", "losses", "sharedWrites"]);
        decoded = { summary: summary(preview.summary), recordCount: importInteger(preview.recordCount), losses: array(preview.losses, decodePublicImportLoss), sharedWrites: strings(preview.sharedWrites) };
    }
    const commitAllowed = bool(item.commitAllowed), operationStatus = status(item.status), planId = item.planId === null ? null : importText(item.planId, true);
    if (commitAllowed ? operationStatus !== "prepared" || !planId || !decoded.summary : planId !== null) return invalidImportResponse();
    return { slot: importText(item.slot, true), operationId: importText(item.operationId, true), status: operationStatus, planId, preview: decoded, nodeRefs: array(item.nodeRefs, nodeRef), commitAllowed };
}
export function decodeContentMembers(value: unknown, slot: string): ContentMembers {
    const item = importKeys(value, ["scope", "coverage", "members"]);
    if (item.scope !== "configuredImportSlotMembers" || !["complete", "partial"].includes(String(item.coverage))) return invalidImportResponse();
    return { scope: "configuredImportSlotMembers", coverage: item.coverage as "complete" | "partial", members: array(item.members, value => {
        const member = importKeys(value, ["resource", "nodeRef", "title", "status", "canOpen"]), resource = importText(member.resource, true), ref = nodeRef(member.nodeRef), canOpen = bool(member.canOpen);
        if (resource !== `${slot}/${ref.nodeId}` || !["ready", "unavailable"].includes(String(member.status)) || canOpen && member.status !== "ready") return invalidImportResponse();
        return { resource, nodeRef: ref, title: importText(member.title), status: member.status as "ready" | "unavailable", canOpen };
    }) };
}
export function decodeImportCollection(value: unknown): HostedResourceCollection {
    const item = importKeys(value, ["collection", "owner", "title", "lifecycle", "read", "download", "import", "status"]);
    if (!["live", "trash"].includes(String(item.lifecycle)) || !["ready", "unavailable"].includes(String(item.status))) return invalidImportResponse();
    const result: HostedResourceCollection = { collection: importText(item.collection, true), owner: importText(item.owner, true), title: importText(item.title), lifecycle: item.lifecycle as "live" | "trash", read: bool(item.read), download: bool(item.download), import: bool(item.import), status: item.status as "ready" | "unavailable" };
    if (result.download && !result.read || result.lifecycle === "trash" && result.import) return invalidImportResponse();
    return result;
}
export function decodeResourceImportReceipt(value: unknown): ResourceImportReceipt {
    const item = importKeys(value, ["collection", "owner", "operationId", "status", "planId", "preview", "resourceRefs", "assets", "results", "commitAllowed"]), preview = importRecord(item.preview);
    let decoded: ResourceImportReceipt["preview"] = {};
    if (Object.keys(preview).length) {
        importKeys(preview, ["summary", "files", "totalBytes"]);
        const files = array(preview.files, value => { const file = importKeys(value, ["name", "bytes", "contentType", "metadata"]); return { name: importText(file.name), bytes: importInteger(file.bytes), contentType: importText(file.contentType), metadata: decodeResourceMetadata(file.metadata) }; }), totalBytes = importInteger(preview.totalBytes);
        if (!files.length || files.length > 64 || files.some(file => file.bytes > 16 * 1024 * 1024) || files.reduce((total, file) => total + file.bytes, 0) !== totalBytes || totalBytes > 32 * 1024 * 1024) return invalidImportResponse();
        decoded = { summary: summary(preview.summary), files, totalBytes };
    }
    const commitAllowed = bool(item.commitAllowed), operationStatus = status(item.status), planId = item.planId === null ? null : importText(item.planId, true);
    if (commitAllowed ? operationStatus !== "prepared" || !planId || !decoded.summary : planId !== null) return invalidImportResponse();
    const resourceRef = (value: unknown): string => { const ref = importText(value, true); return /^weftext-resource:\/\//.test(ref) ? ref : invalidImportResponse(); };
    const results = array(item.results, value => { const result = importKeys(value, ["name", "resourceRef", "exactRevision"]); return { name: importText(result.name), resourceRef: resourceRef(result.resourceRef), exactRevision: importText(result.exactRevision, true) }; });
    if (results.length > 64 || results.length && operationStatus !== "committed") return invalidImportResponse();
    return { collection: importText(item.collection, true), owner: importText(item.owner, true), operationId: importText(item.operationId, true), status: operationStatus, planId, preview: decoded, resourceRefs: array(item.resourceRefs, resourceRef), assets: array(item.assets, value => importText(value, true)), results, commitAllowed };
}
