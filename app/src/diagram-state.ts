/** Mermaid is author source plus a derived PNG. Only source is editable in UI.
 * The confirmed literal is an output witness, not another authoring surface:
 * native Typst refuses its PNG as soon as the author source differs. */
import type { DocumentBasis, SourcePatch } from "./data-panel";

export const DIAGRAM_SOURCE_LIMIT = 32_768;
export const DIAGRAM_PNG_LIMIT = 256 * 1024;
export const DOCUMENT_SOURCE_LIMIT = 2 * 1024 * 1024;
export interface DiagramImage { source: string; png: Uint8Array; width: number; height: number }
export interface DiagramOccurrence { id: string; from: number; to: number; source: string; confirmed: string; png: Uint8Array; width: number; height: number; stale: boolean }
const marker = "// weftext-mermaid v1 ";
const literal = '"(?:\\\\.|[^"\\\\])*"';
const idPattern = "[a-f0-9-]{36}";
const header = new RegExp(`^${marker}(${idPattern})\\r?\\n#\\{\\r?\\n  let source = (${literal})\\r?\\n  let confirmed = (${literal})\\r?\\n`);

export function validateDiagramSource(source: string): void {
  if (!source.trim() || source.length > DIAGRAM_SOURCE_LIMIT) throw new Error("图源须为 1–32768 个字符；输入已保留。");
  if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u.test(source)) throw new Error("图源包含不支持的控制字符。");
  // The offline profile has no external resources or per-diagram settings.
  if (/(?:\b(?:https?|file|data|javascript):|\/\/|%%\{|^\s*---\s*$|<\/?[a-z!]|@import|\bclick\s)/imu.test(source)) throw new Error("离线图表暂不接受外部资源、HTML、点击动作或图内配置；请保留普通 Mermaid 图源。");
}
export function validatePng(png: Uint8Array, width: number, height: number): void {
  if (png.length < 24 || png.length > DIAGRAM_PNG_LIMIT || ![137,80,78,71,13,10,26,10].every((value, i) => png[i] === value)) throw new Error("没有取得受支持大小的准确 PNG；请缩小图表后重试。");
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 1 || height < 1 || width > 4096 || height > 4096 || width * height > 8_388_608 || view.getUint32(16) !== width || view.getUint32(20) !== height) throw new Error("PNG 尺寸与本次预览不一致或超出限制。");
}
export function diagramBlock(id: string, image: DiagramImage, confirmed = image.source): string {
  validateDiagramSource(image.source); validateDiagramSource(confirmed);
  return serializeDiagramBlock(id,image,confirmed);
}
function serializeDiagramBlock(id: string, image: DiagramImage, confirmed: string): string {
  if (!new RegExp(`^${idPattern}$`).test(id)) throw new Error("图表标识无效。");
  validatePng(image.png, image.width, image.height);
  const hex = Array.from(image.png, byte => byte.toString(16).padStart(2, "0")).join("");
  return `${marker}${id}\n#{\n  let source = ${JSON.stringify(image.source)}\n  let confirmed = ${JSON.stringify(confirmed)}\n  assert(source == confirmed, message: "Mermaid 图源已改变，请重新生成并确认图片。")\n  // derived PNG ${image.width} ${image.height}; edit source through Mermaid controls\n  image(bytes("${hex}".matches(regex("..")).map(x => int(x.text, base: 16))), format: "png", width: 100%, alt: source)\n}\n// weftext-mermaid end ${id}`;
}
/** Only our exact bounded native block is writable; arbitrary Typst stays intact. */
export function diagramOccurrences(document: string): DiagramOccurrence[] {
  const result: DiagramOccurrence[] = []; let cursor = 0;
  while ((cursor = document.indexOf(marker, cursor)) >= 0) {
    const from = cursor; cursor += marker.length; const match = header.exec(document.slice(from)); if (!match) continue;
    const end = document.indexOf(`// weftext-mermaid end ${match[1]}`, from + match[0].length); if (end < 0) continue;
    const to = end + `// weftext-mermaid end ${match[1]}`.length; const block = document.slice(from, to);
    try {
      const source = JSON.parse(match[2]) as string, confirmed = JSON.parse(match[3]) as string;
      const data = /\/\/ derived PNG (\d+) (\d+); edit source through Mermaid controls\r?\n  image\(bytes\("([a-f0-9]+)"\.matches\(regex\("\.\."\)\)\.map\(x => int\(x\.text, base: 16\)\)\), format: "png", width: 100%, alt: source\)/u.exec(block);
      if (!data || data[3].length > DIAGRAM_PNG_LIMIT * 2 || data[3].length % 2) continue;
      const png = Uint8Array.from(data[3].match(/../g)!.map(value => parseInt(value, 16))); const width = +data[1], height = +data[2];
      if (serializeDiagramBlock(match[1], { source, png, width, height }, confirmed) !== block.replaceAll("\r\n", "\n")) continue;
      result.push({ id: match[1], from, to, source, confirmed, png, width, height, stale: source !== confirmed }); cursor = to;
    } catch { /* Preserve unrecognized author source without guessing a range. */ }
  }
  return result;
}
export function diagramPatch(basis: DocumentBasis, image: DiagramImage, occurrence?: DiagramOccurrence): SourcePatch {
  if (occurrence && !diagramOccurrences(basis.source).some(item => item.id === occurrence.id && item.from === occurrence.from && item.to === occurrence.to && item.source === occurrence.source && item.confirmed === occurrence.confirmed)) throw new Error("图表来源已改变；输入保留，请重新打开当前图表。");
  const block = diagramBlock(occurrence?.id ?? crypto.randomUUID(), image);
  const patch = occurrence ? { from: occurrence.from, to: occurrence.to, text: block } : { from: basis.source.length, to: basis.source.length, text: `${basis.source.endsWith("\n") || !basis.source ? "" : "\n"}\n${block}\n` };
  const next = basis.source.slice(0, patch.from) + patch.text + basis.source.slice(patch.to);
  if (new TextEncoder().encode(next).length > DOCUMENT_SOURCE_LIMIT) throw new Error("插入后将超过 2 MiB 草稿上限；请缩小图表。输入及原稿保留。");
  return patch;
}
export interface DiagramRenderer { (source: string, width: number): Promise<DiagramImage> }
/** Editing, cancel and late results never leave an older image eligible. */
export class DiagramController {
  source: string; width = 1200; image: DiagramImage | null = null; busy = false; private generation = 0;
  constructor(readonly renderer: DiagramRenderer, source = "flowchart LR\n  A[开始] --> B[完成]") { this.source = source; }
  edit(source: string, width = this.width): void { this.source = source; this.width = width; this.generation++; this.image = null; this.busy = false; }
  cancel(): void { this.generation++; this.busy = false; this.image = null; }
  async render(): Promise<boolean> {
    validateDiagramSource(this.source); const source = this.source, width = this.width, generation = ++this.generation; this.image = null; this.busy = true;
    try { const result = await this.renderer(source, width); if (generation !== this.generation) return false; if (result.source !== source) throw new Error("图片与本次图源不一致；请重新生成。"); validatePng(result.png, result.width, result.height); this.image = result; return true; }
    catch (error) { if (generation !== this.generation) return false; throw error; }
    finally { if (generation === this.generation) this.busy = false; }
  }
}
