import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { UiLocale } from "./ui-locale";
import type { DocxImportOptions, OutputFile, OutputLoss, TemplateInspection } from "./export-state";
import { decodeHostedImport, decodeHostedOutput, type FrozenOfficeArtifact, type OfficeImportReceipt, type OfficeOutputReceipt } from "./office-wire";
export type { FrozenOfficeArtifact, OfficeImportReceipt, OfficeOutputReceipt } from "./office-wire";
export type OfficeFormat = "docx" | "doc" | "wps" | "xlsx" | "pdf" | "bibtex" | "csl-json" | "ris" | "native";
export const officeFormats: OfficeFormat[] = ["docx", "doc", "wps", "xlsx", "pdf", "bibtex", "csl-json", "ris", "native"];
export type OfficeImportFormat = "docx" | "doc" | "wps";
export type OfficeBinding = {
    kind: "body" | "title" | "text" | "query" | "field";
    text?: string;
    field?: string;
};
export type OfficeDraft = {
    outputLocale: UiLocale;
    resource: string;
    format: OfficeFormat;
    reviewMode: "none" | "comments" | "changes";
    includeResolved: boolean;
    queryId: string;
    stylePolicy: "template" | "source";
    bindings: Record<string, OfficeBinding>;
    template?: OutputFile;
    inspection?: TemplateInspection;
    bibliographyScope?: "citationWorks" | "selectedWorks";
    works?: string[];
    bibliographySource?: "draft" | "saved";
    includeEmbedded?: boolean;
    legacyBackend?: string;
};
export type OfficeOutputRequest = Pick<OfficeDraft, "outputLocale" | "format" | "reviewMode" | "includeResolved" | "stylePolicy" | "bindings" | "template" | "legacyBackend"> & {
    requestId: string;
    resource?: string;
    source?: string;
    queryId?: string;
    sessionId?: string;
    baseId?: string;
    serial?: number;
    sourceHash?: string;
    bibliographyScope?: "citationWorks" | "selectedWorks";
    works?: string[];
    embeddedQueries?: {
        occurrenceId: string;
        queryId: string;
    }[];
};
export type OfficeOutputVerification = {
    requestId: string;
    epoch: number;
};
export type OfficeImportRequest = {
    resource: string;
    operationId: string;
    bytesBase64: string;
    format?: OfficeImportFormat;
    legacyBackend?: string;
    docxOptions?: DocxImportOptions;
};
const uncertain = (message: UiText) => Object.assign(new UiError(message), { code: "resultUnknown" });
/** Recovery sends the same format, converter, original and confirmed options. */
export function freezeOfficeImportRequest(request: OfficeImportRequest): OfficeImportRequest {
    const format = request.format ?? "docx";
    if (!request.resource || !request.operationId || !request.bytesBase64 || !["docx", "doc", "wps"].includes(format))
        throw new UiError(uiMessage("hostedOfficeState.importInputIncomplete"));
    if (format === "docx" ? !!request.legacyBackend : !request.legacyBackend || !!request.docxOptions)
        throw new UiError(uiMessage("hostedOfficeState.importFormatOptionsMismatch"));
    if (request.docxOptions && (!/^[a-f0-9]{64}$/i.test(request.docxOptions.contentHash) || !Array.isArray(request.docxOptions.captionPairings)))
        throw new UiError(uiMessage("hostedOfficeState.captionOriginalIdentityMissing"));
    return structuredClone(request);
}
/** A receipt may acknowledge state, but cannot replace the frozen artifact. */
export function outputReceipt(requestId: string, incoming: unknown, previous?: OfficeOutputReceipt | FrozenOfficeArtifact): OfficeOutputReceipt {
    const result = decodeHostedOutput(incoming);
    if (result.requestId !== requestId)
        throw uncertain(uiMessage("hostedOfficeState.outputReceiptRequestMismatch"));
    // A terminal lookup acknowledges the original ID without redelivering old
    // target/source-derived preview data after expiration or lost authority.
    if (["unknown", "cancelled", "expired", "failed", "notRegistered"].includes(result.status))
        return result;
    if (result.status === "pending") {
        if (previous && ["prepared", "confirmed"].includes(previous.status))
            throw uncertain(uiMessage("hostedOfficeState.artifactCannotReturnToPending"));
        return result;
    }
    if (previous?.digest)
        for (const key of ["digest", "length", "filename", "contentType"] as const) {
            if (result[key] !== previous[key])
                throw uncertain(uiMessage("hostedOfficeState.originalOutputFileChanged"));
        }
    if (previous?.status === "confirmed" && result.status === "prepared")
        throw uncertain(uiMessage("hostedOfficeState.confirmedOutputCannotReplacePrepared"));
    return result;
}
/** Cache is a recovery locator, never current delivery authority. */
export function officeOutputPresentation(requestId: string, receipt: OfficeOutputReceipt | undefined, verification: OfficeOutputVerification | undefined, epoch: number): OfficeOutputReceipt | undefined {
    return receipt && verification?.requestId === requestId && verification.epoch === epoch ? outputReceipt(requestId, receipt) : undefined;
}
export function importReceipt(request: OfficeImportRequest, incoming: unknown, previous?: OfficeImportReceipt, pinnedPlanId?: string): OfficeImportReceipt {
    const result = decodeHostedImport(incoming);
    if (result.operationId !== request.operationId || result.resource !== request.resource)
        throw uncertain(uiMessage("hostedOfficeState.importReceiptMismatch"));
    if (previous?.planId && result.planId && previous.planId !== result.planId)
        throw uncertain(uiMessage("hostedOfficeState.originalImportPlanChanged"));
    if (pinnedPlanId && result.planId && result.planId !== pinnedPlanId)
        throw uncertain(uiMessage("hostedOfficeState.originalImportPlanChanged"));
    if (previous?.status === "committed" && result.status !== "committed")
        throw uncertain(uiMessage("hostedOfficeState.committedImportCannotReplay"));
    return result;
}
export type OfficeChunk = {
    bytesBase64: string;
    offset: number;
    length: number;
    total: number;
    sha256: string;
    done: boolean;
};
export function officeChunk(chunk: OfficeChunk, offset: number, total: number, sha256: string): Uint8Array {
    if (!chunk || chunk.offset !== offset || chunk.total !== total || chunk.sha256 !== sha256 || typeof chunk.bytesBase64 !== "string" || chunk.bytesBase64.length > 65536 * 4 / 3 + 4 || !Number.isSafeInteger(chunk.length) || chunk.length <= 0 || chunk.length > 65536 || offset + chunk.length > total || chunk.done !== (offset + chunk.length === total))
        throw uncertain(uiMessage("hostedOfficeState.downloadArtifactMismatch"));
    let bytes: Uint8Array;
    try {
        bytes = Uint8Array.from(atob(chunk.bytesBase64), c => c.charCodeAt(0));
    }
    catch {
        throw uncertain(uiMessage("hostedOfficeState.downloadEncodingInvalid"));
    }
    if (bytes.length !== chunk.length)
        throw uncertain(uiMessage("hostedOfficeState.downloadLengthIncomplete"));
    return bytes;
}
