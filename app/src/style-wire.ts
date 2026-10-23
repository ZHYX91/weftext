import { decodeApiErrorBody, type ApiErrorBody, type OperationReceiptFacts } from "./api-error";
import { decodeErrorDescription, decodeErrorDescriptions, exactKeys, type ErrorDescription } from "./ui-diagnostics";
import { message, parseUiMessage, UiError, type UiMessage } from "./ui-locale";
import type { ProcessingDocument } from "./processing-settings-panel";

type WireRecord = Record<string, unknown>;
export function invalidStyleWire(): never { throw Object.assign(new UiError(message("error.invalidResponse")), { code: "resultUnknown" }); }
function record(value: unknown): WireRecord { if (!value || typeof value !== "object" || Array.isArray(value)) invalidStyleWire(); return value as WireRecord; }
function keys(value: unknown, required: readonly string[], optional: readonly string[] = []): WireRecord { const item = record(value); if (!exactKeys(item, required, optional)) invalidStyleWire(); return item; }
function text(value: unknown, nonempty = false): string { if (typeof value !== "string" || nonempty && !value) invalidStyleWire(); return value; }
function bool(value: unknown): boolean { if (typeof value !== "boolean") invalidStyleWire(); return value; }
function nullableText(value: unknown): string | null { return value === null ? null : text(value); }
function array<T>(value: unknown, decode: (item: unknown) => T): T[] { if (!Array.isArray(value)) invalidStyleWire(); return value.map(decode); }
function uiMessage(value: unknown): UiMessage { return parseUiMessage(value) ?? invalidStyleWire(); }
function hash(value: unknown): string { const result = text(value); if (!/^[a-f0-9]{64}$/.test(result)) invalidStyleWire(); return result; }

/** The native owner exposes expressions as text; units and locale values remain authored source. */
export type NativeStyleValue = string;
export interface NativeStyleControl { key: string; label: UiMessage; kind: "text"; value: string; editable: boolean; reason: ErrorDescription | null; }
export function decodeNativeStyleControl(value: unknown): NativeStyleControl {
    const item = keys(value, ["key", "label", "kind", "value", "editable", "reason"]);
    if (item.kind !== "text") invalidStyleWire();
    return { key: text(item.key, true), label: uiMessage(item.label), kind: "text", value: text(item.value), editable: bool(item.editable), reason: item.reason === null ? null : decodeErrorDescription(item.reason) ?? invalidStyleWire() };
}
export interface LibraryStyleState { path: string; source: string; baseId: string; enabled: boolean; entryFunction: string; resourceRef: string | null; installed: boolean; sourceHash: string; syntaxValid: boolean; controls: NativeStyleControl[]; warnings: UiMessage[]; }
export function decodeLibraryStyleState(value: unknown): LibraryStyleState {
    const item = keys(value, ["path", "source", "baseId", "enabled", "entryFunction", "resourceRef", "installed", "sourceHash", "syntaxValid", "controls", "warnings"]);
    return { path: text(item.path, true), source: text(item.source), baseId: text(item.baseId, true), enabled: bool(item.enabled), entryFunction: text(item.entryFunction), resourceRef: nullableText(item.resourceRef), installed: bool(item.installed), sourceHash: hash(item.sourceHash), syntaxValid: bool(item.syntaxValid), controls: array(item.controls, decodeNativeStyleControl), warnings: array(item.warnings, uiMessage) };
}
export interface LibraryStylePlan { operationId: string; planId: string; beforeSource: string; afterSource: string; summary: UiMessage[]; enabled: boolean; status: string; }
export function decodeLibraryStylePlan(value: unknown): LibraryStylePlan {
    const item = keys(value, ["operationId", "planId", "beforeSource", "afterSource", "summary", "enabled", "status"]);
    if (item.status !== "prepared") invalidStyleWire();
    return { operationId: text(item.operationId, true), planId: text(item.planId, true), beforeSource: text(item.beforeSource), afterSource: text(item.afterSource), summary: array(item.summary, uiMessage), enabled: bool(item.enabled), status: "prepared" };
}
export function decodeStyleOperationReceipt(value: unknown): OperationReceiptFacts {
    const item = keys(value, ["operationId", "planId", "status", "path", "nodeIds"]);
    return { operationId: text(item.operationId, true), planId: text(item.planId, true), status: text(item.status), path: text(item.path), nodeIds: array(item.nodeIds, value => text(value)) };
}
export function decodeAppliedLibraryStyle(value: unknown): LibraryStyleState & { operationReceipt: OperationReceiptFacts } {
    const item = record(value), { operationReceipt, ...state } = item;
    return { ...decodeLibraryStyleState(state), operationReceipt: decodeStyleOperationReceipt(operationReceipt) };
}

export type LibraryRole = "schema" | "settings" | "rules" | "style";
export const libraryRoles: readonly LibraryRole[] = ["schema", "settings", "rules", "style"];
export interface LibraryProfile { role: LibraryRole; basis: string; source: string; installed: boolean; enabled?: boolean; controls?: NativeStyleControl[]; warnings?: ErrorDescription[]; }
export function decodeLibraryProfile(value: unknown): LibraryProfile {
    const item = record(value), style = item.role === "style";
    keys(item, ["role", "basis", "source", "installed", ...(style ? ["enabled", "controls", "warnings"] : [])]);
    if (!libraryRoles.includes(item.role as LibraryRole)) invalidStyleWire();
    return { role: item.role as LibraryRole, basis: text(item.basis, true), source: text(item.source), installed: bool(item.installed), ...(style ? { enabled: bool(item.enabled), controls: array(item.controls, decodeNativeStyleControl), warnings: [...(decodeErrorDescriptions(item.warnings) ?? invalidStyleWire())] } : {}) };
}
export interface LibraryCatalog { roles: { role: LibraryRole; title: UiMessage; read: boolean; write: boolean }[]; styleUse: boolean; reorderNodes: boolean; orderParents: { reference: string; title: string }[]; }
export function decodeLibraryCatalog(value: unknown): LibraryCatalog {
    const item = keys(value, ["roles", "styleUse", "reorderNodes", "orderParents"]);
    const roles = array(item.roles, value => { const role = keys(value, ["role", "title", "read", "write"]); if (!libraryRoles.includes(role.role as LibraryRole)) invalidStyleWire(); return { role: role.role as LibraryRole, title: uiMessage(role.title), read: bool(role.read), write: bool(role.write) }; });
    if (new Set(roles.map(role => role.role)).size !== roles.length) invalidStyleWire();
    return { roles, styleUse: bool(item.styleUse), reorderNodes: bool(item.reorderNodes), orderParents: array(item.orderParents, value => { const parent = keys(value, ["reference", "title"]); return { reference: text(parent.reference, true), title: text(parent.title) }; }) };
}
export interface LibraryPreview { beforeSource?: string; afterSource?: string; summary?: UiMessage; enabled?: boolean; }
export interface LibraryReceipt { role: LibraryRole; operationId: string; status: string; planId: string | null; preview: LibraryPreview; commitAllowed: boolean; }
export function decodeLibraryReceipt(value: unknown): LibraryReceipt {
    const item = keys(value, ["role", "operationId", "status", "planId", "preview", "commitAllowed"]);
    if (!libraryRoles.includes(item.role as LibraryRole) || typeof item.status !== "string" || !["prepared", "committed", "cancelled", "resolved", "recoveryPending", "notRegistered", "notCommitted"].includes(item.status)) invalidStyleWire();
    const source = keys(item.preview, [], ["beforeSource", "afterSource", "summary", "enabled"]), preview: LibraryPreview = {};
    for (const key of ["beforeSource", "afterSource"] as const) if (Object.hasOwn(source, key)) preview[key] = text(source[key]);
    if (Object.hasOwn(source, "summary")) preview.summary = uiMessage(source.summary);
    if (Object.hasOwn(source, "enabled")) preview.enabled = bool(source.enabled);
    if (Object.keys(source).length && (preview.beforeSource === undefined || preview.afterSource === undefined || !preview.summary)) invalidStyleWire();
    const result = { role: item.role as LibraryRole, operationId: text(item.operationId, true), status: text(item.status), planId: nullableText(item.planId), preview, commitAllowed: bool(item.commitAllowed) };
    if (result.commitAllowed && (result.status !== "prepared" || !result.planId || preview.beforeSource === undefined || preview.afterSource === undefined || !preview.summary)) invalidStyleWire();
    if (!result.commitAllowed && result.planId !== null) invalidStyleWire();
    if (result.status === "notRegistered" && Object.keys(preview).length) invalidStyleWire();
    return result;
}

export interface ProcessingRead { baseId: string; roleRef: string; beforeSource: string | null; revision: string | null; processing: unknown; warning: ApiErrorBody | null; source: "shared-library-suggestion"; }
export type ProcessingDocumentDecoder = (value: unknown) => ProcessingDocument;
export function decodeProcessingRead(value: unknown, decodeDocument: ProcessingDocumentDecoder): ProcessingRead {
    const item = keys(value, ["baseId", "roleRef", "beforeSource", "revision", "processing", "warning", "source"]);
    if (item.source !== "shared-library-suggestion") invalidStyleWire();
    const warning = item.warning === null ? null : decodeApiErrorBody(item.warning) ?? invalidStyleWire();
    if (item.processing === undefined) invalidStyleWire();
    const processing = warning ? structuredClone(item.processing) : item.processing === null ? null : decodeDocument(item.processing);
    return { baseId: text(item.baseId, true), roleRef: text(item.roleRef), beforeSource: nullableText(item.beforeSource), revision: nullableText(item.revision), processing, warning, source: "shared-library-suggestion" };
}
export interface ProcessingPreview { beforeSource: string | null; afterSource: string; summary: UiMessage; }
export interface ProcessingOperation { operationId: string; planId: string | null; status: string; resource?: "library.processing"; preview?: ProcessingPreview | null; intentKind?: "updateFiles" | "unknown"; resourceRefs?: string[]; nodeRefs?: { workspaceId: string; nodeId: string }[]; navigationRef?: string; }
export function decodeProcessingOperation(value: unknown): ProcessingOperation {
    const item = record(value);
    if (!Object.hasOwn(item, "resource")) {
        keys(item, ["operationId", "planId", "status"]);
        if (item.status !== "notRegistered" || item.planId !== null) invalidStyleWire();
        return { operationId: text(item.operationId, true), planId: null, status: "notRegistered" };
    }
    keys(item, ["operationId", "resource", "status", "planId", "preview", "intentKind", "resourceRefs", "nodeRefs"], ["navigationRef"]);
    if (item.resource !== "library.processing" || typeof item.status !== "string" || !["prepared", "committed", "recoveryPending", "notCommitted", "cancelled", "resolved"].includes(item.status) || typeof item.intentKind !== "string" || !["updateFiles", "unknown"].includes(item.intentKind)) invalidStyleWire();
    let preview: ProcessingPreview | null = null;
    if (item.preview !== null) { const source = keys(item.preview, ["beforeSource", "afterSource", "summary"]); preview = { beforeSource: nullableText(source.beforeSource), afterSource: text(source.afterSource), summary: uiMessage(source.summary) }; }
    const result: ProcessingOperation = { operationId: text(item.operationId, true), resource: "library.processing", status: text(item.status), planId: nullableText(item.planId), preview, intentKind: item.intentKind as "updateFiles" | "unknown", resourceRefs: array(item.resourceRefs, value => text(value)), nodeRefs: array(item.nodeRefs, value => { const node = keys(value, ["workspaceId", "nodeId"]); return { workspaceId: text(node.workspaceId), nodeId: text(node.nodeId) }; }), ...(Object.hasOwn(item, "navigationRef") ? { navigationRef: text(item.navigationRef) } : {}) };
    if (["prepared", "committed", "recoveryPending"].includes(result.status) && (!result.planId || !preview || result.intentKind !== "updateFiles")) invalidStyleWire();
    if (result.intentKind === "unknown" && (result.status !== "cancelled" || result.planId !== null || preview !== null || result.resourceRefs!.length || result.nodeRefs!.length || result.navigationRef !== undefined)) invalidStyleWire();
    return result;
}
