import { message as uiMessage, type UiText } from "./ui-locale";
import type { DraftInput, OpenedDocument, RenderOutput } from "./types";

/** Presentation may collapse only the same known complete input. Unknown
 * provenance and independently identical author text remain separate. */
export function uniqueRetainedDrafts(inputs: readonly DraftInput[]): DraftInput[] {
    const out: DraftInput[] = [];
    for (const input of inputs) {
        if (!out.some(other => other === input || input.producerId !== undefined && other.producerId === input.producerId
            && other.path === input.path && other.baseSource === input.baseSource && other.source === input.source && other.baseKnown === input.baseKnown
            && other.role?.kind === input.role?.kind
            && (input.role?.kind !== "committedBaseline" || other.role?.kind === "committedBaseline" && other.role.operationId === input.role.operationId))) out.push(input);
    }
    return out;
}
/** A qualified same-Node move does not authorize silently rebinding a Draft's
 * writer. Consumers wait for the user's existing comparison/merge flow. */
export function sourceConsumerNeedsComparison(opened: Pick<OpenedDocument, "path" | "nodeId" | "workspaceId">, target?: Pick<OpenedDocument, "path" | "nodeId" | "workspaceId">): boolean {
    const coordinate = (path: string): string => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
    return !!target && !!opened.nodeId && !!opened.workspaceId && target.nodeId === opened.nodeId && target.workspaceId === opened.workspaceId && coordinate(target.path) !== coordinate(opened.path);
}
export const sourceMoveComparisonMessage = uiMessage("sessionState.syncDiskAndLocationChanged");
/** Draft, save base, and projection freshness stay independent. */
export class DocumentSession {
    source: string;
    baseSource: string;
    baseId: string;
    path: string;
    serial = 0;
    projection: RenderOutput | null = null;
    savePending = false;
    lastDraftSource: string | null = null;
    lastDraftSerial = -1;
    lastDraftRevision: number | null = null;
    private nextDraftRevision = 0;
    issueDraftRevision(): number { return ++this.nextDraftRevision; }
    error: UiText | null = null;
    recovered: boolean;
    /** Only the recovery primary actually installed in this editor may be
     * explicitly continued. Later observations never adopt another producer. */
    readonly recoveryContinuation: { sessionId: string; recordWitness: string; recoveredSource: string } | null;
    recoveryContinuationSettled = false;
    constructor(public opened: OpenedDocument) {
        this.path = opened.path;
        this.baseId = opened.baseId;
        this.baseSource = opened.source;
        this.source = opened.recoveredDraft ?? opened.source;
        this.recovered = opened.recoveredDraft != null && opened.recoveredDraft !== opened.source;
        this.recoveryContinuation = opened.recoveredDraft != null && opened.recoveredDraftWitness
            ? { sessionId: opened.sessionId, recordWitness: opened.recoveredDraftWitness, recoveredSource: opened.recoveredDraft } : null;
    }
    get dirty(): boolean { return this.source !== this.baseSource; }
    get projectionCurrent(): boolean { return this.projection?.serial === this.serial && this.projection.source === this.source; }
    edit(source: string): void {
        if (source !== this.source) {
            this.source = source;
            this.serial++;
            this.error = null;
        }
    }
    acceptProjection(result: RenderOutput): boolean {
        if (result.serial !== this.serial || result.source !== this.source)
            return false;
        this.projection = result;
        return true;
    }
    acknowledgeSave(savedSource: string, baseId: string, path = this.path): void {
        this.baseSource = savedSource;
        this.baseId = baseId;
        this.path = path;
        this.recovered = false;
        this.error = null;
    }
    acknowledgeDraft(source: string, serial: number, revision: number): void {
        if (serial < this.lastDraftSerial) return;
        this.lastDraftSerial = serial;
        this.lastDraftSource = source;
        this.lastDraftRevision = revision;
    }
}
/** Install only the same original owner's verified saved bytes. A fresh
 * Session has its own actual base; the original receipt base cannot be reused. */
export function rebindCommittedSave(session: DocumentSession, frozen: { sessionId: string; source: string }, document: OpenedDocument): boolean {
  const prior = session.opened;
  const sameOwner = prior.nodeId && prior.workspaceId
    ? document.nodeId === prior.nodeId && document.workspaceId === prior.workspaceId
    : !document.nodeId && (!prior.workspaceId || document.workspaceId === prior.workspaceId) && document.path.replaceAll("\\", "/").toLowerCase() === session.path.replaceAll("\\", "/").toLowerCase();
  if (prior.sessionId !== frozen.sessionId || !sameOwner || document.source !== frozen.source || document.recoveredDraftConflict) return false;
  session.opened = document;
  session.acknowledgeSave(frozen.source, document.baseId, document.path);
  return true;
}
/** Conservative native literal escaping; source-mode paste remains exact. */
export function encodeLiteral(text: string): string {
    return text.replace(/[\\#\[\]{}$*_<>@`=+\-/:~"']/g, "\\$&");
}
export function validLiteralRange(source: string, from: number, to: number, raw: string): boolean {
    return Number.isInteger(from) && Number.isInteger(to) && from >= 0 && to >= from && to <= source.length && source.slice(from, to) === raw;
}
export interface PendingLiteralProposal {
    source: string;
    from: number;
    to: number;
    raw: string;
    text: string;
    nodeKey: string;
}
export function recoverySource(source: string, pending?: PendingLiteralProposal | null): string {
    if (!pending || pending.source !== source || !validLiteralRange(source, pending.from, pending.to, pending.raw))
        return source;
    const escaped = encodeLiteral(pending.text);
    const replacement = pending.nodeKey === "@body" ? `${pending.from > 0 ? "\n\n" : ""}${escaped}\n` : escaped;
    return source.slice(0, pending.from) + replacement + source.slice(pending.to);
}
export function canRebindSaveAs(savedSource: string, opened: OpenedDocument): boolean {
    return opened.source === savedSource && !opened.recoveredDraftConflict;
}
