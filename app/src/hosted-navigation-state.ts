import { sourceBoundary } from "./editor";
import type { Origin } from "./types";
import { draftDisposition, type HostedDraft } from "./hosted-state";

export interface NodeFragment { label: string; status: "found" | "missing" | "ambiguous" | "unavailable"; basis: "saved" | "draft"; sourceHash: string; origin?: Origin }
export interface HostedNodeResolution { resource: string; reference: string; canOpen: boolean; fragment?: NodeFragment }
export interface HostedNodeBasis { sessionId: string; source: string; serial: number; sourceHash: string }
export interface HostedNodeFragmentResult { resource: string; reference: string; serial: number; sourceHash: string; fragment: NodeFragment }
export interface HostedNodeAudience { epoch: number; authorization: number; credential: string; actor?: string; deployment?: string; generation?: number; sessionId?: string; source: string; serial: number; navigation: number }
export function sameHostedNodeAudience(left: HostedNodeAudience, right: HostedNodeAudience): boolean {
  return left.epoch === right.epoch && left.authorization === right.authorization && left.credential === right.credential && left.actor === right.actor && left.deployment === right.deployment && left.generation === right.generation && left.sessionId === right.sessionId && left.source === right.source && left.serial === right.serial && left.navigation === right.navigation;
}
/** The base is only a candidate for a later independently authorized action. */
export function nodeReferenceParts(reference: string): { base: string; fragment?: string } {
  if (!reference.startsWith("weftext-node://")) throw new Error("不是稳定笔记地址，未按文件路径打开。");
  const at = reference.indexOf("#"); if (at < 0) return { base: reference };
  const base = reference.slice(0, at), encoded = reference.slice(at + 1);
  let fragment: string; try { fragment = decodeURIComponent(encoded); } catch { throw new Error("片段标签的编码无效，未丢弃片段打开整篇。"); }
  if (!fragment || encoded.includes("#") || /[\u0000-\u001f\u007f]/.test(fragment)) throw new Error("片段标签为空或无效，未丢弃片段打开整篇。");
  return { base, fragment };
}
export function nodeTargetDraft(saved: string, recovered: string | undefined, recoveredBase: string | undefined, local?: HostedDraft): HostedDraft {
  if (local !== undefined && (!local || typeof local.source !== "string" || typeof local.baseSource !== "string")) throw new Error("目标的本页恢复稿无法准确读取；原记录保留，未安装另一份正文。");
  const candidate = local ? structuredClone(local) : { source: recovered ?? saved, baseSource: recoveredBase ?? saved };
  if (candidate.pending) throw new Error("目标的原保存操作尚未核对；请明确仅打开笔记后按原编号恢复，未暗中结束操作或定位旧范围。");
  return draftDisposition(saved, candidate) === "current" ? { ...candidate, source: saved, baseSource: saved } : candidate;
}
export function nodeFragmentOrigin(result: HostedNodeFragmentResult, resource: string, reference: string, basis: HostedNodeBasis): Origin | undefined {
  const expected = nodeReferenceParts(reference);
  if (result.resource !== resource || result.reference !== reference || result.serial !== basis.serial || result.sourceHash !== basis.sourceHash || result.fragment?.sourceHash !== basis.sourceHash || result.fragment?.basis !== "draft" || result.fragment.label !== expected.fragment || !["found", "missing", "ambiguous", "unavailable"].includes(result.fragment.status)) throw new Error("片段定位回执与准确当前草稿不符，未采用旧范围。");
  if (result.fragment.status !== "found") return;
  const origin = result.fragment.origin;
  if (!origin || origin.isMain !== true || origin.path !== `resource:${resource}` || !Number.isSafeInteger(origin.utf16From) || !Number.isSafeInteger(origin.utf16To) || origin.utf16From < 0 || origin.utf16To <= origin.utf16From || origin.utf16To > basis.source.length || !sourceBoundary(basis.source, origin.utf16From) || !sourceBoundary(basis.source, origin.utf16To) || basis.source.slice(origin.utf16From, origin.utf16To) !== origin.raw) throw new Error("标签没有准确主稿范围，未按显示文字或旧源码猜测位置。");
  return origin;
}
export async function nodeSourceHash(source: string): Promise<string> { return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(byte => byte.toString(16).padStart(2, "0")).join(""); }
