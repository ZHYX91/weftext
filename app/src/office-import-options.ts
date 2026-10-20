import type { DocxImportAnalysis, DocxImportOptions, LegacyCapabilities } from "./export-state";

export const captionKinds: [string, string][] = [["figure", "图"], ["table", "表"], ["equation", "公式"]];
export const kindLabel = (kind: string): string => captionKinds.find(item => item[0] === kind)?.[1] ?? kind;
export const backendLabel = (id: string): string => ({ libreoffice: "LibreOffice", word: "Microsoft Word", wps: "WPS Office", configured: "已配置的外部软件" } as Record<string, string>)[id] ?? id;
export function legacyBackendChoices(capabilities: LegacyCapabilities | null, format: string, importing: boolean): [string, string][] {
  const direction = format === "doc" ? importing ? "docToDocx" : "docxToDoc" : format === "wps" ? importing ? "wpsToDocx" : "docxToWps" : "";
  return capabilities?.backends.filter(backend => backend.directions.some(item => item.direction === direction && item.available)).map(backend => [backend.id, `${backendLabel(backend.id)}${backend.version ? ` · ${backend.version}` : ""}`]) ?? [];
}
export interface CaptionChoice { enabled: boolean; captionBlock: number; objectBlock: string; kind: string; captionText: string; position: string }
/** Pairings require an explicit choice and are bound to the inspected original. */
export function selectedCaptionOptions(analysis: DocxImportAnalysis | null, choices: CaptionChoice[]): DocxImportOptions | undefined {
  const selected = choices.filter(choice => choice.enabled); if (!selected.length) return undefined;
  if (!analysis || !/^[a-f0-9]{64}$/i.test(analysis.contentHash)) throw new Error("题注原件检查已失效，请重新检查。");
  const used = new Set<number>();
  const captionPairings = selected.map(choice => {
    const objectBlock = Number(choice.objectBlock);
    if (!choice.objectBlock || !Number.isSafeInteger(objectBlock) || !analysis.captions.some(item => item.blockIndex === choice.captionBlock) || !analysis.objects.some(item => item.blockIndex === objectBlock && item.kind === choice.kind) || !["top", "bottom"].includes(choice.position) || !choice.captionText.trim()) throw new Error("请为已确认的每条题注明确选择对象、同类类别、题注正文及显示位置。");
    if (used.has(choice.captionBlock) || used.has(objectBlock) || choice.captionBlock === objectBlock) throw new Error("每条题注和对象只能配对一次，请检查重复选择。");
    used.add(choice.captionBlock); used.add(objectBlock);
    return { captionBlock: choice.captionBlock, objectBlock, kind: choice.kind, captionText: choice.captionText, position: choice.position };
  });
  return { contentHash: analysis.contentHash, captionPairings };
}
export function validDocxAnalysis(analysis: DocxImportAnalysis): boolean {
  return !!analysis && /^[a-f0-9]{64}$/i.test(analysis.contentHash) && Array.isArray(analysis.captions) && Array.isArray(analysis.objects) && !analysis.captions.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || typeof item.sequence !== "string" || typeof item.text !== "string" || typeof item.suggestedText !== "string") && !analysis.objects.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || !captionKinds.some(kind => kind[0] === item.kind));
}
