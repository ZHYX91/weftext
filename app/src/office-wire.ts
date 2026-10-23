import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody, type ApiErrorDetails } from "./api-error";
import { coreDescriptionUiText, decodeCoreDescription, decodeCoreErrorDetails, decodeDescriptionFields, decodeEngineDiagnostic, decodeErrorDescriptions, descriptionUiText, engineDiagnosticUiText, exactKeys, type CoreDescription, type CoreErrorDetails, type EngineDiagnostic, type ErrorDescription } from "./ui-diagnostics";
import { joinUiText, message, parseUiMessage, UiError, type UiMessage, type UiText } from "./ui-locale";

/** Only declared product fields are decoded. Author records never become messages by shape. */
type RecordValue = Record<string, unknown>;
const descriptionKeys = ["explanation", "diagnostics", "causes"];
const artifactKeys = ["filename", "contentType", "length", "digest", "losses", "summary", "preview"];
const directions = ["docToDocx", "docxToDoc", "wpsToDocx", "docxToWps"] as const;
export type LegacyDirection = typeof directions[number];
export function invalidOfficeWire(): never { throw Object.assign(new UiError(message("error.invalidResponse")), { code: "resultUnknown" }); }
function record(value: unknown): RecordValue {
    if (!value || typeof value !== "object" || Array.isArray(value)) invalidOfficeWire();
    return value as RecordValue;
}
function keys(value: unknown, required: readonly string[], optional: readonly string[] = []): RecordValue {
    const item = record(value);
    if (!exactKeys(item, required, optional)) invalidOfficeWire();
    return item;
}
function text(value: unknown, nonempty = false): string { if (typeof value !== "string" || nonempty && !value) invalidOfficeWire(); return value; }
function integer(value: unknown, maximum = Number.MAX_SAFE_INTEGER): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > maximum) invalidOfficeWire(); return value; }
function bool(value: unknown): boolean { if (typeof value !== "boolean") invalidOfficeWire(); return value; }
function nullableText(value: unknown): string | null { return value === null ? null : text(value); }
function array<T>(value: unknown, decode: (item: unknown) => T): T[] { if (!Array.isArray(value)) invalidOfficeWire(); return value.map(decode); }
function uiMessage(value: unknown): UiMessage { return parseUiMessage(value) ?? invalidOfficeWire(); }
function description(item: RecordValue): ErrorDescription { return decodeDescriptionFields(item) ?? invalidOfficeWire(); }
function descriptions(value: unknown): ErrorDescription[] { return [...(decodeErrorDescriptions(value) ?? invalidOfficeWire())]; }
function digest(value: unknown): string { const result = text(value); if (!/^[a-f0-9]{64}$/.test(result)) invalidOfficeWire(); return result; }
function stringMap(value: unknown): Record<string, string> { return Object.fromEntries(Object.entries(record(value)).map(([key, item]) => [key, text(item)])); }

export interface ExportError extends ErrorDescription { productDiagnostics: ErrorDescription[]; }
export interface OutputLoss extends ExportError { code: string; location?: string; }
export interface NativeLoss extends ExportError { code: string; path: string; }
export interface ConversionLoss extends ExportError { code: string; location: string | null; }
export type OfficeLoss = OutputLoss | NativeLoss | ConversionLoss;
export function decodeExportError(value: unknown): ExportError {
    const item = keys(value, [...descriptionKeys, "productDiagnostics"]);
    return { ...description(item), productDiagnostics: descriptions(item.productDiagnostics) };
}
export function decodeOutputLoss(value: unknown): OutputLoss {
    const item = keys(value, ["code", ...descriptionKeys, "productDiagnostics"], ["location"]);
    return { ...description(item), code: text(item.code, true), productDiagnostics: descriptions(item.productDiagnostics), ...(Object.hasOwn(item, "location") ? { location: text(item.location) } : {}) };
}
export function decodeNativeLoss(value: unknown): NativeLoss {
    const item = keys(value, ["path", "code", ...descriptionKeys, "productDiagnostics"]);
    return { ...description(item), code: text(item.code, true), path: text(item.path), productDiagnostics: descriptions(item.productDiagnostics) };
}
function decodeConversionLoss(value: unknown): ConversionLoss {
    const item = keys(value, ["code", ...descriptionKeys, "productDiagnostics", "location"]);
    return { ...description(item), code: text(item.code, true), productDiagnostics: descriptions(item.productDiagnostics), location: nullableText(item.location) };
}
export function exportErrorUiText(value: ExportError): UiText { return joinUiText([descriptionUiText(value), ...value.productDiagnostics.map(descriptionUiText)], "\n"); }
export function officeLossUiText(value: OfficeLoss): UiText {
    const location = "path" in value ? value.path : value.location;
    return joinUiText([exportErrorUiText(value), ...(location ? [" (" + location + ")"] : [])]);
}

export interface LegacyCapability extends ErrorDescription { direction: LegacyDirection; available: boolean; contentRoundtripVerified: boolean; }
export interface LegacyCapabilities { backends: { id: string; version: string | null; directions: LegacyCapability[] }[]; ordinaryDocxRequiresOffice: boolean; executionLocation: "local" | "server"; }
export function decodeLegacyCapabilities(value: unknown, executionLocation: "local" | "server"): LegacyCapabilities {
    const item = keys(value, ["backends", "ordinaryDocxRequiresOffice", "executionLocation"]);
    if (item.executionLocation !== executionLocation) invalidOfficeWire();
    return { executionLocation, ordinaryDocxRequiresOffice: bool(item.ordinaryDocxRequiresOffice), backends: array(item.backends, value => {
        const backend = keys(value, ["id", "version", "directions"]);
        return { id: text(backend.id, true), version: nullableText(backend.version), directions: array(backend.directions, value => {
            const capability = keys(value, ["direction", "available", "contentRoundtripVerified", ...descriptionKeys]);
            if (!directions.includes(capability.direction as LegacyDirection)) invalidOfficeWire();
            return { ...description(capability), direction: capability.direction as LegacyDirection, available: bool(capability.available), contentRoundtripVerified: bool(capability.contentRoundtripVerified) };
        }) };
    }) };
}
export interface TemplateAnchor { name: string; kind: "content" | "cell" | "range" | "table"; label: string; supported: boolean; description: UiMessage | null; reason: ExportError | null; placement?: string; rows?: number; columns?: number; capacity?: number; }
export interface TemplateInspection { slots: TemplateAnchor[]; warnings: never[]; }
export function decodeTemplateInspection(value: unknown, format: string): TemplateInspection {
    const item = keys(value, ["slots", "warnings"]);
    if (!Array.isArray(item.warnings) || item.warnings.length || !["docx", "xlsx"].includes(format)) invalidOfficeWire();
    return { warnings: [], slots: array(item.slots, value => {
        const slot = keys(value, ["name", "kind", "label", "supported", "description", "reason", ...(format === "docx" ? ["placement"] : ["rows", "columns", "capacity"])]);
        if (format === "docx" ? slot.kind !== "content" || slot.description !== null : !["cell", "range", "table"].includes(String(slot.kind))) invalidOfficeWire();
        return { name: text(slot.name, true), kind: slot.kind as TemplateAnchor["kind"], label: text(slot.label), supported: bool(slot.supported), description: slot.description === null ? null : uiMessage(slot.description), reason: slot.reason === null ? null : decodeExportError(slot.reason), ...(format === "docx" ? { placement: text(slot.placement) } : { rows: integer(slot.rows, 0xffffffff), columns: integer(slot.columns, 0xffffffff), capacity: integer(slot.capacity, 0xffffffff) }) };
    }) };
}

interface LegacyArtifact { filename: string; length: number; sha256: string; }
export interface LegacyFeature extends ErrorDescription { feature: string; before: string; after: string; }
export interface LegacyMetadata extends ErrorDescription {
    requestId: string; status: string; code: string; backend: string; backendVersion: string | null; direction: LegacyDirection; inputFormat: string; outputFormat: string; contentType: string; stage: string;
    original: LegacyArtifact; intermediateDocx: LegacyArtifact | null; generated: LegacyArtifact | null; features: LegacyFeature[]; losses: ConversionLoss[];
    requiresLossConfirmation: boolean; contentRoundtripVerified: boolean; nativeConversionRequiresSeparateReport: boolean; details?: CoreErrorDetails;
}
export function decodeLegacyMetadata(value: unknown): LegacyMetadata {
    const item = keys(value, ["requestId", "status", "code", ...descriptionKeys, "backend", "backendVersion", "direction", "inputFormat", "outputFormat", "contentType", "stage", "original", "intermediateDocx", "generated", "features", "losses", "requiresLossConfirmation", "contentRoundtripVerified", "nativeConversionRequiresSeparateReport"], ["details"]);
    if (!directions.includes(item.direction as LegacyDirection) || !["succeeded", "failed", "cancelled", "unknown", "unavailable"].includes(String(item.status))) invalidOfficeWire();
    const artifact = (value: unknown): LegacyArtifact => { const item = keys(value, ["filename", "length", "sha256"]); return { filename: text(item.filename), length: integer(item.length, 64 * 1024 * 1024), sha256: digest(item.sha256) }; };
    const details = Object.hasOwn(item, "details") ? decodeCoreErrorDetails(item.details) ?? invalidOfficeWire() : undefined;
    return { ...description(item), requestId: text(item.requestId, true), status: text(item.status), code: text(item.code, true), backend: text(item.backend, true), backendVersion: nullableText(item.backendVersion), direction: item.direction as LegacyDirection, inputFormat: text(item.inputFormat), outputFormat: text(item.outputFormat), contentType: text(item.contentType), stage: text(item.stage), original: artifact(item.original), intermediateDocx: item.intermediateDocx === null ? null : artifact(item.intermediateDocx), generated: item.generated === null ? null : artifact(item.generated), features: array(item.features, value => { const feature = keys(value, ["feature", "before", "after", ...descriptionKeys]); return { ...description(feature), feature: text(feature.feature), before: text(feature.before), after: text(feature.after) }; }), losses: array(item.losses, decodeConversionLoss), requiresLossConfirmation: bool(item.requiresLossConfirmation), contentRoundtripVerified: bool(item.contentRoundtripVerified), nativeConversionRequiresSeparateReport: bool(item.nativeConversionRequiresSeparateReport), ...(details ? { details } : {}) };
}
export function legacyMetadataUiText(value: LegacyMetadata): UiText[] {
    const primary: CoreDescription = { explanation: value.explanation, diagnostics: value.diagnostics, causes: value.causes, ...(value.details ? { details: value.details } : {}) };
    return [coreDescriptionUiText(primary), ...value.features.map(feature => joinUiText([feature.feature + ": ", descriptionUiText(feature)])), ...value.losses.map(officeLossUiText)];
}

/** Preview is a heterogeneous facts document; only these producer-owned slots are messages. */
export interface OfficeEmbeddedQuery {
    occurrenceId: string; label: string; queryId: string; scope: "previouslyDisplayedFrozenResult"; cutId: string; coverage: "complete" | "partial"; freshness: "current" | "stale"; limited: boolean; rowCount: number; status: UiMessage[]; diagnostics: CoreDescription[];
    engineDiagnostics: { entityKey: string; diagnostics: EngineDiagnostic[] }[]; productErrors: { entityKey: string; error: ApiErrorBody }[]; columns: string[];
}
function decodeOfficeEmbeddedQuery(value: unknown): OfficeEmbeddedQuery {
    const item = keys(value, ["occurrenceId", "label", "queryId", "scope", "cutId", "coverage", "freshness", "limited", "rowCount", "status", "diagnostics", "engineDiagnostics", "productErrors", "columns"]);
    if (item.scope !== "previouslyDisplayedFrozenResult" || !["complete", "partial"].includes(String(item.coverage)) || !["current", "stale"].includes(String(item.freshness))) invalidOfficeWire();
    return { occurrenceId: text(item.occurrenceId, true), label: text(item.label), queryId: text(item.queryId, true), scope: "previouslyDisplayedFrozenResult", cutId: text(item.cutId), coverage: item.coverage as "complete" | "partial", freshness: item.freshness as "current" | "stale", limited: bool(item.limited), rowCount: integer(item.rowCount), status: array(item.status, uiMessage), diagnostics: array(item.diagnostics, value => decodeCoreDescription(value) ?? invalidOfficeWire()), engineDiagnostics: array(item.engineDiagnostics, value => { const group = keys(value, ["entityKey", "diagnostics"]); return { entityKey: text(group.entityKey), diagnostics: array(group.diagnostics, value => decodeEngineDiagnostic(value) ?? invalidOfficeWire()) }; }), productErrors: array(item.productErrors, value => { const group = keys(value, ["entityKey", "error"]); return { entityKey: text(group.entityKey), error: decodeApiErrorBody(group.error) ?? invalidOfficeWire() }; }), columns: array(item.columns, value => text(value)) };
}
export interface OfficePreview extends RecordValue { summary?: UiMessage; recoveryNotice?: UiMessage; warnings?: CoreDescription[]; explanation?: UiMessage; causes?: ErrorDescription["causes"]; diagnostics?: EngineDiagnostic[] | ErrorDescription["diagnostics"]; legacy?: LegacyMetadata; legacyConversion?: LegacyMetadata; embeddedQueries?: OfficeEmbeddedQuery[]; }
export function decodeOfficePreview(value: unknown): OfficePreview | null {
    if (value === null) return null;
    const item = structuredClone(record(value)) as OfficePreview;
    for (const key of ["summary", "recoveryNotice"] as const) if (Object.hasOwn(item, key)) item[key] = uiMessage(item[key]);
    for (const key of ["legacy", "legacyConversion"] as const) if (Object.hasOwn(item, key)) item[key] = decodeLegacyMetadata(item[key]);
    if (Object.hasOwn(item, "embeddedQueries")) item.embeddedQueries = array(item.embeddedQueries, decodeOfficeEmbeddedQuery);
    if (Object.hasOwn(item, "warnings")) item.warnings = array(item.warnings, value => decodeCoreDescription(value) ?? invalidOfficeWire());
    if (Object.hasOwn(item, "diagnostics") && !Object.hasOwn(item, "explanation")) item.diagnostics = array(item.diagnostics, value => decodeEngineDiagnostic(value) ?? invalidOfficeWire());
    if (Object.hasOwn(item, "document")) {
        const document = { ...record(item.document) };
        if (Object.hasOwn(document, "warnings")) document.warnings = array(document.warnings, value => decodeCoreDescription(value) ?? invalidOfficeWire());
        item.document = document;
    }
    if (Object.hasOwn(item, "query")) {
        const query = { ...record(item.query) };
        if (Object.hasOwn(query, "reason")) query.reason = uiMessage(query.reason);
        item.query = query;
    }
    // Interrupted hosted jobs store the explicitly typed unknown_answer in preview.
    if (Object.hasOwn(item, "explanation")) {
        const parsed = description(item);
        item.explanation = parsed.explanation; item.diagnostics = parsed.diagnostics; item.causes = parsed.causes;
    }
    return item;
}
export function officePreviewNotices(value: OfficePreview | null): UiText[] {
    if (!value) return [];
    const notices: UiText[] = [];
    if (value.summary) notices.push(value.summary);
    if (value.recoveryNotice) notices.push(value.recoveryNotice);
    if (value.explanation) notices.push(descriptionUiText(description(value)));
    else if (value.diagnostics) notices.push(...(value.diagnostics as EngineDiagnostic[]).map(engineDiagnosticUiText));
    notices.push(...(value.warnings ?? []).map(coreDescriptionUiText));
    const document = value.document as { warnings?: CoreDescription[] } | undefined, query = value.query as { reason?: UiMessage } | undefined;
    notices.push(...(document?.warnings ?? []).map(coreDescriptionUiText));
    if (query?.reason) notices.push(query.reason);
    for (const query of value.embeddedQueries ?? []) {
        notices.push(...query.status.map(status => joinUiText([query.label + ": ", status])), ...query.diagnostics.map(coreDescriptionUiText));
        for (const group of query.engineDiagnostics) notices.push(...group.diagnostics.map(diagnostic => joinUiText([group.entityKey + ": ", engineDiagnosticUiText(diagnostic)])));
        for (const group of query.productErrors) notices.push(joinUiText([group.entityKey + ": ", apiErrorUiText(group.error)]));
    }
    return notices;
}
/** Remove only product-owned display slots; nested author records keep all original keys/values. */
export function officePreviewFacts(value: OfficePreview | null): unknown {
    if (!value) return value;
    const facts: RecordValue = { ...value };
    for (const key of ["summary", "recoveryNotice", "warnings", "diagnostics", "explanation", "causes", "legacy", "legacyConversion"]) delete facts[key];
    if (value.document) { const document = { ...record(value.document) }; delete document.warnings; facts.document = document; }
    if (value.query) { const query = { ...record(value.query) }; delete query.reason; facts.query = query; }
    if (value.embeddedQueries) facts.embeddedQueries = value.embeddedQueries.map(({ status, diagnostics, engineDiagnostics, productErrors, ...facts }) => facts);
    return facts;
}

export interface PreparedOutput { id: string; requestId: string; filename: string; contentType: string; length: number; digest: string; summary: UiMessage; losses: OutputLoss[]; preview: OfficePreview | null; expiresInSeconds: number; status: "prepared" | "confirmed"; }
export type OutputJobFailure = { id: string; requestId: string; status: "pending" } | ({ id: string; requestId: string; status: "failed"; code: string; details?: ApiErrorDetails | null } & ErrorDescription);
function outputArtifact(item: RecordValue): Pick<PreparedOutput, "filename" | "contentType" | "length" | "digest" | "summary" | "losses" | "preview"> {
    return { filename: text(item.filename, true), contentType: text(item.contentType, true), length: integer(item.length, 64 * 1024 * 1024), digest: digest(item.digest), summary: uiMessage(item.summary), losses: array(item.losses, decodeOutputLoss), preview: decodeOfficePreview(item.preview) };
}
export function decodeLocalOutput(value: unknown): PreparedOutput | OutputJobFailure {
    const item = record(value), id = text(item.id, true), requestId = text(item.requestId, true);
    if (item.status === "pending") { keys(item, ["id", "requestId", "status"]); return { id, requestId, status: "pending" }; }
    if (item.status === "failed") {
        keys(item, ["id", "requestId", "status", "code", ...descriptionKeys], ["details"]);
        const parsed = decodeApiErrorBody({ code: item.code, ...description(item), ...(Object.hasOwn(item, "details") && item.details !== null ? { details: item.details } : {}) }) ?? invalidOfficeWire();
        return { id, requestId, status: "failed", code: parsed.code, explanation: parsed.explanation, diagnostics: parsed.diagnostics, causes: parsed.causes, ...(Object.hasOwn(item, "details") ? { details: item.details === null ? null : parsed.details! } : {}) };
    }
    keys(item, ["id", "requestId", "status", ...artifactKeys, "expiresInSeconds"]);
    if (item.status !== "prepared" && item.status !== "confirmed") invalidOfficeWire();
    return { id, requestId, status: item.status, ...outputArtifact(item), expiresInSeconds: integer(item.expiresInSeconds) };
}
export function outputFailureUiText(value: Extract<OutputJobFailure, { status: "failed" }>): UiText {
    const parsed = decodeApiErrorBody({ code: value.code, explanation: value.explanation, diagnostics: value.diagnostics, causes: value.causes, ...(value.details ? { details: value.details } : {}) }) ?? invalidOfficeWire();
    return apiErrorUiText(parsed);
}
export type OfficeOutputStatus = "pending" | "unknown" | "cancelled" | "prepared" | "confirmed" | "failed" | "expired" | "notRegistered";
export interface OfficeOutputReceipt { requestId: string; status: OfficeOutputStatus; filename?: string; contentType?: string; length?: number; digest?: string; losses?: OutputLoss[]; summary?: UiMessage; preview?: OfficePreview | null; code?: string; explanation?: UiMessage; diagnostics?: ErrorDescription["diagnostics"]; causes?: ErrorDescription["causes"]; executionLocation?: "server"; }
export interface FrozenOfficeArtifact { requestId: string; status: "prepared" | "confirmed"; filename: string; contentType: string; length: number; digest: string; }
export function decodeFrozenOfficeArtifact(value: unknown): FrozenOfficeArtifact {
    const item = keys(value, ["requestId", "status", "filename", "contentType", "length", "digest"]);
    if (item.status !== "prepared" && item.status !== "confirmed") invalidOfficeWire();
    return { requestId: text(item.requestId, true), status: item.status, filename: text(item.filename, true), contentType: text(item.contentType, true), length: integer(item.length, 64 * 1024 * 1024), digest: digest(item.digest) };
}
export function decodeHostedOutput(value: unknown): OfficeOutputReceipt {
    const item = record(value), requestId = text(item.requestId, true), status = text(item.status) as OfficeOutputStatus;
    if (!["pending", "unknown", "cancelled", "prepared", "confirmed", "failed", "expired", "notRegistered"].includes(status)) invalidOfficeWire();
    if (status === "prepared" || status === "confirmed") { keys(item, ["requestId", "status", ...artifactKeys]); return { requestId, status, ...outputArtifact(item) }; }
    if (status === "expired" || status === "notRegistered") { keys(item, ["requestId", "status"]); return { requestId, status }; }
    if (Object.hasOwn(item, "explanation")) {
        keys(item, ["requestId", "status", "code", ...descriptionKeys, "executionLocation"]);
        if (status !== "unknown" || item.executionLocation !== "server") invalidOfficeWire();
        return { requestId, status, code: text(item.code, true), ...description(item), executionLocation: "server" };
    }
    keys(item, ["requestId", "status", "preview"], ["executionLocation"]);
    if (Object.hasOwn(item, "executionLocation") && item.executionLocation !== "server") invalidOfficeWire();
    const preview = decodeOfficePreview(item.preview);
    // Keep current server-supplied conversion explanations, never a stale source/artifact preview.
    const safe: OfficePreview | null = preview ? { ...(preview.legacy ? { legacy: preview.legacy } : {}), ...(preview.legacyConversion ? { legacyConversion: preview.legacyConversion } : {}), ...(preview.explanation ? { explanation: preview.explanation, diagnostics: preview.diagnostics, causes: preview.causes } : {}) } : null;
    return { requestId, status, preview: safe, ...(item.executionLocation ? { executionLocation: "server" } : {}) };
}

export interface OfficeImportReceipt { operationId: string; resource: string; status: string; planId: string | null; preview: OfficePreview & { losses?: OutputLoss[] }; commitAllowed: boolean; }
export function decodeHostedImport(value: unknown): OfficeImportReceipt {
    const item = keys(value, ["operationId", "resource", "status", "planId", "preview", "commitAllowed"]);
    if (!["pending", "prepared", "committed", "cancelled", "resolved", "notRegistered", "notCommitted", "recoveryPending", "failed", "expired"].includes(String(item.status))) invalidOfficeWire();
    const preview = decodeOfficePreview(item.preview) ?? invalidOfficeWire();
    if (!exactKeys(preview, [], ["summary", "losses", "legacyConversion"])) invalidOfficeWire();
    if (Object.hasOwn(preview, "losses")) preview.losses = array(preview.losses, value => {
        // The hosted Core wrapper deliberately discloses no private path/location.
        keys(value, ["code", ...descriptionKeys, "productDiagnostics"]);
        return decodeOutputLoss(value);
    });
    const result = { operationId: text(item.operationId, true), resource: text(item.resource, true), status: text(item.status), planId: nullableText(item.planId), preview, commitAllowed: bool(item.commitAllowed) } as OfficeImportReceipt;
    if (result.commitAllowed && (result.status !== "prepared" || !result.planId || !result.preview.losses || !result.preview.summary)) invalidOfficeWire();
    if (!result.commitAllowed && result.planId !== null) invalidOfficeWire();
    if (result.status !== "prepared" && result.commitAllowed) invalidOfficeWire();
    return result;
}

export interface ExchangePlan { operationId: string; planId: string; kind: string; status: string; changes: { path: string; action: string; bytes: number }[]; nodeMap: Record<string, string>; newNodeId: string | null; revisions: Record<string, string>; resourceMap: Record<string, string>; losses: NativeLoss[]; legacyConversion?: LegacyMetadata; contentPreview?: { source: string; truncated: boolean; comments: number; revisions: number; annotations: { author: string; body: string; excerpt: string | null; unlocated: boolean }[] }; }
export function decodeExchangePlan(value: unknown): ExchangePlan {
    const item = keys(value, ["operationId", "planId", "kind", "status", "changes", "nodeMap", "newNodeId", "revisions", "resourceMap", "losses"], ["legacyConversion", "contentPreview"]);
    const result: ExchangePlan = { operationId: text(item.operationId, true), planId: text(item.planId, true), kind: text(item.kind), status: text(item.status), changes: array(item.changes, value => { const change = keys(value, ["path", "action", "bytes"]); return { path: text(change.path), action: text(change.action), bytes: integer(change.bytes) }; }), nodeMap: stringMap(item.nodeMap), newNodeId: nullableText(item.newNodeId), revisions: stringMap(item.revisions), resourceMap: stringMap(item.resourceMap), losses: array(item.losses, decodeNativeLoss) };
    if (Object.hasOwn(item, "legacyConversion")) result.legacyConversion = decodeLegacyMetadata(item.legacyConversion);
    if (Object.hasOwn(item, "contentPreview")) {
        const content = keys(item.contentPreview, ["source", "truncated", "comments", "revisions", "annotations"]);
        result.contentPreview = { source: text(content.source), truncated: bool(content.truncated), comments: integer(content.comments), revisions: integer(content.revisions), annotations: array(content.annotations, value => { const annotation = keys(value, ["author", "body", "excerpt", "unlocated"]); return { author: text(annotation.author), body: text(annotation.body), excerpt: nullableText(annotation.excerpt), unlocated: bool(annotation.unlocated) }; }) };
    }
    return result;
}
export interface ExchangeReceipt { operationId: string; planId: string; status: string; path: string; nodeIds: string[]; }
export function decodeExchangeReceipt(value: unknown): ExchangeReceipt {
    const item = keys(value, ["operationId", "planId", "status", "path", "nodeIds"]);
    return { operationId: text(item.operationId, true), planId: text(item.planId, true), status: text(item.status), path: text(item.path), nodeIds: array(item.nodeIds, value => text(value)) };
}
export function officeOutputUiText(value: OfficeOutputReceipt): UiText | undefined { return value.explanation ? descriptionUiText(description(value as unknown as RecordValue)) : value.summary; }
