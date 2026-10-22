import { message, parseUiMessage, uiTextValue, UiError, type UiMessage, type UiText } from "./ui-locale";
export type DiagnosticOrigin = "os" | "json" | "typst" | "network" | "externalTool" | "dependency";
export interface RawDiagnostic {
    readonly origin: DiagnosticOrigin;
    readonly text: string;
}
export interface ErrorCause {
    readonly code: string;
    readonly explanation: UiMessage;
    readonly diagnostics: readonly RawDiagnostic[];
}
export interface ErrorDescription {
    readonly explanation: UiMessage;
    readonly diagnostics: readonly RawDiagnostic[];
    readonly causes: readonly ErrorCause[];
}
export type DataErrorKind = "invalid" | "unavailable" | "coverage" | "budget" | "nativeBudget" | "closedExpressionUnavailable";
export type SemanticObjectKey = Readonly<{
    kind: "node" | "directory" | "rootDirectory";
    id: string;
}> | Readonly<{
    kind: "schema" | "settings" | "tags" | "rules";
}>;
export interface SemanticNodeRef {
    readonly workspaceId: string;
    readonly nodeId: string;
}
export type RequiredSemanticEvidence = Readonly<{
    kind: "object";
    object: SemanticObjectKey;
    installedRevision: string | null;
}> | Readonly<{
    kind: "resource";
    owner: SemanticNodeRef;
    reference: string;
    fileRevision: string;
}> | Readonly<{
    kind: "definition" | "queryInvocation";
    owner: SemanticNodeRef;
    key: string;
    sourceRevision: string;
}> | Readonly<{
    kind: "absentConfiguration";
    object: SemanticObjectKey;
}> | Readonly<{
    kind: "annotationTargets";
    owner: SemanticNodeRef;
    annotationFileRevision: string;
}>;
export type EvidenceGap = "originalQualification" | "plannedObjectMaterialization" | "plannedResourceMaterialization" | "definitionAdmission" | "exactHistoricalSource" | "originalNegativeConfiguration" | "legacyRulesFeatureMerge" | "exactAnnotationHistory" | "compiledElementOrigin" | "exactInvocationInputs";
export type ErrorDetailGroup = Readonly<{
    kind: "data";
    classification: DataErrorKind;
    diagnostics: readonly ErrorDescription[];
}> | Readonly<{
    kind: "product";
    code?: string;
    diagnostics: readonly ErrorDescription[];
}> | Readonly<{
    kind: "semanticEvidence";
    gaps: readonly Readonly<{
        obligation: RequiredSemanticEvidence;
        gap: EvidenceGap;
    }>[];
}>;
export interface CoreErrorDetails {
    readonly kind: "aggregate";
    readonly groups: readonly ErrorDetailGroup[];
}
export interface CoreDescription extends ErrorDescription {
    readonly details?: CoreErrorDetails;
}
export interface CoreError extends CoreDescription {
    readonly code: string;
    readonly currentSource?: string;
}
export interface EngineSourceOrigin {
    readonly path: string;
    readonly from: number;
    readonly to: number;
    readonly utf16From: number;
    readonly utf16To: number;
    readonly raw: string;
    readonly syntaxKind: string;
    readonly isMain: boolean;
}
export interface EngineDiagnostic {
    readonly severity: string;
    readonly explanation?: UiMessage;
    readonly diagnostics: readonly RawDiagnostic[];
    readonly origin: EngineSourceOrigin | null;
}
const diagnosticOrigins: ReadonlySet<string> = new Set(["os", "json", "typst", "network", "externalTool", "dependency"]);
const originLabels = {
    os: () => message("diagnostic.os"), json: () => message("diagnostic.json"), typst: () => message("diagnostic.typst"), network: () => message("diagnostic.network"), externalTool: () => message("diagnostic.externalTool"), dependency: () => message("diagnostic.dependency"),
};
export function exactKeys(record: Record<string, unknown>, required: readonly string[], optional: readonly string[] = []): boolean {
    return required.every(key => Object.hasOwn(record, key)) && Object.keys(record).every(key => required.includes(key) || optional.includes(key));
}
/** Original diagnostics are text, with a closed provenance. They never become a product explanation. */
export function decodeRawDiagnostics(value: unknown): readonly RawDiagnostic[] | undefined {
    if (!Array.isArray(value))
        return;
    const diagnostics: RawDiagnostic[] = [];
    for (const candidate of value) {
        if (!candidate || typeof candidate !== "object" || Array.isArray(candidate))
            return;
        const item = candidate as Record<string, unknown>;
        if (!exactKeys(item, ["origin", "text"]) || typeof item.origin !== "string" || !diagnosticOrigins.has(item.origin) || typeof item.text !== "string")
            return;
        diagnostics.push(Object.freeze({ origin: item.origin as DiagnosticOrigin, text: item.text }));
    }
    return Object.freeze(diagnostics);
}
export function decodeErrorDescription(value: unknown): ErrorDescription | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["explanation", "diagnostics", "causes"]))
        return;
    const explanation = parseUiMessage(record.explanation), diagnostics = decodeRawDiagnostics(record.diagnostics), causes = decodeErrorCauses(record.causes);
    return explanation && diagnostics && causes ? Object.freeze({ explanation, diagnostics, causes }) : undefined;
}
export function decodeDescriptionFields(record: Record<string, unknown>): ErrorDescription | undefined {
    return decodeErrorDescription({ explanation: record.explanation, diagnostics: record.diagnostics, causes: record.causes });
}
export function decodeErrorDescriptions(value: unknown): readonly ErrorDescription[] | undefined {
    if (!Array.isArray(value))
        return;
    const result: ErrorDescription[] = [];
    for (const item of value) {
        const description = decodeErrorDescription(item);
        if (!description)
            return;
        result.push(description);
    }
    return Object.freeze(result);
}
export function decodeCoreErrorDetails(value: unknown): CoreErrorDetails | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["kind", "groups"]) || record.kind !== "aggregate" || !Array.isArray(record.groups))
        return;
    const groups: ErrorDetailGroup[] = [];
    for (const value of record.groups) {
        if (!value || typeof value !== "object" || Array.isArray(value))
            return;
        const group = value as Record<string, unknown>;
        if (group.kind === "semanticEvidence") {
            if (!exactKeys(group, ["kind", "gaps"]) || !Array.isArray(group.gaps))
                return;
            const gaps: {
                obligation: RequiredSemanticEvidence;
                gap: EvidenceGap;
            }[] = [];
            for (const value of group.gaps) {
                if (!value || typeof value !== "object" || Array.isArray(value))
                    return;
                const item = value as Record<string, unknown>, obligation = decodeSemanticEvidence(item.obligation);
                if (!exactKeys(item, ["obligation", "gap"]) || !obligation || typeof item.gap !== "string" || !["originalQualification", "plannedObjectMaterialization", "plannedResourceMaterialization", "definitionAdmission", "exactHistoricalSource", "originalNegativeConfiguration", "legacyRulesFeatureMerge", "exactAnnotationHistory", "compiledElementOrigin", "exactInvocationInputs"].includes(item.gap))
                    return;
                gaps.push(Object.freeze({ obligation, gap: item.gap as EvidenceGap }));
            }
            groups.push(Object.freeze({ kind: "semanticEvidence", gaps: Object.freeze(gaps) }));
            continue;
        }
        const diagnostics = decodeErrorDescriptions(group.diagnostics);
        if (!diagnostics)
            return;
        if (group.kind === "data") {
            if (!exactKeys(group, ["kind", "classification", "diagnostics"]) || typeof group.classification !== "string" || !["invalid", "unavailable", "coverage", "budget", "nativeBudget", "closedExpressionUnavailable"].includes(group.classification))
                return;
            groups.push(Object.freeze({ kind: "data", classification: group.classification as DataErrorKind, diagnostics }));
        }
        else if (group.kind === "product") {
            if (!exactKeys(group, ["kind", "diagnostics"], ["code"]) || Object.hasOwn(group, "code") && (typeof group.code !== "string" || !group.code))
                return;
            groups.push(Object.freeze({ kind: "product", ...(typeof group.code === "string" ? { code: group.code } : {}), diagnostics }));
        }
        else
            return;
    }
    return Object.freeze({ kind: "aggregate", groups: Object.freeze(groups) });
}
export function decodeCoreDescription(value: unknown): CoreDescription | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["explanation", "diagnostics", "causes"], ["details"]))
        return;
    const description = decodeDescriptionFields(record), details = Object.hasOwn(record, "details") ? decodeCoreErrorDetails(record.details) : undefined;
    if (!description || Object.hasOwn(record, "details") && !details)
        return;
    return Object.freeze({ ...description, ...(details ? { details } : {}) });
}
export function decodeCoreError(value: unknown): CoreError | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["code", "explanation", "diagnostics", "causes"], ["currentSource", "details"]) || typeof record.code !== "string" || !record.code || Object.hasOwn(record, "currentSource") && typeof record.currentSource !== "string")
        return;
    const description = decodeCoreDescription({ explanation: record.explanation, diagnostics: record.diagnostics, causes: record.causes, ...(Object.hasOwn(record, "details") ? { details: record.details } : {}) });
    return description ? Object.freeze({ ...description, code: record.code, ...(typeof record.currentSource === "string" ? { currentSource: record.currentSource } : {}) }) : undefined;
}
export function decodeEngineDiagnostic(value: unknown): EngineDiagnostic | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const record = value as Record<string, unknown>;
    if (!exactKeys(record, ["severity", "diagnostics", "origin"], ["explanation"]) || typeof record.severity !== "string")
        return;
    const diagnostics = decodeRawDiagnostics(record.diagnostics), explanation = Object.hasOwn(record, "explanation") ? parseUiMessage(record.explanation) : undefined;
    if (!diagnostics || Object.hasOwn(record, "explanation") && !explanation)
        return;
    let origin: EngineSourceOrigin | null = null;
    if (record.origin !== null) {
        if (!record.origin || typeof record.origin !== "object" || Array.isArray(record.origin))
            return;
        const item = record.origin as Record<string, unknown>;
        if (!exactKeys(item, ["path", "from", "to", "utf16From", "utf16To", "raw", "syntaxKind", "isMain"]) || ["path", "raw", "syntaxKind"].some(key => typeof item[key] !== "string") || ["from", "to", "utf16From", "utf16To"].some(key => typeof item[key] !== "number" || !Number.isSafeInteger(item[key]) || Number(item[key]) < 0) || typeof item.isMain !== "boolean" || Number(item.from) > Number(item.to) || Number(item.utf16From) > Number(item.utf16To))
            return;
        origin = Object.freeze({ ...item }) as unknown as EngineSourceOrigin;
    }
    return Object.freeze({ severity: record.severity, diagnostics, origin, ...(explanation ? { explanation } : {}) });
}
export function engineDiagnosticUiText(value: EngineDiagnostic): UiText {
    const diagnostic = decodeEngineDiagnostic(value);
    if (!diagnostic)
        throw new UiError(message("error.invalidResponse"));
    return () => (diagnostic.explanation ? uiTextValue(diagnostic.explanation) : "") + rawDiagnosticsText(diagnostic.diagnostics);
}
export function coreDescriptionUiText(value: CoreDescription): UiText {
    const description = decodeCoreDescription(value);
    if (!description)
        throw new UiError(message("error.invalidResponse"));
    const primary = descriptionUiText(description);
    return () => uiTextValue(primary) + (description.details ? coreDetailsText(description.details) : "");
}
export function coreDetailsText(details: CoreErrorDetails): string {
    return details.groups.map(group => group.kind === "semanticEvidence" ? `\n[semanticEvidence]\n${JSON.stringify(group.gaps, null, 2)}` : `${group.kind === "data" ? `\n[${group.classification}]` : group.code ? `\n[${group.code}]` : ""}${group.diagnostics.map(description => `\n${uiTextValue(descriptionUiText(description))}`).join("")}`).join("");
}
function decodeSemanticEvidence(value: unknown): RequiredSemanticEvidence | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    const item = value as Record<string, unknown>;
    const object = item.object && typeof item.object === "object" && !Array.isArray(item.object) ? item.object as Record<string, unknown> : undefined;
    const validObject = object && (["node", "directory", "rootDirectory"].includes(String(object.kind)) ? exactKeys(object, ["kind", "id"]) && typeof object.id === "string" : ["schema", "settings", "tags", "rules"].includes(String(object.kind)) && exactKeys(object, ["kind"]));
    const owner = item.owner && typeof item.owner === "object" && !Array.isArray(item.owner) ? item.owner as Record<string, unknown> : undefined;
    const validOwner = owner && exactKeys(owner, ["workspaceId", "nodeId"]) && typeof owner.workspaceId === "string" && typeof owner.nodeId === "string";
    let valid = false;
    switch (item.kind) {
        case "object":
            valid = !!validObject && exactKeys(item, ["kind", "object", "installedRevision"]) && (item.installedRevision === null || typeof item.installedRevision === "string");
            break;
        case "absentConfiguration":
            valid = !!validObject && exactKeys(item, ["kind", "object"]);
            break;
        case "resource":
            valid = !!validOwner && exactKeys(item, ["kind", "owner", "reference", "fileRevision"]) && typeof item.reference === "string" && typeof item.fileRevision === "string";
            break;
        case "definition":
        case "queryInvocation":
            valid = !!validOwner && exactKeys(item, ["kind", "owner", "key", "sourceRevision"]) && typeof item.key === "string" && typeof item.sourceRevision === "string";
            break;
        case "annotationTargets":
            valid = !!validOwner && exactKeys(item, ["kind", "owner", "annotationFileRevision"]) && typeof item.annotationFileRevision === "string";
            break;
    }
    return valid ? Object.freeze({ ...item, ...(object ? { object: Object.freeze({ ...object }) } : {}), ...(owner ? { owner: Object.freeze({ ...owner }) } : {}) }) as unknown as RequiredSemanticEvidence : undefined;
}
export function decodeErrorCauses(value: unknown): readonly ErrorCause[] | undefined {
    if (!Array.isArray(value) || value.length > 32)
        return;
    const causes: ErrorCause[] = [];
    for (const candidate of value) {
        if (!candidate || typeof candidate !== "object" || Array.isArray(candidate))
            return;
        const record = candidate as Record<string, unknown>;
        if (!exactKeys(record, ["code", "explanation", "diagnostics"]) || typeof record.code !== "string" || !record.code)
            return;
        const explanation = parseUiMessage(record.explanation), diagnostics = decodeRawDiagnostics(record.diagnostics);
        if (!explanation || !diagnostics)
            return;
        causes.push(Object.freeze({ code: record.code, explanation, diagnostics }));
    }
    return Object.freeze(causes);
}
/** Resolve at display time; changing language does not alter the delivered facts or source. */
export function descriptionUiText(description: ErrorDescription): UiText {
    const parsed = decodeDescriptionFields(description as unknown as Record<string, unknown>);
    if (!parsed) throw new UiError(message("error.invalidResponse"));
    return () => uiTextValue(parsed.explanation) + rawDiagnosticsText(parsed.diagnostics) + parsed.causes.map(cause => `\n[${cause.code}] ${uiTextValue(cause.explanation)}${rawDiagnosticsText(cause.diagnostics)}`).join("");
}
export function rawDiagnosticsText(diagnostics: readonly RawDiagnostic[]): string {
    return diagnostics.length ? `\n${uiTextValue(message("common.originalDiagnostic"))}\n${diagnostics.map(item => `[${uiTextValue(originLabels[item.origin]())}] ${item.text}`).join("\n")}` : "";
}
