import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
import { utf8Offset } from "./writing";
import type { QueryPlan } from "./query-panel";
export interface DefinitionAuthorContext {
    inputId?: string;
    resource: string;
    sessionId: string;
    baseId: string;
    source: string;
    serial: number;
    writable: boolean;
    pending: boolean;
    compare: boolean;
    definitionReservations?: readonly string[];
}
export interface DefinitionPrepareRequest {
    resource: string;
    sessionId: string;
    baseId: string;
    source: string;
    serial: number;
    sourceHash: string;
    operationId: string;
    label: string;
    plan: QueryPlan;
}
export interface DefinitionProposal {
    definition: {
        key: string;
        label: string;
        binding: unknown;
        plan: QueryPlan;
    };
    source: string;
    basisSource?: string;
    sourceHash: string;
    serial?: number;
    patchUnit: "utf16";
    patches: {
        from: number;
        to: number;
        text: string;
        expected?: string;
        byteFrom?: number;
        byteTo?: number;
    }[];
    draftOnly: true;
    canApply: boolean;
    recoveryOnly: boolean;
}
export interface DefinitionReceipt {
    operationId: string;
    resource: string;
    status: "reserved" | "admitted" | "cancelled" | "recoveryPending" | "notRegistered";
    key?: string;
    binding?: unknown;
    saveOperationId?: string;
    proposal?: DefinitionProposal;
    proposalUnavailable?: boolean;
}
export type DefinitionPrepared = DefinitionReceipt & DefinitionProposal;
export interface DefinitionRecord {
    inputId?: string;
    operationId: string;
    resource: string;
    request?: DefinitionPrepareRequest;
    receipt?: DefinitionReceipt;
    applied?: boolean;
}
export function sameDefinitionBasis(context: DefinitionAuthorContext | null, request: DefinitionPrepareRequest, inputId?: string): boolean {
    return !!context && context.resource === request.resource && context.sessionId === request.sessionId && context.baseId === request.baseId && context.source === request.source && context.serial === request.serial && context.inputId === inputId && context.writable && !context.pending && !context.compare;
}
export function validateDefinitionPatches(source: string, proposal: DefinitionProposal): SourcePatch[] {
    if (proposal.patchUnit !== "utf16" || proposal.draftOnly !== true || !Array.isArray(proposal.patches) || typeof proposal.source !== "string" || !proposal.definition?.key)
        throw new UiError(uiMessage("hostedDefinitionState.exactDraftPatchMissing"));
    const patches = proposal.patches.map(patch => {
        if (!Number.isSafeInteger(patch.from) || !Number.isSafeInteger(patch.to) || patch.from < 0 || patch.to < patch.from || patch.to > source.length || !sourceBoundary(source, patch.from) || !sourceBoundary(source, patch.to) || typeof patch.text !== "string" || patch.expected !== undefined && patch.expected !== source.slice(patch.from, patch.to) || patch.byteFrom !== undefined && patch.byteFrom !== utf8Offset(source, patch.from) || patch.byteTo !== undefined && patch.byteTo !== utf8Offset(source, patch.to))
            throw new UiError(uiMessage("hostedDefinitionState.originalOrUtf16Mismatch"));
        return { from: patch.from, to: patch.to, insert: patch.text, expected: source.slice(patch.from, patch.to) };
    });
    if (applySourcePatches(source, patches) !== proposal.source)
        throw new UiError(uiMessage("hostedDefinitionState.patchProposalMismatch"));
    return patches;
}
export function validatePreparedDefinition(request: DefinitionPrepareRequest, incoming: DefinitionPrepared): DefinitionProposal {
    if (incoming.operationId !== request.operationId || incoming.resource !== request.resource || incoming.status !== "reserved" || incoming.key !== incoming.definition?.key || incoming.definition?.label !== request.label || incoming.sourceHash !== request.sourceHash || incoming.serial !== request.serial)
        throw new UiError(uiMessage("hostedDefinitionState.receiptIdOrDraftMismatch"));
    validateDefinitionPatches(request.source, incoming);
    return { definition: incoming.definition, source: incoming.source, basisSource: request.source, sourceHash: incoming.sourceHash, serial: incoming.serial, patchUnit: incoming.patchUnit, patches: incoming.patches, draftOnly: true, canApply: incoming.canApply, recoveryOnly: incoming.recoveryOnly };
}
/** Terminal cancellation never keeps an earlier protected proposal as current delivery. */
export function retainDefinitionReceipt(previous: DefinitionRecord, incoming: DefinitionReceipt): DefinitionRecord {
    if (incoming.operationId !== previous.operationId || incoming.resource !== previous.resource || !["reserved", "admitted", "cancelled", "recoveryPending", "notRegistered"].includes(incoming.status))
        throw new UiError(uiMessage("hostedDefinitionState.originalReceiptMismatch"));
    return { ...previous, receipt: incoming };
}
