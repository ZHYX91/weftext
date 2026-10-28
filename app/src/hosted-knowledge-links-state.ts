import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
export interface HostedLinksContext {
    inputId: string;
    actor: string;
    deployment: string;
    generation: number;
    credential: string;
    epoch: number;
    authorization: number;
    resource: string;
    sessionId: string;
    source: string;
    serial: number;
    writable: boolean;
    pending: boolean;
    compare: boolean;
}
export type HostedLinksIdentity = Pick<HostedLinksContext, "actor" | "deployment" | "generation" | "credential" | "authorization">;
export function sameHostedLinksIdentity(a: HostedLinksIdentity, b: HostedLinksIdentity | null): boolean { return !!b && a.actor === b.actor && a.deployment === b.deployment && a.generation === b.generation && a.credential === b.credential && a.authorization === b.authorization; }
export interface HostedLinksBasis {
    actor: string;
    actorEpoch: number;
    deployment: string;
    resource: string;
    sessionId: string;
    sourceHash: string;
    serial: number;
    snapshot: string;
}
export interface HostedLinksRange {
    from: number;
    to: number;
    utf16From: number;
    utf16To: number;
    raw: string;
}
export interface HostedLinksPatch {
    range: HostedLinksRange;
    replacement: string;
    after: string;
}
export interface HostedLinksProposal {
    basis: HostedLinksBasis;
    patch: HostedLinksPatch;
    saved: false;
}
export interface HostedLinksNode {
    resource: string;
    title: string;
    nodeRef: {
        workspaceId: string;
        nodeId: string;
    };
    reference: string;
    canOpen: boolean;
}
export interface HostedLinksResolution {
    status: string;
    form?: string;
    fragment?: string;
    target?: HostedLinksNode | null;
    asset?: {
        asset: string;
        title: string;
        reference: string;
    } | null;
    reference?: string | null;
    canOpen: boolean;
}
export interface HostedLinksOccurrence {
    from: number;
    to: number;
    utf16From: number;
    utf16To: number;
    target: string;
    kind: string;
    destinationSlot?: HostedLinksRange | null;
}
export interface HostedLinksEdge {
    source: HostedLinksNode;
    sourceHash?: string;
    occurrence: HostedLinksOccurrence;
    raw: string;
    resolution: HostedLinksResolution;
    editable: boolean;
}
export interface HostedLinksHit {
    candidates?: {
        target: HostedLinksNode;
        fragment?: string | null;
        term: string;
        provenance: string;
    }[];
    startScalar: number;
    endScalar: number;
    startUtf16: number;
    endUtf16: number;
    terms: string[];
    writable: boolean;
    sourceRange?: HostedLinksRange | null;
    lane?: number | null;
    component: number;
    targets: HostedLinksNode[];
}
export interface HostedLinksProfile {
    graph?: import("./relationship-graph-state").GraphSnapshot;
    headingStructure?: import("./heading-structure-state").HeadingSnapshot;
    diagnostics?: {
        coverage: "complete" | "partial";
        scannedNodes: number;
        issues: HostedLinksEdge[];
        propertyIssues?: {
            source: HostedLinksNode;
            fieldId: string;
            itemPath: string[];
            address: string;
            resolution: HostedLinksEdge["resolution"];
        }[];
        nodes: {
            node: HostedLinksNode;
            incoming: number;
            outgoing: number;
            invalidOutgoing: number;
            state: string;
        }[];
    };
    basis: HostedLinksBasis;
    scope: "configuredAuthorizedManagedSources";
    writable: boolean;
    fileConversion: false;
    links: {
        coverage: "complete" | "partial";
        outgoing: HostedLinksEdge[];
        incomingId: HostedLinksEdge[];
        incomingPath: HostedLinksEdge[];
    };
    renderedLinks: {
        compileIdentity: string;
        target: "html";
        occurrenceOrdinal: number;
        destination: string;
        origin: string;
        resolution: HostedLinksResolution;
    }[];
    propertyReferences: {
        scope: "explicitAuthorizedFields";
        coverage: "complete" | "partial";
        items: {
            source: HostedLinksNode;
            fieldId: string;
            itemPath: string[];
            resolution: HostedLinksResolution;
        }[];
    };
    mentions: {
        scope: "nativeTitlesAndAuthorizedNameFields";
        coverage: "complete" | "partial";
        fragments: {
            id: string;
            text: string;
            coverage: "complete" | "partial";
            hits: HostedLinksHit[];
        }[];
    };
}
export function sameHostedLinksContext(a: HostedLinksContext, b: HostedLinksContext | null): boolean {
    return !!b && a.actor === b.actor && a.deployment === b.deployment && a.generation === b.generation && a.credential === b.credential && a.epoch === b.epoch && a.authorization === b.authorization && a.resource === b.resource && a.sessionId === b.sessionId && a.source === b.source && a.serial === b.serial && a.inputId === b.inputId && a.writable === b.writable && a.pending === b.pending && a.compare === b.compare;
}
export function requireHostedLinksBasis(basis: HostedLinksBasis, context: HostedLinksContext, hash: string, snapshot?: string): void {
    if (!basis || basis.actor !== context.actor || basis.actorEpoch !== context.generation || basis.deployment !== context.deployment || basis.resource !== context.resource || basis.sessionId !== context.sessionId || basis.serial !== context.serial || basis.sourceHash !== hash || !/^[0-9a-f]{64}$/.test(basis.snapshot) || snapshot !== undefined && basis.snapshot !== snapshot)
        throw new UiError(uiMessage("hostedLinkState.receiptBasisMismatch"));
}
/** Both coordinate systems must describe the same exact preimage. The server
 * cannot use an after string to replace bytes outside this one native splice. */
export function hostedLinksPatch(result: HostedLinksProposal, context: HostedLinksContext, hash: string, snapshot: string): SourcePatch {
    requireHostedLinksBasis(result.basis, context, hash, snapshot);
    if (!context.writable || context.pending || context.compare || result.saved !== false)
        throw new UiError(uiMessage("hostedLinkState.draftNotEligible"));
    const patch = result.patch, range = patch?.range;
    if (!range || !Number.isSafeInteger(range.from) || !Number.isSafeInteger(range.to) || !Number.isSafeInteger(range.utf16From) || !Number.isSafeInteger(range.utf16To) || range.from < 0 || range.to < range.from || range.utf16From < 0 || range.utf16To < range.utf16From || range.utf16To > context.source.length || typeof range.raw !== "string" || typeof patch.replacement !== "string" || typeof patch.after !== "string" || !sourceBoundary(context.source, range.utf16From) || !sourceBoundary(context.source, range.utf16To) || context.source.slice(range.utf16From, range.utf16To) !== range.raw)
        throw new UiError(uiMessage("hostedLinkState.proposalRangeMissing"));
    const encode = new TextEncoder();
    if (encode.encode(context.source.slice(0, range.utf16From)).length !== range.from || encode.encode(context.source.slice(0, range.utf16To)).length !== range.to)
        throw new UiError(uiMessage("hostedLinkState.proposalEncodingRangeMismatch"));
    const native = { from: range.utf16From, to: range.utf16To, expected: range.raw, insert: patch.replacement };
    if (applySourcePatches(context.source, [native]) !== patch.after)
        throw new UiError(uiMessage("hostedLinkState.proposalChangesOutsideSelection"));
    return native;
}
