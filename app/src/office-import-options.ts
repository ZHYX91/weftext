import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError } from "./ui-locale";
import type { UiText } from "./ui-locale";
import type { DocxImportAnalysis, DocxImportOptions, LegacyCapabilities } from "./export-state";
export const captionKinds: [
    string,
    UiText
][] = [["figure", uiMessage("documentNotes.figureCategory")], ["table", uiMessage("documentNotes.tableCategory")], ["equation", uiMessage("shell.formatEquation")]];
export const kindLabel = (kind: string): UiText => captionKinds.find(item => item[0] === kind)?.[1] ?? kind;
export const backendLabel = (id: string): UiText => ({ libreoffice: "LibreOffice", word: "Microsoft Word", wps: "WPS Office", configured: uiMessage("officeImportOptions.configuredExternalSoftware") } as Record<string, UiText>)[id] ?? id;
/** Legacy Word output still inspects its exact DOCX template before conversion. */
export function templateInspectionFormat(outputFormat: string): "docx" | "xlsx" {
    if (outputFormat === "xlsx") return "xlsx";
    if (["docx", "doc", "wps"].includes(outputFormat)) return "docx";
    throw new UiError(uiMessage("error.invalidResponse"));
}
export function legacyBackendChoices(capabilities: LegacyCapabilities | null, format: string, importing: boolean): [
    string,
    UiText
][] {
    const direction = format === "doc" ? importing ? "docToDocx" : "docxToDoc" : format === "wps" ? importing ? "wpsToDocx" : "docxToWps" : "";
    return capabilities?.backends.filter(backend => backend.directions.some(item => item.direction === direction && item.available)).map(backend => [backend.id, () => `${uiTextValue(backendLabel(backend.id))}${backend.version ? ` · ${backend.version}` : ""}`]) ?? [];
}
export interface CaptionChoice {
    enabled: boolean;
    captionBlock: number;
    objectBlock: string;
    kind: string;
    captionText: string;
    position: string;
}
/** Pairings require an explicit choice and are bound to the inspected original. */
export function selectedCaptionOptions(analysis: DocxImportAnalysis | null, choices: CaptionChoice[]): DocxImportOptions | undefined {
    const selected = choices.filter(choice => choice.enabled);
    if (!selected.length)
        return undefined;
    if (!analysis || !/^[a-f0-9]{64}$/i.test(analysis.contentHash))
        throw new UiError(uiMessage("officeImportOptions.captionInspectionExpired"));
    const used = new Set<number>();
    const captionPairings = selected.map(choice => {
        const objectBlock = Number(choice.objectBlock);
        if (!choice.objectBlock || !Number.isSafeInteger(objectBlock) || !analysis.captions.some(item => item.blockIndex === choice.captionBlock) || !analysis.objects.some(item => item.blockIndex === objectBlock && item.kind === choice.kind) || !["top", "bottom"].includes(choice.position) || !choice.captionText.trim())
            throw new UiError(uiMessage("officeImportOptions.captionPairingRequired"));
        if (used.has(choice.captionBlock) || used.has(objectBlock) || choice.captionBlock === objectBlock)
            throw new UiError(uiMessage("officeImportOptions.captionObjectPairOnce"));
        used.add(choice.captionBlock);
        used.add(objectBlock);
        return { captionBlock: choice.captionBlock, objectBlock, kind: choice.kind, captionText: choice.captionText, position: choice.position };
    });
    return { contentHash: analysis.contentHash, captionPairings };
}
export function validDocxAnalysis(analysis: DocxImportAnalysis): boolean {
    return !!analysis && /^[a-f0-9]{64}$/i.test(analysis.contentHash) && Array.isArray(analysis.captions) && Array.isArray(analysis.objects) && !analysis.captions.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || typeof item.sequence !== "string" || typeof item.text !== "string" || typeof item.suggestedText !== "string") && !analysis.objects.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || !captionKinds.some(kind => kind[0] === item.kind));
}
