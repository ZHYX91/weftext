import { message, UiError } from "./ui-locale";
import { decodeDescriptionFields, decodeErrorDescriptions, exactKeys, type ErrorDescription } from "./ui-diagnostics";
import type { MediaResource, ResourceList } from "./media-viewer";

export interface MediaPage {
    width: number;
    height: number;
    rotation: number;
    extractionId: string | null;
    extractedScalars: number | null;
}
export type ResourceMetadata =
    | { kind: "image"; width: number; height: number }
    | { kind: "pdf"; pages: MediaPage[] }
    | { kind: "audio"; timebase: { numerator: number; denominator: number }; durationTick: number }
    | { kind: "video"; timebase: { numerator: number; denominator: number }; durationTick: number; width: number; height: number }
    | ({ kind: "unsupported" } & ErrorDescription);
export interface LocalMediaResource extends MediaResource { nodeId: string; readBasis: string }
export type ThumbnailResponse = {
    sourceRevision: string | null;
    sourceBaseId: string | null;
    sourceBasisKind: "observed" | "committed";
} & ({ state: "ready"; descriptor: LocalMediaResource; previewBasis: "currentResourceAtRowSource" }
    | ({ state: "absent" | "unsupported" | "unavailable" } & ErrorDescription));

export const invalidMediaResponse = (): never => { throw new UiError(message("error.invalidResponse")); };
export function mediaRecord(value: unknown): Record<string, unknown> {
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalidMediaResponse();
    return value as Record<string, unknown>;
}
export const mediaInteger = (value: unknown, minimum = 0): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= minimum;
const positive = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value) && value > 0;
const nullableString = (value: unknown): value is string | null => value === null || typeof value === "string";

/** Only the ResourceMetadata protocol owns these display descriptions. Author objects do not. */
export function decodeResourceMetadata(value: unknown): ResourceMetadata {
    const record = mediaRecord(value);
    switch (record.kind) {
        case "image":
            if (!exactKeys(record, ["kind", "width", "height"]) || !mediaInteger(record.width, 1) || !mediaInteger(record.height, 1)) return invalidMediaResponse();
            return { kind: "image", width: record.width, height: record.height };
        case "pdf": {
            if (!exactKeys(record, ["kind", "pages"]) || !Array.isArray(record.pages)) return invalidMediaResponse();
            const pages = record.pages.map(value => {
                const page = mediaRecord(value);
                if (!exactKeys(page, ["width", "height", "rotation", "extractionId", "extractedScalars"]) || !positive(page.width) || !positive(page.height) || !mediaInteger(page.rotation) || page.rotation > 65535 || !nullableString(page.extractionId) || page.extractedScalars !== null && !mediaInteger(page.extractedScalars)) return invalidMediaResponse();
                return { width: page.width, height: page.height, rotation: page.rotation, extractionId: page.extractionId, extractedScalars: page.extractedScalars };
            });
            return { kind: "pdf", pages };
        }
        case "audio":
        case "video": {
            const video = record.kind === "video";
            if (!exactKeys(record, video ? ["kind", "timebase", "durationTick", "width", "height"] : ["kind", "timebase", "durationTick"]) || !mediaInteger(record.durationTick)) return invalidMediaResponse();
            const timebase = mediaRecord(record.timebase);
            if (!exactKeys(timebase, ["numerator", "denominator"]) || !mediaInteger(timebase.numerator, 1) || !mediaInteger(timebase.denominator, 1)) return invalidMediaResponse();
            const clock = { numerator: timebase.numerator, denominator: timebase.denominator };
            if (!video) return { kind: "audio", timebase: clock, durationTick: record.durationTick };
            if (!mediaInteger(record.width, 1) || !mediaInteger(record.height, 1)) return invalidMediaResponse();
            return { kind: "video", width: record.width, height: record.height, timebase: clock, durationTick: record.durationTick };
        }
        case "unsupported": {
            if (!exactKeys(record, ["kind", "explanation", "diagnostics", "causes"])) return invalidMediaResponse();
            const description = decodeDescriptionFields(record);
            if (!description) return invalidMediaResponse();
            return { kind: "unsupported", ...description };
        }
        default: return invalidMediaResponse();
    }
}

export function decodeLocalMediaResource(value: unknown): LocalMediaResource {
    const record = mediaRecord(value);
    if (!exactKeys(record, ["resourceId", "resource", "nodeId", "name", "revision", "readBasis", "bytes", "contentType", "metadata", "annotationReady", "currentness"]) || ["resourceId", "resource", "nodeId", "name", "readBasis", "contentType", "currentness"].some(key => typeof record[key] !== "string") || typeof record.resource !== "string" || !record.resource.startsWith("weftext-resource://") || !nullableString(record.revision) || record.bytes !== null && !mediaInteger(record.bytes) || typeof record.annotationReady !== "boolean") return invalidMediaResponse();
    return { resourceId: record.resourceId as string, resource: record.resource, nodeId: record.nodeId as string, name: record.name as string, revision: record.revision, readBasis: record.readBasis as string, bytes: record.bytes, contentType: record.contentType as string, metadata: decodeResourceMetadata(record.metadata), annotationReady: record.annotationReady, currentness: record.currentness as string };
}

export function decodeResourceList(value: unknown): ResourceList {
    const record = mediaRecord(value);
    if (!exactKeys(record, ["resources", "warnings"]) || !Array.isArray(record.resources)) return invalidMediaResponse();
    const warnings = decodeErrorDescriptions(record.warnings);
    if (!warnings) return invalidMediaResponse();
    return { resources: record.resources.map(decodeLocalMediaResource), warnings };
}

/** Building/failed are client rendering states; a server response cannot claim them. */
export function decodeThumbnailResponse(value: unknown): ThumbnailResponse {
    const record = mediaRecord(value);
    const basisKeys = ["state", "sourceRevision", "sourceBaseId", "sourceBasisKind"];
    if (!nullableString(record.sourceRevision) || !nullableString(record.sourceBaseId) || record.sourceBasisKind !== "observed" && record.sourceBasisKind !== "committed") return invalidMediaResponse();
    if (record.sourceBasisKind === "observed" ? !record.sourceBaseId || record.sourceRevision !== null : !record.sourceRevision || record.sourceBaseId !== null) return invalidMediaResponse();
    const basis = { sourceRevision: record.sourceRevision, sourceBaseId: record.sourceBaseId, sourceBasisKind: record.sourceBasisKind as "observed" | "committed" };
    if (record.state === "ready") {
        if (!exactKeys(record, [...basisKeys, "descriptor", "previewBasis"]) || record.previewBasis !== "currentResourceAtRowSource") return invalidMediaResponse();
        const descriptor = decodeLocalMediaResource(record.descriptor);
        if (!descriptor.revision || !["image", "pdf"].includes(descriptor.metadata.kind)) return invalidMediaResponse();
        return { ...basis, state: "ready", descriptor, previewBasis: "currentResourceAtRowSource" };
    }
    if (!["absent", "unsupported", "unavailable"].includes(String(record.state)) || !exactKeys(record, [...basisKeys, "explanation", "diagnostics", "causes"])) return invalidMediaResponse();
    const description = decodeDescriptionFields(record);
    if (!description) return invalidMediaResponse();
    return { ...basis, state: record.state as "absent" | "unsupported" | "unavailable", ...description };
}
