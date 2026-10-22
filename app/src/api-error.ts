import { UiError, message, parseUiMessage, type UiMessage } from "./ui-locale";
import { decodeRawDiagnostics, decodeErrorCauses, decodeCoreError, decodeCoreErrorDetails, decodeEngineDiagnostic, decodeErrorDescriptions, descriptionUiText, coreDescriptionUiText, engineDiagnosticUiText, exactKeys, type RawDiagnostic, type ErrorCause, type ErrorDescription, type CoreError, type EngineDiagnostic } from "./ui-diagnostics";
import { uiTextValue, type UiText } from "./ui-locale";
export type { DiagnosticOrigin, RawDiagnostic } from "./ui-diagnostics";
export interface OperationReceiptFacts {
    readonly operationId: string;
    readonly planId: string;
    readonly status: string;
    readonly path: string;
    readonly nodeIds: readonly string[];
}
export interface ApiErrorBody extends ErrorDescription {
    readonly code: string;
    readonly details?: ApiErrorDetails;
}
export type ApiErrorDetails = Readonly<{
    kind: "core";
} & CoreError> | Readonly<{
    kind: "currentSource";
    currentSource: string;
}> | Readonly<{
    kind: "schemaSource";
    source: string;
    roleRef: string | null;
    revision: string | null;
    cause: ApiErrorBody;
}> | Readonly<{
    kind: "originalOperation";
    fact: unknown;
    cause: ApiErrorBody;
}> | Readonly<{
    kind: "engineDiagnostics";
    diagnostics: readonly EngineDiagnostic[];
}> | Readonly<{
    kind: "productDiagnostics";
    code: string | null;
    diagnostics: readonly ErrorDescription[];
}> | Readonly<{
    kind: "committedReceipt";
    receipt: OperationReceiptFacts;
    cause: ApiErrorBody;
}> | Readonly<{
    kind: "sourcePreserved";
    serial: number;
    sourcePreserved: boolean;
    cause: ApiErrorBody;
}>;
function currentSource(details?: ApiErrorDetails): string | undefined {
    if (!details)
        return;
    if (details.kind === "core" || details.kind === "currentSource")
        return details.currentSource;
    if ("cause" in details)
        return currentSource(details.cause.details);
}
function apiDescription(body: ApiErrorBody): UiText {
    const primary = descriptionUiText(body);
    return () => uiTextValue(primary) + detailsText(body.details);
}
function detailsText(details?: ApiErrorDetails): string {
    if (!details || details.kind === "currentSource")
        return "";
    if (details.kind === "core")
        return `\n[${details.code}] ${uiTextValue(coreDescriptionUiText({ explanation: details.explanation, diagnostics: details.diagnostics, causes: details.causes, ...(details.details ? { details: details.details } : {}) }))}`;
    if (details.kind === "engineDiagnostics")
        return details.diagnostics.map(item => `\n${uiTextValue(engineDiagnosticUiText(item))}`).join("");
    if (details.kind === "productDiagnostics")
        return `${details.code ? `\n[${details.code}]` : ""}${details.diagnostics.map(item => `\n${uiTextValue(descriptionUiText(item))}`).join("")}`;
    return `\n[${details.cause.code}] ${uiTextValue(apiDescription(details.cause))}`;
}
/** One typed error envelope. Original diagnostics and conflict content never become translations. */
export class ApiError extends UiError {
    readonly currentSource?: string;
    readonly diagnostics: readonly RawDiagnostic[];
    readonly causes: readonly ErrorCause[];
    constructor(readonly code: string, readonly explanation: UiMessage, diagnostics: readonly RawDiagnostic[] = [], readonly details?: ApiErrorDetails, causes: readonly ErrorCause[] = [], readonly httpStatus?: number) {
        const original = Object.freeze(diagnostics.map(item => Object.freeze({ ...item })));
        const originalCauses = Object.freeze(causes.map(cause => Object.freeze({ ...cause, diagnostics: Object.freeze(cause.diagnostics.map(item => Object.freeze({ ...item }))) })));
        super(apiDescription({ code, explanation, diagnostics: original, causes: originalCauses, ...(details ? { details } : {}) }));
        this.name = "ApiError";
        this.diagnostics = original;
        this.causes = originalCauses;
        this.currentSource = currentSource(details);
    }
}
/** Strictly decode the delivered schema; old message strings and extra fields are not a second protocol. */
export function decodeApiError(value: unknown): ApiError | undefined {
    try {
        const body = decodeBody(value, new Set());
        return body ? new ApiError(body.code, body.explanation, body.diagnostics, body.details, body.causes) : undefined;
    }
    catch {
        return;
    }
}
/** Result metadata retains the wire facts, without storing an Error or a display getter. */
export function decodeApiErrorBody(value: unknown): ApiErrorBody | undefined {
    try { return decodeBody(value, new Set()); } catch { return; }
}
export function apiErrorUiText(value: ApiErrorBody): UiText {
    const body = decodeApiErrorBody(value);
    if (!body) throw new UiError(message("error.invalidResponse"));
    return apiDescription(body);
}
function decodeBody(value: unknown, active: Set<object>): ApiErrorBody | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    if (active.has(value))
        return;
    active.add(value);
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["code", "explanation", "diagnostics", "causes"], ["details"]) || typeof record.code !== "string" || !record.code || !Array.isArray(record.diagnostics))
        return;
    const explanation = parseUiMessage(record.explanation);
    if (!explanation)
        return;
    const diagnostics = decodeRawDiagnostics(record.diagnostics), causes = decodeErrorCauses(record.causes);
    if (!diagnostics || !causes)
        return;
    const details = Object.hasOwn(record, "details") ? decodeDetails(record.details, active) : undefined;
    if (Object.hasOwn(record, "details") && !details)
        return;
    active.delete(value);
    return Object.freeze({ code: record.code, explanation, diagnostics, causes, ...(details ? { details } : {}) });
}
function decodeDetails(value: unknown, active: Set<object>): ApiErrorDetails | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    switch (record.kind) {
        case "core": {
            const { kind, ...body } = record, core = decodeCoreError(body);
            return core ? Object.freeze({ kind: "core", ...core }) : undefined;
        }
        case "currentSource": return exactKeys(record, ["kind", "currentSource"]) && typeof record.currentSource === "string" ? Object.freeze({ kind: "currentSource", currentSource: record.currentSource }) : undefined;
        case "engineDiagnostics": {
            if (!exactKeys(record, ["kind", "diagnostics"]) || !Array.isArray(record.diagnostics))
                return;
            const diagnostics: EngineDiagnostic[] = [];
            for (const value of record.diagnostics) {
                const item = decodeEngineDiagnostic(value);
                if (!item)
                    return;
                diagnostics.push(item);
            }
            return Object.freeze({ kind: "engineDiagnostics", diagnostics: Object.freeze(diagnostics) });
        }
        case "productDiagnostics": {
            if (!exactKeys(record, ["kind", "code", "diagnostics"]) || record.code !== null && typeof record.code !== "string")
                return;
            const diagnostics = decodeErrorDescriptions(record.diagnostics);
            return diagnostics ? Object.freeze({ kind: "productDiagnostics", code: record.code as string | null, diagnostics }) : undefined;
        }
        case "schemaSource": {
            if (!exactKeys(record, ["kind", "source", "roleRef", "revision", "cause"]) || typeof record.source !== "string" || record.roleRef !== null && typeof record.roleRef !== "string" || record.revision !== null && typeof record.revision !== "string")
                return;
            const cause = decodeBody(record.cause, active);
            return cause ? Object.freeze({ kind: "schemaSource", source: record.source, roleRef: record.roleRef as string | null, revision: record.revision as string | null, cause }) : undefined;
        }
        case "originalOperation": {
            if (!exactKeys(record, ["kind", "fact", "cause"]))
                return;
            const cause = decodeBody(record.cause, active);
            return cause ? Object.freeze({ kind: "originalOperation", fact: record.fact, cause }) : undefined;
        }
        case "sourcePreserved": {
            if (!exactKeys(record, ["kind", "serial", "sourcePreserved", "cause"]) || typeof record.serial !== "number" || !Number.isSafeInteger(record.serial) || record.serial < 0 || typeof record.sourcePreserved !== "boolean")
                return;
            const cause = decodeBody(record.cause, active);
            return cause ? Object.freeze({ kind: "sourcePreserved", serial: record.serial, sourcePreserved: record.sourcePreserved, cause }) : undefined;
        }
        case "committedReceipt": {
            if (!exactKeys(record, ["kind", "receipt", "cause"]) || !record.receipt || typeof record.receipt !== "object" || Array.isArray(record.receipt))
                return;
            const receipt = record.receipt as Record<string, unknown>;
            if (!exactKeys(receipt, ["operationId", "planId", "status", "path", "nodeIds"]) || ["operationId", "planId", "status", "path"].some(key => typeof receipt[key] !== "string") || !Array.isArray(receipt.nodeIds) || receipt.nodeIds.some(item => typeof item !== "string"))
                return;
            const cause = decodeBody(record.cause, active);
            return cause ? Object.freeze({ kind: "committedReceipt", receipt: Object.freeze({ operationId: receipt.operationId as string, planId: receipt.planId as string, status: receipt.status as string, path: receipt.path as string, nodeIds: Object.freeze([...receipt.nodeIds]) }), cause }) : undefined;
        }
    }
}
