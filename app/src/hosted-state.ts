// Disconnected drafts carry their actual base. Reopening never silently adopts
// a new base for an older proposal, including after a lost save response.
export interface HostedDraft { source: string; baseSource: string; pending?: { operationId: string; source: string } }
export function draftDisposition(saved: string, cached?: HostedDraft): "current" | "restore" | "compare" | "unknown" {
  if (cached?.pending) return "unknown";
  if (!cached || cached.source === saved || cached.source === cached.baseSource) return "current";
  return cached.baseSource === saved ? "restore" : "compare";
}
export function applySaveResult(draft: HostedDraft, saved: string): HostedDraft {
  // Typing after dispatch must not be replaced by the older committed proposal.
  return { source: draft.source, baseSource: saved };
}
