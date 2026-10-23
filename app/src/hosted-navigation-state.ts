import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { sourceBoundary } from "./editor";
import type { Origin } from "./types";
import { draftDisposition, type HostedDraft } from "./hosted-state";
export interface NodeFragment {
    label: string;
    status: "found" | "missing" | "ambiguous" | "unavailable";
    basis: "saved" | "draft";
    sourceHash: string;
    origin?: Origin;
}
export interface HostedNodeResolution {
    resource: string;
    reference: string;
    canOpen: boolean;
    fragment?: NodeFragment;
}
export interface HostedNodeBasis {
    sessionId: string;
    source: string;
    serial: number;
    sourceHash: string;
}
export interface HostedNodeFragmentResult {
    resource: string;
    reference: string;
    serial: number;
    sourceHash: string;
    fragment: NodeFragment;
}
export interface HostedNodeAudience {
    epoch: number;
    authorization: number;
    credential: string;
    actor?: string;
    deployment?: string;
    generation?: number;
    sessionId?: string;
    source: string;
    serial: number;
    navigation: number;
}
export function sameHostedNodeAudience(left: HostedNodeAudience, right: HostedNodeAudience): boolean {
    return left.epoch === right.epoch && left.authorization === right.authorization && left.credential === right.credential && left.actor === right.actor && left.deployment === right.deployment && left.generation === right.generation && left.sessionId === right.sessionId && left.source === right.source && left.serial === right.serial && left.navigation === right.navigation;
}
/** The base is only a candidate for a later independently authorized action. */
export function nodeReferenceParts(reference: string): {
    base: string;
    fragment?: string;
} {
    if (!reference.startsWith("weftext-node://"))
        throw new UiError(uiMessage("hostedNavigationState.stableNoteAddressRequired"));
    const at = reference.indexOf("#");
    if (at < 0)
        return { base: reference };
    const base = reference.slice(0, at), encoded = reference.slice(at + 1);
    let fragment: string;
    try {
        fragment = decodeURIComponent(encoded);
    }
    catch {
        throw new UiError(uiMessage("hostedNavigationState.fragmentEncodingInvalid"));
    }
    if (!fragment || encoded.includes("#") || /[\u0000-\u001f\u007f]/.test(fragment))
        throw new UiError(uiMessage("hostedNavigationState.fragmentLabelInvalid"));
    return { base, fragment };
}
export function nodeTargetDraft(saved: string, recovered: string | undefined, recoveredBase: string | undefined, local?: HostedDraft): HostedDraft {
    if (local !== undefined && (!local || typeof local.source !== "string" || typeof local.baseSource !== "string"))
        throw new UiError(uiMessage("hostedNavigationState.targetRecoveryUnreadable"));
    const candidate = local ? structuredClone(local) : { source: recovered ?? saved, baseSource: recoveredBase ?? saved };
    if (candidate.pending)
        throw new UiError(uiMessage("hostedNavigationState.targetSaveUnverified"));
    return draftDisposition(saved, candidate) === "current" ? { ...candidate, source: saved, baseSource: saved } : candidate;
}
export function nodeFragmentOrigin(result: HostedNodeFragmentResult, resource: string, reference: string, basis: HostedNodeBasis): Origin | undefined {
    const expected = nodeReferenceParts(reference);
    if (result.resource !== resource || result.reference !== reference || result.serial !== basis.serial || result.sourceHash !== basis.sourceHash || result.fragment?.sourceHash !== basis.sourceHash || result.fragment?.basis !== "draft" || result.fragment.label !== expected.fragment || !["found", "missing", "ambiguous", "unavailable"].includes(result.fragment.status))
        throw new UiError(uiMessage("hostedNavigationState.fragmentReceiptDraftMismatch"));
    if (result.fragment.status !== "found")
        return;
    const origin = result.fragment.origin;
    if (!origin || origin.isMain !== true || origin.path !== `resource:${resource}` || !Number.isSafeInteger(origin.utf16From) || !Number.isSafeInteger(origin.utf16To) || origin.utf16From < 0 || origin.utf16To <= origin.utf16From || origin.utf16To > basis.source.length || !sourceBoundary(basis.source, origin.utf16From) || !sourceBoundary(basis.source, origin.utf16To) || basis.source.slice(origin.utf16From, origin.utf16To) !== origin.raw)
        throw new UiError(uiMessage("hostedNavigationState.labelMainSourceRangeMissing"));
    return origin;
}
export async function nodeSourceHash(source: string): Promise<string> { return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(byte => byte.toString(16).padStart(2, "0")).join(""); }
