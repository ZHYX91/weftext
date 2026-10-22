import { exactKeys } from "./ui-diagnostics";
import { decodeProcessingSettings, type ProcessingLanguage, type ProcessingSettings } from "./processing-settings";
import { message, parseUiMessage, UiError, type UiMessage, type UiText } from "./ui-locale";

export type ProcessingMatch = "supported" | "inapplicable" | "unknown" | "missingDependency";
export type ProofreadFinding = {
    id: string; category: string; ruleId: string; ruleVersion: string; excerpt: string;
    replacement: string | null; executable: boolean;
    sourceRange: { from: number; to: number; utf16From: number; utf16To: number; raw?: string } | null;
} & ({ messageOrigin: "product"; explanation: UiMessage } | { messageOrigin: "author"; authorMessage: string });
export interface ProofreadFragment {
    language: ProcessingLanguage | null; nativeLanguage: ProcessingLanguage | null;
    source: string; checkedRules: string[]; skippedRules: [string, ProcessingMatch][];
}
export interface ProofreadResult {
    coverage: "complete" | "partial"; findings: ProofreadFinding[]; warnings: UiMessage[];
    checkedCharacters: number; existing: string[]; processing: ProcessingSettings;
    processingOrigin: { source: string; baseId: string; revision: string | null } | null;
    fragments: ProofreadFragment[]; scanId?: string; sourceHash?: string;
}
function invalid(): never { throw new UiError(message("error.invalidResponse")); }
function record(value: unknown): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) return invalid(); return value as Record<string, unknown>; }
function keys(value: unknown, required: readonly string[], optional: readonly string[] = []) { const item = record(value); if (!exactKeys(item, required, optional)) return invalid(); return item; }
function text(value: unknown): string { if (typeof value !== "string") return invalid(); return value; }
function integer(value: unknown): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) return invalid(); return value; }
function array<T>(value: unknown, decode: (item: unknown) => T): T[] { if (!Array.isArray(value)) return invalid(); return value.map(decode); }
function uiMessage(value: unknown): UiMessage { return parseUiMessage(value) ?? invalid(); }
function language(value: unknown): ProcessingLanguage | null {
    if (value === null) return null;
    const item = keys(value, ["language", "script", "region"]);
    return { language: text(item.language), script: item.script === null ? null : text(item.script), region: item.region === null ? null : text(item.region) };
}
/** HTTP scopes deliberately differ in source-range disclosure; neither accepts old rendered messages. */
export function decodeProofreadResult(value: unknown, scope: "local" | "hosted"): ProofreadResult {
    const item = keys(value, ["coverage", "findings", "warnings", "checkedCharacters", "processing", "processingOrigin", "fragments", "existing", ...(scope === "hosted" ? ["scanId", "sourceHash"] : [])]);
    if (item.coverage !== "complete" && item.coverage !== "partial") return invalid();
    const processing = decodeProcessingSettings(item.processing);
    const origin = item.processingOrigin === null ? null : keys(item.processingOrigin, ["source", "baseId", "revision"]);
    const findings = array(item.findings, value => {
        const item = record(value), product = item.messageOrigin === "product";
        if (!product && item.messageOrigin !== "author" || !exactKeys(item, ["id", "category", "ruleId", "ruleVersion", "excerpt", "messageOrigin", product ? "explanation" : "authorMessage", "replacement", "sourceRange", "executable"]) || typeof item.executable !== "boolean") return invalid();
        const range = item.sourceRange === null ? null : keys(item.sourceRange, ["from", "to", "utf16From", "utf16To", ...(scope === "local" ? ["raw"] : [])]);
        const sourceRange = range ? { from: integer(range.from), to: integer(range.to), utf16From: integer(range.utf16From), utf16To: integer(range.utf16To), ...(scope === "local" ? { raw: text(range.raw) } : {}) } : null;
        if (sourceRange && (sourceRange.to < sourceRange.from || sourceRange.utf16To < sourceRange.utf16From)) return invalid();
        const common = { id: text(item.id), category: text(item.category), ruleId: text(item.ruleId), ruleVersion: text(item.ruleVersion), excerpt: text(item.excerpt), replacement: item.replacement === null ? null : text(item.replacement), sourceRange, executable: item.executable };
        return product ? { ...common, messageOrigin: "product" as const, explanation: uiMessage(item.explanation) } : { ...common, messageOrigin: "author" as const, authorMessage: text(item.authorMessage) };
    });
    return { coverage: item.coverage, findings, warnings: array(item.warnings, uiMessage), checkedCharacters: integer(item.checkedCharacters), existing: array(item.existing, text), processing,
        processingOrigin: origin ? { source: text(origin.source), baseId: text(origin.baseId), revision: origin.revision === null ? null : text(origin.revision) } : null,
        fragments: array(item.fragments, value => { const fragment = keys(value, ["language", "nativeLanguage", "source", "checkedRules", "skippedRules"]); return { language: language(fragment.language), nativeLanguage: language(fragment.nativeLanguage), source: text(fragment.source), checkedRules: array(fragment.checkedRules, text), skippedRules: array(fragment.skippedRules, value => { if (!Array.isArray(value) || value.length !== 2 || !["supported", "inapplicable", "unknown", "missingDependency"].includes(value[1])) return invalid(); return [text(value[0]), value[1] as ProcessingMatch]; }) }; }),
        ...(scope === "hosted" ? { scanId: text(item.scanId), sourceHash: text(item.sourceHash) } : {}),
    };
}
export function proofreadFindingUiText(finding: ProofreadFinding): UiText { return finding.messageOrigin === "product" ? finding.explanation : finding.authorMessage; }
export function processingMatchUiText(match: ProcessingMatch): UiText {
    switch (match) {
        case "supported": return message("proofreading.ruleScopeSupported");
        case "inapplicable": return message("proofreading.ruleScopeInapplicable");
        case "unknown": return message("proofreading.ruleScopeUnknown");
        case "missingDependency": return message("proofreading.ruleDependencyMissing");
    }
}
