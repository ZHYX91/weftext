import type { HostedDraft } from "./hosted-state";
import { RecoveryTextCache, type StoragePort } from "./recovery-text-cache";

export class HostedDraftCache extends RecoveryTextCache<HostedDraft> {
  constructor(storage: StoragePort, audiencePrefix: () => string) {
    super(storage, () => { const prefix = audiencePrefix(); if (!prefix.startsWith("weftext-hosted-draft:")) throw invalid(); return prefix; }, mapDraftTexts, prefix => prefix.replace(/^weftext-hosted-draft:/, "weftext-hosted-draft-text:"));
  }
}

const invalid = () => new Error("浏览器恢复稿无法完整读取；原记录保留，未采用另一份正文。");
function mapDraftTexts(value: unknown, convert: (text: unknown, field: string) => unknown): unknown {
    if (!value || typeof value !== "object" || Array.isArray(value)) throw invalid();
    const draft = value as HostedDraft;
    const mapped = { ...draft, source: convert(draft.source, "source"), baseSource: convert(draft.baseSource, "baseSource") };
    if (draft.pending === undefined) return mapped;
    if (!draft.pending || typeof draft.pending !== "object" || typeof draft.pending.operationId !== "string") throw invalid();
    const pending = { ...draft.pending, source: convert(draft.pending.source, "pending") };
    if (draft.pending.request) pending.request = { ...draft.pending.request, source: convert(draft.pending.request.source, "request") } as typeof draft.pending.request;
    return { ...mapped, pending };
  }
