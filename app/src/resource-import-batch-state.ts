import type { DocumentBasis, SourcePatch } from "./native-value-form";
import { isUnicodeScalarText, message, UiError } from "./ui-locale";
import { decodeWritingMap, exactWritingRange, validateWritingResult, type WritingMap, type WritingResult } from "./writing";

export type LocalResourceMode = "importOnly" | "autoAppend" | "linksAppend";
export interface LocalResourceOwner { workspaceId: string; nodeId: string; root: string; path: string; }
export interface LocalResourceFileInfo { name: string; fileName: string; bytes: number; sha256: string; }
/** A recovery locator, never a second source of commit facts or upload bytes. */
export interface LocalResourceLocator {
    version: 1;
    owner: LocalResourceOwner;
    operationId: string;
    requestKind: "operation";
    wireDigest: string;
    files: LocalResourceFileInfo[];
    mode: LocalResourceMode;
    insertion: "pending" | "applying" | "inserted" | "uncertain";
    phase: "original";
}
export interface LocalResourceResult { name: string; resourceRef: string; exactRevision: string; }
export interface LocalResourcePlan {
    operationId: string;
    planId: string;
    kind: "importResources";
    status: "prepared" | "committed" | "recoveryPending" | "cancelled" | "resolved";
    changes: { path: string; action: string; bytes: number }[];
    imports: LocalResourceResult[];
}
export type LocalResourceStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;
/** Keep the exact original JSON as the compare token, including its formatting. */
export interface LocalResourceRecord { readonly raw: string; readonly locator: LocalResourceLocator; }
export const localResourceMaxFiles = 64;
export const localResourceBatchBytes = 64 * 1024 * 1024;
const locatorBytes = 128 * 1024;
const documentBytes = 2 * 1024 * 1024;
const sha = /^[a-f0-9]{64}$/;
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const invalid = (): never => { throw new UiError(message("error.invalidResponse")); };
const object = (value: unknown): Record<string, unknown> | undefined => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : undefined;
function record(value: unknown, required: readonly string[], optional: readonly string[] = []): Record<string, unknown> {
    const item = object(value);
    if (!item || required.some(key => !Object.hasOwn(item, key)) || Object.keys(item).some(key => !required.includes(key) && !optional.includes(key))) return invalid();
    return item;
}
function text(value: unknown, maxBytes = 4096): string {
    if (typeof value !== "string" || !value.trim() || !isUnicodeScalarText(value) || /[\u0000-\u001f\u007f-\u009f]/u.test(value) || new TextEncoder().encode(value).length > maxBytes) return invalid();
    return value;
}
function integer(value: unknown): number {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : invalid();
}
function digest(value: unknown): string {
    return typeof value === "string" && sha.test(value) ? value : invalid();
}
function decodeOwner(value: unknown): LocalResourceOwner {
    const item = record(value, ["workspaceId", "nodeId", "root", "path"]);
    return { workspaceId: text(item.workspaceId), nodeId: text(item.nodeId), root: text(item.root), path: text(item.path) };
}
export function resourceOwner(basis: DocumentBasis): LocalResourceOwner {
    return decodeOwner({ workspaceId: basis.workspaceId, nodeId: basis.nodeId, root: basis.root, path: basis.path });
}
export function sameResourceOwner(a: LocalResourceOwner | null | undefined, b: LocalResourceOwner | null | undefined): boolean {
    return !!a && !!b && a.workspaceId === b.workspaceId && a.nodeId === b.nodeId && a.root === b.root && a.path === b.path;
}
export function resourceBasisKey(basis: DocumentBasis): string {
    resourceOwner(basis);
    return JSON.stringify([basis.inputId, basis.sessionId, basis.source, basis.serial, basis.baseId, basis.path, basis.root, basis.workspaceId, basis.nodeId, basis.inputRevision]);
}
export function localResourceKey(owner: LocalResourceOwner): string {
    const checked = decodeOwner(owner);
    // A renamed/moved owner must discover and block on its original locator,
    // rather than acquire a second slot that can allocate another operation.
    return `weftext:resource-import:v1:${encodeURIComponent(JSON.stringify([checked.workspaceId, checked.nodeId]))}`;
}
export function checkLocalResourceSizes(files: readonly { bytes: number }[]): void {
    if (!Array.isArray(files) || !files.length || files.length > localResourceMaxFiles) return invalid();
    let total = 0;
    for (const file of files) {
        if (!file || !Number.isSafeInteger(file.bytes) || file.bytes < 0 || file.bytes > localResourceBatchBytes) return invalid();
        total += file.bytes;
        if (total > localResourceBatchBytes) return invalid();
    }
}
export async function resourceDigest(value: Uint8Array | string): Promise<string> {
    if (typeof value === "string" && !isUnicodeScalarText(value)) return invalid();
    const bytes = typeof value === "string" ? new TextEncoder().encode(value) : Uint8Array.from(value);
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes.buffer))].map(byte => byte.toString(16).padStart(2, "0")).join("");
}
/** Preserve even a Core-rejected target name under its original operation ID.
 * This record is not a client-side replica of the Core namespace validator. */
function resourceName(value: unknown): string {
    return typeof value === "string" && isUnicodeScalarText(value) && new TextEncoder().encode(value).length <= 1024 ? value : invalid();
}
function locator(value: unknown): LocalResourceLocator {
    const item = record(value, ["version", "owner", "operationId", "requestKind", "wireDigest", "files", "mode", "insertion", "phase"]);
    if (item.version !== 1 || item.requestKind !== "operation" || item.phase !== "original"
        || typeof item.operationId !== "string" || !uuid.test(item.operationId)
        || !["importOnly", "autoAppend", "linksAppend"].includes(item.mode as string)
        || !["pending", "applying", "inserted", "uncertain"].includes(item.insertion as string)
        || !Array.isArray(item.files)) return invalid();
    const files = item.files.map(value => {
        const file = record(value, ["name", "fileName", "bytes", "sha256"]), fileName = text(file.fileName, 1024);
        if (/[\\/]/u.test(fileName)) return invalid();
        return { name: resourceName(file.name), fileName, bytes: integer(file.bytes), sha256: digest(file.sha256) };
    });
    checkLocalResourceSizes(files);
    return { version: 1, owner: decodeOwner(item.owner), operationId: item.operationId, requestKind: "operation", wireDigest: digest(item.wireDigest), files,
        mode: item.mode as LocalResourceMode, insertion: item.insertion as LocalResourceLocator["insertion"], phase: "original" };
}
export function decodeLocalResourceLocator(raw: string, owner?: LocalResourceOwner): LocalResourceLocator {
    try {
        if (typeof raw !== "string" || new TextEncoder().encode(raw).length > locatorBytes) return invalid();
        const decoded = locator(JSON.parse(raw));
        if (owner && !sameResourceOwner(decoded.owner, owner)) return invalid();
        return decoded;
    } catch { return invalid(); }
}
function checkedRecord(expected: LocalResourceRecord): LocalResourceLocator {
    const decoded = decodeLocalResourceLocator(expected.raw);
    if (JSON.stringify(decoded) !== JSON.stringify(locator(expected.locator))) return invalid();
    return decoded;
}
export function readLocalResourceLocator(storage: LocalResourceStorage, owner: LocalResourceOwner): LocalResourceRecord | null {
    try {
        const raw = storage.getItem(localResourceKey(owner));
        return raw === null ? null : { raw, locator: decodeLocalResourceLocator(raw, owner) };
    } catch { return invalid(); }
}
export function writeLocalResourceLocator(storage: LocalResourceStorage, value: LocalResourceLocator, expected: LocalResourceRecord | null): LocalResourceRecord {
    const decoded = locator(value), key = localResourceKey(decoded.owner), raw = JSON.stringify(decoded);
    if (new TextEncoder().encode(raw).length > locatorBytes) return invalid();
    if (expected) {
        const original = checkedRecord(expected);
        if (JSON.stringify({ ...original, insertion: decoded.insertion }) !== raw) return invalid();
    }
    try {
        if (storage.getItem(key) !== (expected?.raw ?? null)) return invalid();
        storage.setItem(key, raw);
        if (storage.getItem(key) !== raw) return invalid();
        return { raw, locator: decoded };
    } catch { return invalid(); }
}
export function removeLocalResourceLocator(storage: LocalResourceStorage, expected: LocalResourceRecord): void {
    const key = localResourceKey(checkedRecord(expected).owner);
    try {
        if (storage.getItem(key) !== expected.raw) return invalid();
        storage.removeItem(key);
        if (storage.getItem(key) !== null) return invalid();
    } catch { return invalid(); }
}
function imports(value: unknown, request: LocalResourceLocator): LocalResourceResult[] {
    if (!Array.isArray(value) || value.length !== request.files.length) return invalid();
    const prefix = `weftext-resource://${request.owner.workspaceId}/`;
    const result = value.map((value, index) => {
        const item = record(value, ["name", "resourceRef", "exactRevision"]), resourceRef = text(item.resourceRef);
        if (item.name !== request.files[index].name || !resourceRef.startsWith(prefix) || !uuid.test(resourceRef.slice(prefix.length))) return invalid();
        return { name: item.name as string, resourceRef, exactRevision: text(item.exactRevision) };
    });
    if (new Set(result.map(item => item.resourceRef)).size !== result.length) return invalid();
    return result;
}
export function decodeLocalResourcePlan(value: unknown, request: LocalResourceLocator): LocalResourcePlan {
    const original = locator(request), wrapper = record(value, ["operation", "imports"]);
    const operation = record(wrapper.operation, ["operationId", "planId", "kind", "status", "changes", "nodeMap"], ["newNodeId", "revisions", "resourceMap", "losses"]);
    if (operation.operationId !== original.operationId || operation.kind !== "importResources"
        || !["prepared", "committed", "recoveryPending", "cancelled", "resolved"].includes(operation.status as string)
        || !Array.isArray(operation.changes) || !operation.changes.length || operation.changes.length > localResourceMaxFiles + 1) return invalid();
    const nodeMap = object(operation.nodeMap), nodes = nodeMap && Object.values(nodeMap);
    if (!nodes || nodes.length !== 1 || nodes[0] !== original.owner.nodeId) return invalid();
    for (const key of ["nodeMap", "revisions", "resourceMap"] as const) {
        if (Object.hasOwn(operation, key)) {
            const map = object(operation[key]);
            if (!map || Object.values(map).some(value => typeof value !== "string" || !value || !isUnicodeScalarText(value))) return invalid();
        }
    }
    if (Object.hasOwn(operation, "newNodeId") && operation.newNodeId !== null) text(operation.newNodeId);
    if (Object.hasOwn(operation, "losses") && !Array.isArray(operation.losses)) return invalid();
    const changes = operation.changes.map(value => {
        const change = record(value, ["path", "action", "bytes"]);
        return { path: text(change.path), action: text(change.action, 128), bytes: integer(change.bytes) };
    });
    return { operationId: original.operationId, planId: text(operation.planId, 128), kind: "importResources", status: operation.status as LocalResourcePlan["status"], changes, imports: imports(wrapper.imports, original) };
}
/** The caller first obtains Core route qualification for this stable owner, then
 * keeps the complete current basis/map current across await. Frozen root/path
 * metadata cannot qualify a renamed or moved runtime route by themselves.
 * Native EditOutput.sourceHash belongs to the resulting source, not that input. */
export function decodeLocalResourceInsertion(value: unknown, request: LocalResourceLocator, plan: LocalResourcePlan, basis: DocumentBasis, sourceHash: string, originalMap: WritingMap): SourcePatch {
    const original = locator(request), currentOwner = resourceOwner(basis);
    if (original.owner.workspaceId !== currentOwner.workspaceId || original.owner.nodeId !== currentOwner.nodeId
        || plan.operationId !== original.operationId || plan.kind !== "importResources" || plan.status !== "committed"
        || original.mode === "importOnly" || original.insertion !== "pending" || !isUnicodeScalarText(basis.source) || !sha.test(sourceHash)) return invalid();
    text(plan.planId, 128);
    const inputMap = decodeWritingMap(originalMap), originalAt = inputMap.bodyInsertion;
    if (inputMap.sourceHash !== sourceHash || !inputMap.syntaxValid || inputMap.unavailableReason !== null || !originalAt
        || !originalAt.writable || originalAt.raw !== "" || originalAt.from !== originalAt.to || originalAt.utf16From !== originalAt.utf16To
        || !exactWritingRange(basis.source, originalAt)) return invalid();
    const wrapper = record(value, ["operationId", "imports", "edit"]);
    if (wrapper.operationId !== original.operationId || JSON.stringify(imports(wrapper.imports, original)) !== JSON.stringify(imports(plan.imports, original))) return invalid();
    const edit = record(wrapper.edit, ["source", "sourceHash", "patches", "selection", "map", "serial"]);
    if (edit.serial !== basis.serial || integer(edit.serial) !== edit.serial || typeof edit.source !== "string" || !isUnicodeScalarText(edit.source)
        || new TextEncoder().encode(edit.source).length > documentBytes || !Array.isArray(edit.patches) || edit.patches.length !== 1) return invalid();
    digest(edit.sourceHash);
    const nativePatch = record(edit.patches[0], ["from", "to", "utf16From", "utf16To", "expected", "insert"]);
    for (const key of ["from", "to", "utf16From", "utf16To"]) integer(nativePatch[key]);
    if (nativePatch.from !== originalAt.from || nativePatch.to !== originalAt.to || nativePatch.utf16From !== originalAt.utf16From || nativePatch.utf16To !== originalAt.utf16To || nativePatch.expected !== ""
        || typeof nativePatch.insert !== "string" || !nativePatch.insert || !isUnicodeScalarText(nativePatch.insert)) return invalid();
    const selection = record(edit.selection, ["from", "to"]);
    integer(selection.from); integer(selection.to);
    const outputMap = decodeWritingMap(edit.map);
    if (!outputMap.syntaxValid || outputMap.unavailableReason !== null) return invalid();
    // Reuse the native writing consumer's complete map, offset, source and
    // selection proof. Its returned patch is already in UTF-16 coordinates.
    const patches = validateWritingResult(basis.source, edit as unknown as WritingResult);
    const patch = patches[0];
    if (patches.length !== 1 || patch.from !== patch.to || patch.expected !== "") return invalid();
    return { from: patch.from, to: patch.to, text: patch.insert };
}
