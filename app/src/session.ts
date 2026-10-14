import type { OpenedDocument, RenderOutput } from "./types";

/** Draft, save base, and projection freshness stay independent. */
export class DocumentSession {
  source: string;
  baseSource: string;
  baseId: string;
  path: string;
  serial = 0;
  projection: RenderOutput | null = null;
  savePending = false;
  lastDraftSource: string | null = null;
  lastDraftSerial = -1;
  error: string | null = null;
  recovered: boolean;

  constructor(public opened: OpenedDocument) {
    this.path = opened.path;
    this.baseId = opened.baseId;
    this.baseSource = opened.source;
    this.source = opened.recoveredDraft ?? opened.source;
    this.recovered = opened.recoveredDraft != null && opened.recoveredDraft !== opened.source;
  }
  get dirty(): boolean { return this.source !== this.baseSource; }
  get projectionCurrent(): boolean { return this.projection?.serial === this.serial && this.projection.source === this.source; }
  edit(source: string): void {
    if (source !== this.source) {
      this.source = source;
      this.serial++;
      this.error = null;
    }
  }
  acceptProjection(result: RenderOutput): boolean {
    if (result.serial !== this.serial || result.source !== this.source) return false;
    this.projection = result;
    return true;
  }
  acknowledgeSave(savedSource: string, baseId: string, path = this.path): void {
    this.baseSource = savedSource;
    this.baseId = baseId;
    this.path = path;
    this.recovered = false;
    this.error = null;
  }
  acknowledgeDraft(source: string, serial: number): void {
    if (serial < this.lastDraftSerial) return;
    this.lastDraftSerial = serial;
    this.lastDraftSource = source;
  }
}

/** Conservative native literal escaping; source-mode paste remains exact. */
export function encodeLiteral(text: string): string {
  return text.replace(/[\\#\[\]{}$*_<>@`=+\-/:~"']/g, "\\$&");
}

export function validLiteralRange(source: string, from: number, to: number, raw: string): boolean {
  return Number.isInteger(from) && Number.isInteger(to) && from >= 0 && to >= from && to <= source.length && source.slice(from, to) === raw;
}

export interface PendingLiteralProposal { source: string; from: number; to: number; raw: string; text: string; nodeKey: string }
export function recoverySource(source: string, pending?: PendingLiteralProposal | null): string {
  if (!pending || pending.source !== source || !validLiteralRange(source, pending.from, pending.to, pending.raw)) return source;
  const escaped = encodeLiteral(pending.text);
  const replacement = pending.nodeKey === "@body" ? `${pending.from > 0 ? "\n\n" : ""}${escaped}\n` : escaped;
  return source.slice(0, pending.from) + replacement + source.slice(pending.to);
}

export function canRebindSaveAs(savedSource: string, opened: OpenedDocument): boolean {
  return opened.source === savedSource && !opened.recoveredDraftConflict;
}
