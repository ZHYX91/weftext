import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody } from "./api-error";
import { copyOriginalDeliveryPresentation, originalDeliveryErrors, type OriginalFact } from "./original-operation";
import { decodeCoreDescription, decodeDescriptionFields, decodeEngineDiagnostic, decodeErrorDescriptions, descriptionUiText, exactKeys, type CoreDescription, type EngineDiagnostic, type ErrorDescription } from "./ui-diagnostics";
import type { RenderNode } from "./types";
/** Presentation state only. Core owns discovery, identities and write qualification. */
export interface KnowledgeNodeRef {
    workspaceId: string;
    nodeId: string;
    title?: string;
    path?: string;
}
export interface KnowledgeMention {
    candidates?: {
        target: KnowledgeNodeRef;
        fragment?: string | null;
        term: string;
        provenance: string;
    }[];
    startScalar: number;
    endScalar: number;
    startUtf16: number;
    endUtf16: number;
    targets: KnowledgeNodeRef[];
    terms: string[];
    writable: boolean;
    lane?: 0 | 1;
    component: number;
}
export const nodeIdentity = (ref: KnowledgeNodeRef): string => `${ref.workspaceId}/${ref.nodeId}`;
export interface KnowledgeLoss extends ErrorDescription {
    path: string;
    code: string;
    productDiagnostics: readonly ErrorDescription[];
}
const invalidKnowledgeResponse = (): never => { throw new UiError(uiMessage("error.invalidResponse")); };
function knowledgeRecord(value: unknown): Record<string, unknown> {
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalidKnowledgeResponse();
    return value as Record<string, unknown>;
}
export function decodeKnowledgeLoss(value: unknown): KnowledgeLoss {
    const record = knowledgeRecord(value), description = decodeDescriptionFields(record), productDiagnostics = decodeErrorDescriptions(record.productDiagnostics);
    if (!exactKeys(record, ["path", "code", "explanation", "diagnostics", "causes", "productDiagnostics"]) || typeof record.path !== "string" || typeof record.code !== "string" || !description || !productDiagnostics) return invalidKnowledgeResponse();
    return { path: record.path, code: record.code, ...description, productDiagnostics };
}
export function knowledgeLossMessage(value: KnowledgeLoss): UiText {
    const loss = decodeKnowledgeLoss(value);
    return () => `${loss.path ? `${loss.path}\n` : ""}${uiTextValue(descriptionUiText(loss))}${loss.productDiagnostics.map(item => `\n${uiTextValue(descriptionUiText(item))}`).join("")}`;
}
export interface KnowledgeReadError { path: string | null; error: ApiErrorBody }
export interface KnowledgeDiagnosticFields {
    warnings: readonly CoreDescription[];
    errors: readonly KnowledgeReadError[];
    annotationDiagnostics: readonly ApiErrorBody[];
    propertyReferences: { coverage: "complete" | "partial"; items: unknown[]; diagnostics: readonly ErrorDescription[] };
}
/** Decode only declared product diagnostic slots; graph/relationship/author data is never guessed as prose. */
export function decodeKnowledgeDiagnosticFields(value: unknown): Record<string, unknown> & KnowledgeDiagnosticFields {
    const record = knowledgeRecord(value);
    if (!exactKeys(record, ["graph", "headingStructure", "links", "diagnostics", "renderedLinks", "propertyReferences", "citations", "mentions", "annotations", "annotationStatus", "annotationDiagnostics", "templates", "trash", "warnings", "errors"]) || !Array.isArray(record.warnings) || !Array.isArray(record.errors) || !Array.isArray(record.annotationDiagnostics)) return invalidKnowledgeResponse();
    const warnings = record.warnings.map(value => decodeCoreDescription(value) ?? invalidKnowledgeResponse());
    const errors = record.errors.map(value => {
        const entry = knowledgeRecord(value), error = decodeApiErrorBody(entry.error);
        if (!exactKeys(entry, ["path", "error"]) || entry.path !== null && typeof entry.path !== "string" || !error) return invalidKnowledgeResponse();
        return { path: entry.path, error };
    });
    const annotationDiagnostics = record.annotationDiagnostics.map(value => decodeApiErrorBody(value) ?? invalidKnowledgeResponse());
    const properties = knowledgeRecord(record.propertyReferences), diagnostics = decodeErrorDescriptions(properties.diagnostics);
    if (!exactKeys(properties, ["coverage", "items", "diagnostics"]) || properties.coverage !== "complete" && properties.coverage !== "partial" || !Array.isArray(properties.items) || !diagnostics) return invalidKnowledgeResponse();
    return { ...record, warnings, errors, annotationDiagnostics, propertyReferences: { coverage: properties.coverage, items: properties.items, diagnostics } };
}
export interface KnowledgeTemplatePreview {
    source: string;
    tree: RenderNode | null;
    diagnostics: readonly EngineDiagnostic[];
    ok: boolean;
    previewBasis: "registeredSource";
    sourceNode: KnowledgeNodeRef | null;
}
export function decodeKnowledgeTemplatePreview(value: unknown): KnowledgeTemplatePreview {
    const record = knowledgeRecord(value);
    if (!exactKeys(record, ["source", "tree", "diagnostics", "ok", "previewBasis", "sourceNode"]) || typeof record.source !== "string" || typeof record.ok !== "boolean" || record.previewBasis !== "registeredSource" || !Array.isArray(record.diagnostics) || record.tree !== null && (!record.tree || typeof record.tree !== "object" || Array.isArray(record.tree))) return invalidKnowledgeResponse();
    let sourceNode: KnowledgeNodeRef | null = null;
    if (record.sourceNode !== null) {
        const node = knowledgeRecord(record.sourceNode);
        if (!exactKeys(node, ["workspaceId", "nodeId"]) || typeof node.workspaceId !== "string" || typeof node.nodeId !== "string") return invalidKnowledgeResponse();
        sourceNode = { workspaceId: node.workspaceId, nodeId: node.nodeId };
    }
    return { source: record.source, tree: record.tree as RenderNode | null, diagnostics: record.diagnostics.map(value => decodeEngineDiagnostic(value) ?? invalidKnowledgeResponse()), ok: record.ok, previewBasis: "registeredSource", sourceNode };
}
/** Delivery errors describe a committed operation separately; they do not change its journal status. */
export function decodeKnowledgeDelivery<T extends { status: string }>(value: T): T {
    const record = knowledgeRecord(value);
    if (typeof record.status !== "string") return invalidKnowledgeResponse();
    const result = { ...record };
    if (Object.hasOwn(record, "deliveryError")) result.deliveryError = decodeApiErrorBody(record.deliveryError) ?? invalidKnowledgeResponse();
    if (Object.hasOwn(record, "documentError")) result.documentError = decodeApiErrorBody(record.documentError) ?? invalidKnowledgeResponse();
    if (Object.hasOwn(record, "affectedDocuments")) {
        if (!Array.isArray(record.affectedDocuments)) return invalidKnowledgeResponse();
        result.affectedDocuments = record.affectedDocuments.map(value => {
            const document = knowledgeRecord(value);
            return { ...document, ...(Object.hasOwn(document, "documentError") ? { documentError: decodeApiErrorBody(document.documentError) ?? invalidKnowledgeResponse() } : {}) };
        });
    }
    return copyOriginalDeliveryPresentation(value, result) as T;
}
export function knowledgeDeliveryErrors(value: Pick<OriginalFact, "deliveryError" | "documentError"> & { affectedDocuments?: readonly { documentError?: ApiErrorBody }[] }): readonly UiText[] {
    return [...originalDeliveryErrors(value), ...(value.affectedDocuments ?? []).flatMap(document => document.documentError ? [apiErrorUiText(document.documentError)] : [])];
}
export function uncertainKnowledgeOperation(status: string): boolean {
    return ["unknown", "pending", "resultUnknown", "in-progress", "pending_reconciliation", "recoveryPending"].includes(status);
}
/** Draft edits have no server operation to reconcile, even if local persistence fails. */
export function retainPendingKnowledgePlan(draftOnly: boolean, error: unknown): boolean {
    if (draftOnly)
        return false;
    return true;
}
/** Same-span targets merge; nested and crossing spans survive. No overlap graph. */
export function layoutMentions(hits: readonly KnowledgeMention[]): KnowledgeMention[] {
    const grouped = new Map<string, KnowledgeMention>();
    for (const hit of hits) {
        if (!Number.isInteger(hit.startScalar) || !Number.isInteger(hit.endScalar) || hit.startScalar < 0 || hit.endScalar <= hit.startScalar)
            continue;
        const key = `${hit.startScalar}:${hit.endScalar}`;
        const previous = grouped.get(key);
        if (!previous)
            grouped.set(key, { ...hit, targets: [...hit.targets], terms: [...hit.terms], lane: undefined });
        else {
            previous.targets.push(...hit.targets);
            previous.terms.push(...hit.terms);
            previous.writable = previous.writable && hit.writable && previous.startUtf16 === hit.startUtf16 && previous.endUtf16 === hit.endUtf16;
        }
    }
    const sorted = [...grouped.values()].sort((a, b) => a.startScalar - b.startScalar || b.endScalar - a.endScalar);
    let component = -1, componentEnd = -1;
    const ends = [-1, -1];
    for (const hit of sorted) {
        if (hit.startScalar >= componentEnd) {
            component++;
            ends.fill(-1);
        }
        componentEnd = Math.max(componentEnd, hit.endScalar);
        hit.component = component;
        const lane = ends.findIndex(end => end <= hit.startScalar);
        if (lane >= 0) {
            hit.lane = lane as 0 | 1;
            ends[lane] = hit.endScalar;
        }
        const targets = new Map(hit.targets.map(ref => [nodeIdentity(ref), ref]));
        hit.targets = [...targets.values()].sort((a, b) => nodeIdentity(a).localeCompare(nodeIdentity(b), "en"));
        hit.terms = [...new Set(hit.terms)];
    }
    return sorted;
}
export interface KnowledgeBasis {
    inputId?: string;
    sessionId: string;
    path: string;
    source: string;
    serial: number;
    baseId: string;
}
export function sameKnowledgeBasis(a: KnowledgeBasis | null, b: KnowledgeBasis | null): boolean {
    return a !== null && b !== null && a.sessionId === b.sessionId && a.path === b.path && a.serial === b.serial && a.inputId === b.inputId && a.baseId === b.baseId && a.source === b.source;
}
/** A late background read must never replace a newer document or draft. */
export class KnowledgeReadGate {
    private generation = 0;
    begin(): number { return ++this.generation; }
    accepts(generation: number, captured: KnowledgeBasis, current: KnowledgeBasis | null): boolean {
        return generation === this.generation && sameKnowledgeBasis(captured, current);
    }
    invalidate(): void { this.generation++; }
}
export type AnnotationResolution = "exact" | "mapped" | "stale" | "ambiguous" | "orphaned" | "unavailable";
export function canApplySuggestion(state: {
    purpose?: string;
    suggestionState?: string;
    reviewState?: string;
    targetResolution?: AnnotationResolution;
    verified?: boolean;
}): boolean {
    return state.purpose === "suggestion" && state.suggestionState === "pending" && state.verified !== false && (state.targetResolution === "exact" || state.targetResolution === "mapped");
}
export function isGraphemeRange(text: string, start: number, end: number, allowPoint = false): boolean {
    if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start || !allowPoint && end === start || end > text.length)
        return false;
    const boundaries = new Set<number>([text.length]);
    for (const part of new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text))
        boundaries.add(part.index);
    return boundaries.has(start) && boundaries.has(end);
}
export interface MediaRegionInput {
    kind: "pdf_region" | "image_region" | "media_interval";
    resource: string;
    revision: string;
    pageIndex?: number;
    rotation?: number;
    width?: number;
    height?: number;
    x?: number;
    y?: number;
    rectWidth?: number;
    rectHeight?: number;
    startTick?: number;
    endTick?: number;
    timebase?: number;
    timebaseDenominator?: number;
    durationTick?: number;
    mediaType?: "audio" | "video";
}
/** Validate entered units without rounding, normalizing or fetching the resource. */
export function validateMediaRegion(value: MediaRegionInput): UiText | null {
    if (!value.resource.trim() || !value.revision.trim())
        return uiMessage("mediaTargetState.exactResourceRequired");
    if (value.kind === "media_interval") {
        if (![value.startTick, value.endTick, value.timebase].every(n => typeof n === "number" && Number.isSafeInteger(n)))
            return uiMessage("mediaTargetState.exactClockIntegersRequired");
        if (value.startTick! < 0 || value.endTick! <= value.startTick! || value.timebase! <= 0)
            return uiMessage("mediaTargetState.clockRangeInvalid");
        if (value.timebaseDenominator !== undefined && (!Number.isSafeInteger(value.timebaseDenominator) || value.timebaseDenominator <= 0))
            return uiMessage("mediaTargetState.timeDenominatorInvalid");
        if (value.durationTick !== undefined && (!Number.isSafeInteger(value.durationTick) || value.durationTick < value.endTick!))
            return uiMessage("mediaTargetState.intervalOutsideDuration");
        if (value.mediaType === "audio")
            return null;
    }
    if (value.kind === "pdf_region" && (!Number.isSafeInteger(value.pageIndex) || value.pageIndex! < 0))
        return uiMessage("mediaTargetState.pdfIndexInvalid");
    if (![value.width, value.height, value.x, value.y, value.rectWidth, value.rectHeight].every(n => typeof n === "number" && Number.isFinite(n)))
        return uiMessage("mediaTargetState.dimensionsRectangleRequired");
    if (value.kind !== "pdf_region" && ![value.width, value.height, value.x, value.y, value.rectWidth, value.rectHeight].every(Number.isSafeInteger))
        return uiMessage("mediaTargetState.rectangleIntegerPixelsRequired");
    if (value.width! <= 0 || value.height! <= 0 || value.x! < 0 || value.y! < 0 || value.rectWidth! <= 0 || value.rectHeight! <= 0 || value.x! + value.rectWidth! > value.width! || value.y! + value.rectHeight! > value.height!)
        return uiMessage("mediaTargetState.rectangleOutsideDimensions");
    return null;
}
