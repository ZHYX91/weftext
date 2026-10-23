import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
import { exactKeys } from "./ui-diagnostics";
import type { HostedLinksContext } from "./hosted-knowledge-links-state";
type CitationWorkBase = {
    resource: string;
    title: string;
    use: true;
};
export type CitationWork = CitationWorkBase & ({ status: "ready"; reference: string; export: boolean; download: boolean } | { status: "unavailable"; export: false; download: false });
export interface CitationCatalog { works: CitationWork[]; coverage: "complete" | "partial" }
export function decodeCitationCatalog(value: unknown): CitationCatalog {
    const invalid = (): never => { throw new UiError(uiMessage("hostedCitations.catalogFormatUnconfirmed")); };
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalid();
    const item = value as Record<string, unknown>;
    if (!exactKeys(item, ["works", "coverage"]) || !Array.isArray(item.works) || item.coverage !== "complete" && item.coverage !== "partial") return invalid();
    const works = item.works.map((value): CitationWork => {
        if (!value || typeof value !== "object" || Array.isArray(value)) return invalid();
        const work = value as Record<string, unknown>;
        if (!exactKeys(work, ["resource", "title", "status", "use", "export", "download", ...(work.status === "ready" ? ["reference"] : [])]) || typeof work.resource !== "string" || typeof work.title !== "string" || work.use !== true) return invalid();
        if (work.status === "ready" && typeof work.reference === "string" && typeof work.export === "boolean" && typeof work.download === "boolean")
            return { resource: work.resource, title: work.title, use: true, status: "ready", reference: work.reference, export: work.export, download: work.download };
        if (work.status === "unavailable" && work.export === false && work.download === false)
            return { resource: work.resource, title: work.title, use: true, status: "unavailable", export: false, download: false };
        return invalid();
    });
    return { works, coverage: item.coverage };
}
export type CitationDraft = {
    key: string;
    work: string;
};
/** Only author input is cached; protocol descriptions never share this namespace. */
export function decodeCitationDrafts(value: unknown): Record<string, CitationDraft> {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new UiError(uiMessage("hostedCitations.inputCacheUnparseable"));
    const drafts: Record<string, CitationDraft> = Object.create(null);
    for (const [resource, raw] of Object.entries(value)) {
        if (!raw || typeof raw !== "object" || Array.isArray(raw) || !exactKeys(raw as Record<string, unknown>, ["key", "work"]) || typeof raw.key !== "string" || typeof raw.work !== "string")
            throw new UiError(uiMessage("hostedCitations.inputCacheUnparseable"));
        drafts[resource] = { key: raw.key, work: raw.work };
    }
    return drafts;
}
export type CitationProposal = {
    resource: string;
    serial: number;
    sourceHash: string;
    patchUnit: "utf16";
    source: string;
    patches: {
        from: number;
        to: number;
        text: string;
        expected: string;
        byteFrom: number;
        byteTo: number;
    }[];
};
export function citationPatches(value: unknown, context: HostedLinksContext, hash: string): SourcePatch[] {
    if (!value || typeof value !== "object" || Array.isArray(value) || !exactKeys(value as Record<string, unknown>, ["resource", "serial", "sourceHash", "patchUnit", "source", "patches"]))
        throw new UiError(uiMessage("hostedCitationState.proposalDraftMismatch"));
    const result = value as CitationProposal;
    if (!context.writable || context.pending || context.compare || result.resource !== context.resource || !Number.isSafeInteger(result.serial) || result.serial !== context.serial || result.sourceHash !== hash || result.patchUnit !== "utf16" || typeof result.source !== "string" || !Array.isArray(result.patches) || !result.patches.length || result.patches.length > 20000)
        throw new UiError(uiMessage("hostedCitationState.proposalDraftMismatch"));
    const encoder = new TextEncoder();
    const patches = result.patches.map(p => {
        if (!p || typeof p !== "object" || Array.isArray(p) || !exactKeys(p as unknown as Record<string, unknown>, ["from", "to", "text", "expected", "byteFrom", "byteTo"]) || !Number.isSafeInteger(p.from) || !Number.isSafeInteger(p.to) || p.from < 0 || p.to < p.from || p.to > context.source.length || !sourceBoundary(context.source, p.from) || !sourceBoundary(context.source, p.to) || typeof p.text !== "string" || typeof p.expected !== "string" || context.source.slice(p.from, p.to) !== p.expected || encoder.encode(context.source.slice(0, p.from)).length !== p.byteFrom || encoder.encode(context.source.slice(0, p.to)).length !== p.byteTo)
            throw new UiError(uiMessage("hostedCitationState.proposalRangeMissing"));
        return { from: p.from, to: p.to, expected: p.expected, insert: p.text };
    });
    if (applySourcePatches(context.source, patches) !== result.source)
        throw new UiError(uiMessage("hostedCitationState.proposalChangesOutsidePatch"));
    return patches;
}
