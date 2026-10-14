/** Presentation state only. Core owns discovery, identities and write qualification. */
export interface KnowledgeNodeRef { workspaceId: string; nodeId: string; title?: string; path?: string }
export interface KnowledgeMention {
  startScalar: number;
  endScalar: number;
  startUtf16: number;
  endUtf16: number;
  targets: KnowledgeNodeRef[];
  terms: string[];
  writable: boolean;
  lane?: 0 | 1;
  component: number;
}
export const nodeIdentity = (ref: KnowledgeNodeRef): string => `${ref.workspaceId}/${ref.nodeId}`;

export interface KnowledgeLoss { code: string; message: string }
export function knowledgeLossMessage(loss: string | KnowledgeLoss): string { return typeof loss === "string" ? loss : loss.message; }
export function uncertainKnowledgeOperation(status: string): boolean {
  return ["unknown", "pending", "resultUnknown", "in-progress", "pending_reconciliation", "recoveryPending"].includes(status);
}

/** Draft edits have no server operation to reconcile, even if local persistence fails. */
export function retainPendingKnowledgePlan(draftOnly: boolean, error: unknown): boolean {
  if (draftOnly) return false;
  const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
  return !code || uncertainKnowledgeOperation(code);
}

/** Same-span targets merge; nested and crossing spans survive. No overlap graph. */
export function layoutMentions(hits: readonly KnowledgeMention[]): KnowledgeMention[] {
  const grouped = new Map<string, KnowledgeMention>();
  for (const hit of hits) {
    if (!Number.isInteger(hit.startScalar) || !Number.isInteger(hit.endScalar) || hit.startScalar < 0 || hit.endScalar <= hit.startScalar) continue;
    const key = `${hit.startScalar}:${hit.endScalar}`;
    const previous = grouped.get(key);
    if (!previous) grouped.set(key, { ...hit, targets: [...hit.targets], terms: [...hit.terms], lane: undefined });
    else {
      previous.targets.push(...hit.targets);
      previous.terms.push(...hit.terms);
      previous.writable = previous.writable && hit.writable && previous.startUtf16 === hit.startUtf16 && previous.endUtf16 === hit.endUtf16;
    }
  }
  const sorted = [...grouped.values()].sort((a, b) => a.startScalar - b.startScalar || b.endScalar - a.endScalar);
  let component = -1, componentEnd = -1;
  const ends = [-1, -1];
  for (const hit of sorted) {
    if (hit.startScalar >= componentEnd) { component++; ends.fill(-1); }
    componentEnd = Math.max(componentEnd, hit.endScalar);
    hit.component = component;
    const lane = ends.findIndex(end => end <= hit.startScalar);
    if (lane >= 0) { hit.lane = lane as 0 | 1; ends[lane] = hit.endScalar; }
    const targets = new Map(hit.targets.map(ref => [nodeIdentity(ref), ref]));
    hit.targets = [...targets.values()].sort((a, b) => nodeIdentity(a).localeCompare(nodeIdentity(b), "en"));
    hit.terms = [...new Set(hit.terms)];
  }
  return sorted;
}

export interface KnowledgeBasis { sessionId: string; path: string; source: string; serial: number; baseId: string }
export function sameKnowledgeBasis(a: KnowledgeBasis | null, b: KnowledgeBasis | null): boolean {
  return a !== null && b !== null && a.sessionId === b.sessionId && a.path === b.path && a.serial === b.serial && a.baseId === b.baseId && a.source === b.source;
}

/** A late background read must never replace a newer document or draft. */
export class KnowledgeReadGate {
  private generation = 0;
  begin(): number { return ++this.generation; }
  accepts(generation: number, captured: KnowledgeBasis, current: KnowledgeBasis | null): boolean {
    return generation === this.generation && sameKnowledgeBasis(captured, current);
  }
  invalidate(): void { this.generation++; }
}

export type AnnotationResolution = "exact" | "mapped" | "stale" | "ambiguous" | "orphaned" | "unavailable";
export function canApplySuggestion(state: { purpose?: string; suggestionState?: string; reviewState?: string; targetResolution?: AnnotationResolution; verified?: boolean }): boolean {
  return state.purpose === "suggestion" && state.suggestionState === "pending" && state.verified !== false && (state.targetResolution === "exact" || state.targetResolution === "mapped");
}

export function isGraphemeRange(text: string, start: number, end: number, allowPoint = false): boolean {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start || !allowPoint && end === start || end > text.length) return false;
  const boundaries = new Set<number>([text.length]);
  for (const part of new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text)) boundaries.add(part.index);
  return boundaries.has(start) && boundaries.has(end);
}

export interface MediaRegionInput {
  kind: "pdf_region" | "image_region" | "media_interval";
  resource: string;
  revision: string;
  pageIndex?: number;
  rotation?: number;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  rectWidth?: number;
  rectHeight?: number;
  startTick?: number;
  endTick?: number;
  timebase?: number;
  timebaseDenominator?: number;
  durationTick?: number;
  mediaType?: "audio" | "video";
}
/** Validate entered units without rounding, normalizing or fetching the resource. */
export function validateMediaRegion(value: MediaRegionInput): string | null {
  if (!value.resource.trim() || !value.revision.trim()) return "请输入准确资源地址和版本。";
  if (value.kind === "media_interval") {
    if (![value.startTick, value.endTick, value.timebase].every(n => typeof n === "number" && Number.isSafeInteger(n))) return "时间端点和 timebase 必须是准确整数。";
    if (value.startTick! < 0 || value.endTick! <= value.startTick! || value.timebase! <= 0) return "结束 tick 必须晚于开始，timebase 必须为正。";
    if (value.timebaseDenominator !== undefined && (!Number.isSafeInteger(value.timebaseDenominator) || value.timebaseDenominator <= 0)) return "时间单位分母必须是准确正整数。";
    if (value.durationTick !== undefined && (!Number.isSafeInteger(value.durationTick) || value.durationTick < value.endTick!)) return "所选时间区间必须完整位于准确资源时长内。";
    if (value.mediaType === "audio") return null;
  }
  if (value.kind === "pdf_region" && (!Number.isSafeInteger(value.pageIndex) || value.pageIndex! < 0)) return "PDF 页索引必须从 0 开始。";
  if (![value.width, value.height, value.x, value.y, value.rectWidth, value.rectHeight].every(n => typeof n === "number" && Number.isFinite(n))) return "请输入页面或图像尺寸与完整矩形。";
  if (value.kind !== "pdf_region" && ![value.width, value.height, value.x, value.y, value.rectWidth, value.rectHeight].every(Number.isSafeInteger)) return "图片和视频矩形须使用原始像素的准确整数边界。";
  if (value.width! <= 0 || value.height! <= 0 || value.x! < 0 || value.y! < 0 || value.rectWidth! <= 0 || value.rectHeight! <= 0 || value.x! + value.rectWidth! > value.width! || value.y! + value.rectHeight! > value.height!) return "矩形必须完整位于固定尺寸范围内。";
  return null;
}
