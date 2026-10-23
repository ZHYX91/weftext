import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { exactKeys } from "./ui-diagnostics";
import { decodeLibraryReceipt, libraryRoles, type LibraryProfile, type LibraryReceipt, type LibraryRole, type NativeStyleControl } from "./style-wire";
export { libraryRoles } from "./style-wire";
export type { LibraryProfile, LibraryPreview, LibraryReceipt, LibraryRole } from "./style-wire";
export interface LibraryInput {
    source: string;
    changes: Record<string, string>;
    enabled?: boolean;
}
export interface LibraryPrepare {
    role: LibraryRole;
    basis: string;
    operationId: string;
    source?: string;
    changes?: Record<string, string>;
    enabled?: boolean;
}
export interface LibraryPending {
    request: LibraryPrepare;
    input: LibraryInput;
    revision: number;
    receipt?: LibraryReceipt;
    planId?: string;
    commit?: {
        role: LibraryRole;
        operationId: string;
        planId: string;
    };
}
export interface LibraryDraft extends LibraryInput {
    baseSource: string;
    baseEnabled?: boolean;
    revision: number;
    pending?: LibraryPending;
    lastReceipt?: Pick<LibraryReceipt, "role" | "operationId" | "status">;
}
export const libraryTerminal = (status: string) => ["committed", "cancelled", "resolved"].includes(status);
const failure = (message: UiText) => Object.assign(new UiError(message), { code: "resultUnknown" });
export function libraryReceipt(request: LibraryPrepare, value: unknown, previous?: LibraryReceipt, beforeSource?: string, pinnedPlanId?: string): LibraryReceipt {
    const incoming = decodeLibraryReceipt(value);
    if (incoming.role !== request.role || incoming.operationId !== request.operationId)
        throw failure(uiMessage("hostedLibraryState.receiptRoleOrOperationMismatch"));
    if (previous?.planId && incoming.planId && previous.planId !== incoming.planId || previous?.status === "committed" && incoming.status !== "committed")
        throw failure(uiMessage("hostedLibraryState.originalFactsChanged"));
    if (pinnedPlanId && incoming.planId && incoming.planId !== pinnedPlanId)
        throw failure(uiMessage("hostedLibraryState.originalFactsChanged"));
    const preview = incoming.preview, prior = previous?.preview;
    if (preview.beforeSource !== undefined && beforeSource !== undefined && preview.beforeSource !== beforeSource
        || preview.afterSource !== undefined && request.source !== undefined && preview.afterSource !== request.source
        || prior?.beforeSource !== undefined && preview.beforeSource !== undefined && prior.beforeSource !== preview.beforeSource
        || prior?.afterSource !== undefined && preview.afterSource !== undefined && prior.afterSource !== preview.afterSource
        || prior?.enabled !== undefined && preview.enabled !== undefined && prior.enabled !== preview.enabled
        || request.enabled !== undefined && preview.enabled !== undefined && preview.enabled !== request.enabled)
        throw failure(uiMessage("hostedLibraryState.previewOriginalSourceMismatch"));
    // Never revive a previously readable preview after permission/owner changes.
    return incoming;
}
export function libraryInput(draft: LibraryInput): LibraryInput { return { source: draft.source, changes: { ...draft.changes }, ...(draft.enabled === undefined ? {} : { enabled: draft.enabled }) }; }
export function libraryCanCommit(draft: LibraryDraft): boolean {
    const pending = draft.pending, receipt = pending?.receipt;
    return !!pending && receipt?.status === "prepared" && receipt.commitAllowed === true && !!receipt.planId && (!pending.planId || pending.planId === receipt.planId) && pending.revision === draft.revision && JSON.stringify(pending.input) === JSON.stringify(libraryInput(draft));
}
export function libraryChanges(controls: readonly NativeStyleControl[], raw: LibraryInput["changes"]): Record<string, string> {
    const result: Record<string, string> = {};
    for (const [key, input] of Object.entries(raw)) {
        const control = controls.find(item => item.key === key);
        if (!control?.editable)
            throw new UiError(displayMessage("hostedLibraryState.originalControlNotWritable", { p0: uiDisplayParameter(key) }));
        if (typeof input !== "string")
            throw new UiError(displayMessage("hostedLibraryState.controlRecoveryInvalid", { p0: uiDisplayParameter(key) }));
        result[key] = input;
    }
    return result;
}
export function freezeLibraryPrepare(role: LibraryRole, profile: LibraryProfile, draft: LibraryDraft, operationId: string): LibraryPrepare {
    if (profile.role !== role || !profile.basis || draft.pending || draft.baseSource !== profile.source || draft.baseEnabled !== profile.enabled)
        throw new UiError(uiMessage("hostedLibraryState.basisOrOperationUnverified"));
    const advanced = draft.source !== draft.baseSource;
    if (advanced && Object.keys(draft.changes).length)
        throw new UiError(uiMessage("hostedLibraryState.sourceAndControlsBothChanged"));
    const changes = libraryChanges(profile.controls ?? [], draft.changes);
    if (profile.installed && !advanced && !Object.keys(changes).length && draft.enabled === profile.enabled)
        throw new UiError(uiMessage("hostedLibraryState.noConfigurationChanges"));
    if (role !== "style") {
        try {
            JSON.parse(draft.source);
        }
        catch {
            throw new UiError(uiMessage("hostedLibraryState.jsonIncomplete"));
        }
    }
    return { role, basis: profile.basis, operationId, ...(role !== "style" || advanced || !profile.installed ? { source: draft.source } : Object.keys(changes).length ? { changes } : {}), ...(role === "style" && draft.enabled !== profile.enabled ? { enabled: draft.enabled } : {}) };
}
export function decodeLibraryDraft(raw: string, role: LibraryRole): LibraryDraft {
    const value = JSON.parse(raw) as LibraryDraft;
    if (!value || typeof value.source !== "string" || typeof value.baseSource !== "string" || !Number.isSafeInteger(value.revision) || value.revision < 0 || !value.changes || typeof value.changes !== "object" || Array.isArray(value.changes) || Object.values(value.changes).some(input => typeof input !== "string") || value.enabled !== undefined && typeof value.enabled !== "boolean" || value.baseEnabled !== undefined && typeof value.baseEnabled !== "boolean")
        throw new UiError(uiMessage("hostedLibraryState.recoveryUnparseable"));
    if (value.pending && (value.pending.request?.role !== role || typeof value.pending.request.operationId !== "string" || !value.pending.request.operationId || typeof value.pending.request.basis !== "string" || !value.pending.request.basis || !value.pending.input || !Number.isSafeInteger(value.pending.revision) || value.pending.revision < 0))
        throw new UiError(uiMessage("hostedLibraryState.recoveryRoleMismatch"));
    if (value.pending) {
        const pending = value.pending, request = pending.request;
        if (!exactKeys(request as unknown as Record<string, unknown>, ["role", "basis", "operationId"], ["source", "changes", "enabled"]) || request.source !== undefined && typeof request.source !== "string" || request.enabled !== undefined && typeof request.enabled !== "boolean" || request.changes !== undefined && (!request.changes || typeof request.changes !== "object" || Array.isArray(request.changes) || Object.values(request.changes).some(value => typeof value !== "string")) || typeof pending.input.source !== "string" || !pending.input.changes || typeof pending.input.changes !== "object" || Array.isArray(pending.input.changes) || Object.values(pending.input.changes).some(value => typeof value !== "string") || pending.input.enabled !== undefined && typeof pending.input.enabled !== "boolean" || pending.planId !== undefined && (typeof pending.planId !== "string" || !pending.planId))
            throw new UiError(uiMessage("hostedLibraryState.recoveryRoleMismatch"));
        if (pending.receipt) {
            pending.receipt = libraryReceipt(request, pending.receipt, undefined, value.baseSource, pending.planId);
            if (pending.receipt.planId) pending.planId = pending.receipt.planId;
        }
        if (pending.commit) {
            const commit = pending.commit;
            if (!exactKeys(commit as unknown as Record<string, unknown>, ["role", "operationId", "planId"]) || commit.role !== request.role || commit.operationId !== request.operationId || typeof commit.planId !== "string" || !commit.planId || pending.planId && commit.planId !== pending.planId)
                throw new UiError(uiMessage("hostedLibraryState.recoveryRoleMismatch"));
            pending.planId = commit.planId;
        }
    }
    return value;
}
