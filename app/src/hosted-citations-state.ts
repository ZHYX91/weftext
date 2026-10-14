import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
import type { HostedLinksContext } from "./hosted-knowledge-links-state";

export type CitationWork = { resource: string; title: string; reference?: string; status: "ready" | "unavailable"; export?: boolean; download?: boolean };
export type CitationDraft = { key: string; work: string };
export type CitationProposal = { resource: string; serial: number; sourceHash: string; patchUnit: "utf16"; source: string; patches: { from: number; to: number; text: string; expected: string; byteFrom: number; byteTo: number }[] };
export function citationPatches(result: CitationProposal, context: HostedLinksContext, hash: string): SourcePatch[] {
  if (!context.writable || context.pending || context.compare || !result || result.resource !== context.resource || result.serial !== context.serial || result.sourceHash !== hash || result.patchUnit !== "utf16" || typeof result.source !== "string" || !Array.isArray(result.patches) || !result.patches.length || result.patches.length > 8) throw new Error("引用提案不属于当前准确可写草稿，未应用修改。");
  const encoder = new TextEncoder();
  const patches = result.patches.map(p => {
    if (!p || !Number.isSafeInteger(p.from) || !Number.isSafeInteger(p.to) || p.from < 0 || p.to < p.from || p.to > context.source.length || !sourceBoundary(context.source, p.from) || !sourceBoundary(context.source, p.to) || typeof p.text !== "string" || typeof p.expected !== "string" || context.source.slice(p.from, p.to) !== p.expected || encoder.encode(context.source.slice(0, p.from)).length !== p.byteFrom || encoder.encode(context.source.slice(0, p.to)).length !== p.byteTo) throw new Error("引用提案没有准确原文或 UTF-16/UTF-8 范围，未应用修改。");
    return { from: p.from, to: p.to, expected: p.expected, insert: p.text };
  });
  if (applySourcePatches(context.source, patches) !== result.source) throw new Error("引用提案改动了获证补丁之外的原文，未应用修改。");
  return patches;
}
