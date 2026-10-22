import { exactKeys } from "./ui-diagnostics";
import { isContentLocale } from "./content-language";
import { message, parseUiMessage, UiError, type UiLocale, type UiMessage } from "./ui-locale";

export interface ReviewPreview {
    summary?: UiMessage; kind?: string; body?: string; replacement?: string; count?: number;
    annotationId?: string; annotationLocale?: UiLocale; source?: { before: string; after: string };
}
export interface ReviewReceipt {
    operationId: string; resource: string; status: "notRegistered" | "notCommitted" | "prepared" | "committed" | "recoveryPending" | "resolved" | "cancelled";
    planId: string | null; preview: ReviewPreview; commitAllowed: boolean;
}
function invalid(): never { throw new UiError(message("error.invalidResponse")); }
function record(value: unknown): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) return invalid(); return value as Record<string, unknown>; }
function text(value: unknown): string { if (typeof value !== "string") return invalid(); return value; }
/** Decode only the server's scope-safe preview fields, including frozen content-language intent. */
export function decodeReviewPreview(value: unknown): ReviewPreview {
    const item = record(value);
    if (!exactKeys(item, [], ["summary", "kind", "body", "replacement", "count", "annotationId", "annotationLocale", "source"])) return invalid();
    const result: ReviewPreview = {};
    for (const key of ["kind", "body", "replacement", "annotationId"] as const) if (Object.hasOwn(item, key)) result[key] = text(item[key]);
    if (Object.hasOwn(item, "summary")) result.summary = parseUiMessage(item.summary) ?? invalid();
    if (Object.hasOwn(item, "count")) { if (typeof item.count !== "number" || !Number.isSafeInteger(item.count) || item.count < 0) return invalid(); result.count = item.count; }
    if (Object.hasOwn(item, "annotationLocale")) { if (!isContentLocale(item.annotationLocale)) return invalid(); result.annotationLocale = item.annotationLocale; }
    if (Object.hasOwn(item, "source")) { const source = record(item.source); if (!exactKeys(source, ["before", "after"])) return invalid(); result.source = { before: text(source.before), after: text(source.after) }; }
    return result;
}
export function decodeReviewReceipt(value: unknown, resource: string, operationId: string): ReviewReceipt {
    const item = record(value);
    if (!exactKeys(item, ["operationId", "resource", "status", "planId", "preview", "commitAllowed"]) || item.operationId !== operationId || item.resource !== resource || !["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"].includes(String(item.status)) || typeof item.commitAllowed !== "boolean" || item.planId !== null && (typeof item.planId !== "string" || !item.planId) || item.commitAllowed && (item.status !== "prepared" || !item.planId)) return invalid();
    return { operationId, resource, status: item.status as ReviewReceipt["status"], planId: item.planId as string | null, preview: decodeReviewPreview(item.preview), commitAllowed: item.commitAllowed };
}
