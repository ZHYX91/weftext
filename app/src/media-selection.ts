import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Coordinates belong to the decoded, unrotated resource, never CSS pixels. */
export interface MediaPoint {
    x: number;
    y: number;
}
export interface MediaRectangle {
    x: number;
    y: number;
    rectWidth: number;
    rectHeight: number;
}
import type { MediaRegionInput } from "./knowledge-state";
import type { Cell, NativeValue } from "./data-panel";
import { validateMediaRegion } from "./knowledge-state";
export interface ResourceOpenTarget {
    resource: string;
    revision: string;
    pageIndex?: number;
}
/** Only explicit typed stable refs gain resource navigation, never local paths. */
export function resourceCellReferences(cell: Cell): string[] {
    const found = new Set<string>();
    let budget = 512;
    const visit = (value: NativeValue, depth = 0): void => {
        if (!value || --budget < 0 || depth > 16)
            return;
        if (value.kind === "text" && /^weftext-resource:\/\//.test(value.value))
            found.add(value.value);
        else if (value.kind === "array")
            value.value.forEach(child => visit(child, depth + 1));
        else if (value.kind === "record")
            value.value.forEach(child => visit(child.value, depth + 1));
    };
    if (cell.state === "value" && cell.value)
        visit(cell.value);
    return [...found];
}
/** Resource navigation may only restore an exact matched version. Page indexes
 * arrive normalized by the server; this code never interprets a raw fragment. */
export function resourceOpenError(target: ResourceOpenTarget, resource: {
    resource: string;
    revision: string | null;
    metadata: {
        kind: string;
        pages?: readonly unknown[];
    };
}): UiText | null {
    if (!target.resource || !target.revision || target.resource !== resource.resource || target.revision !== resource.revision)
        return uiMessage("mediaSelection.resourceIdentityChanged");
    if (target.pageIndex !== undefined && (!Number.isSafeInteger(target.pageIndex) || target.pageIndex < 0 || resource.metadata.kind !== "pdf" || !resource.metadata.pages || target.pageIndex >= resource.metadata.pages.length))
        return uiMessage("mediaSelection.exactPdfPageUnavailable");
    return null;
}
/** Flatten an authorized annotation projection for the viewer. A fresh read
 * still checks the exact resource version before restoring these coordinates. */
export function storedMediaSelection(target: unknown): MediaRegionInput | null {
    if (!target || typeof target !== "object")
        return null;
    const record = target as Record<string, unknown>;
    if (typeof record.resource !== "string" || typeof record.revisionId !== "string")
        return null;
    const number = (value: unknown): number | undefined => typeof value === "number" ? value : undefined;
    const rectangle = (value: unknown): Partial<MediaRegionInput> => { const rect = value && typeof value === "object" ? value as Record<string, unknown> : {}; return { x: number(rect.x), y: number(rect.y), rectWidth: number(rect.width), rectHeight: number(rect.height) }; };
    let result: MediaRegionInput;
    if (record.kind === "imageRegion")
        result = { kind: "image_region", resource: record.resource, revision: record.revisionId, width: number(record.pixelWidth), height: number(record.pixelHeight), ...rectangle(record.rect) };
    else if (record.kind === "pdfRegion")
        result = { kind: "pdf_region", resource: record.resource, revision: record.revisionId, width: number(record.pageWidth), height: number(record.pageHeight), pageIndex: number(record.pageIndex), rotation: number(record.rotation), ...rectangle(record.rect) };
    else if (record.kind === "mediaInterval") {
        const timebase = record.timebase && typeof record.timebase === "object" ? record.timebase as Record<string, unknown> : {};
        const region = record.region && typeof record.region === "object" ? record.region as Record<string, unknown> : null;
        result = { kind: "media_interval", resource: record.resource, revision: record.revisionId, timebase: number(timebase.numerator), timebaseDenominator: number(timebase.denominator), durationTick: number(record.durationTick), startTick: number(record.startTick), endTick: number(record.endTick), mediaType: region ? "video" : "audio", ...(region ? { width: number(region.pixelWidth), height: number(region.pixelHeight), ...rectangle(region.rect) } : {}) };
    }
    else
        return null;
    return validateMediaRegion(result) === null ? result : null;
}
export function mediaRectangle(start: MediaPoint, end: MediaPoint, width: number, height: number, pixels: boolean): MediaRectangle | null {
    if (![start.x, start.y, end.x, end.y, width, height].every(Number.isFinite) || width <= 0 || height <= 0)
        return null;
    const clamp = (n: number, limit: number): number => Math.max(0, Math.min(limit, n));
    const lowX = clamp(Math.min(start.x, end.x), width), lowY = clamp(Math.min(start.y, end.y), height);
    const highX = clamp(Math.max(start.x, end.x), width), highY = clamp(Math.max(start.y, end.y), height);
    if (highX <= lowX || highY <= lowY)
        return null;
    const x = pixels ? Math.floor(lowX) : lowX, y = pixels ? Math.floor(lowY) : lowY;
    const right = pixels ? Math.ceil(highX) : highX, bottom = pixels ? Math.ceil(highY) : highY;
    if (right <= x || bottom <= y)
        return null;
    return { x, y, rectWidth: right - x, rectHeight: bottom - y };
}
/** Native player seconds are approximate. Capture explicitly chooses a tick on
 * the verified resource clock; the resulting integer and exact ratio are shown. */
export function playerTick(seconds: number, numerator: number, denominator: number, durationTick: number): number | null {
    if (!Number.isFinite(seconds) || seconds < 0 || ![numerator, denominator, durationTick].every(Number.isSafeInteger) || numerator <= 0 || denominator <= 0 || durationTick <= 0)
        return null;
    const tick = Math.round(seconds * numerator / denominator);
    return Number.isSafeInteger(tick) ? Math.max(0, Math.min(durationTick, tick)) : null;
}
