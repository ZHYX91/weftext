// Disconnected drafts carry their actual base. Reopening never silently adopts
// a new base for an older proposal, including after a lost save response.
export interface HostedDraft { source: string; baseSource: string; pending?: { operationId: string; source: string } }
/** Server actor epochs partition every recoverable protected consumer cache. */
export function hostedCachePrefix(kind: "draft" | "field" | "review" | "office" | "query", deployment: string, actor: string, generation: number): string {
  if (!deployment || !actor || !Number.isSafeInteger(generation) || generation < 0) throw new Error("服务端授权版本无效，未恢复旧缓存。");
  return `weftext-hosted-${kind}:${encodeURIComponent(deployment)}:${encodeURIComponent(actor)}:g${generation}:`;
}
export function draftDisposition(saved: string, cached?: HostedDraft): "current" | "restore" | "compare" | "unknown" {
  if (cached?.pending) return "unknown";
  if (!cached || cached.source === saved || cached.source === cached.baseSource) return "current";
  return cached.baseSource === saved ? "restore" : "compare";
}
export function applySaveResult(draft: HostedDraft, saved: string): HostedDraft {
  // Typing after dispatch must not be replaced by the older committed proposal.
  return { source: draft.source, baseSource: saved };
}
