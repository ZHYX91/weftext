import type { MediaResource, ResourceMetadata } from "./media-viewer";
import type { MediaRegionInput } from "./knowledge-state";
import type { ResourceOpenTarget } from "./media-selection";
import { resourceOpenError, storedMediaSelection } from "./media-selection";
import { validateMediaRegion } from "./knowledge-state";
import type { HostedResourceCollection } from "./hosted-media-import-state";
export { resourceCellReferences as mediaCellReferences } from "./media-selection";

export type MediaPurpose = "view" | "download";
export interface HostedMediaUnavailable { asset: string; owner: string; title: string; name: string; lifecycle: "live" | "trash"; status: "unavailable" }
export interface HostedMediaDescriptor extends Omit<HostedMediaUnavailable, "status"> { status: "ready"; resource: string; revision: string; bytes: number; contentType: string; metadata: ResourceMetadata; annotationReady: boolean; wholeAnnotationReady: boolean; currentness: "accepted"; download: boolean }
export interface HostedMediaCatalog { resources: (HostedMediaDescriptor | HostedMediaUnavailable)[]; collections?: HostedResourceCollection[]; coverage: "complete" | "partial"; scope: "configuredAuthorizedResources" }
export interface HostedMediaResolved extends HostedMediaDescriptor { viewerTarget: ResourceOpenTarget & { fragment?: string; resourceFragment?: { status: "none" | "found" | "missing" | "unavailable"; pageIndex?: number } } }
export interface HostedMediaTicket extends Omit<HostedMediaDescriptor, "download" | "status"> { ticket: string; purpose: MediaPurpose; sha256: string; expiresInMs: number }
export interface HostedMediaChunk { ticket: string; purpose: MediaPurpose; bytesBase64: string; offset: number; length: number; total: number; sha256: string; done: boolean }
export interface HostedMediaSelection { descriptor: HostedMediaDescriptor; target: MediaRegionInput | null }
const safePositive = (n: unknown) => typeof n === "number" && Number.isSafeInteger(n) && n > 0;
export function decodeMediaDescriptor(value: HostedMediaDescriptor | HostedMediaUnavailable): HostedMediaDescriptor | HostedMediaUnavailable {
  if (!value || ![value.asset, value.owner, value.name, value.title].every(item => typeof item === "string" && item.length > 0) || !["live", "trash"].includes(value.lifecycle)) throw new Error("媒体目录缺少独立获权身份，未安装资源。");
  if (value.status === "unavailable") return { asset: value.asset, owner: value.owner, title: value.title, name: value.name, lifecycle: value.lifecycle, status: "unavailable" };
  if (value.status !== "ready" || !/^weftext-resource:\/\//.test(value.resource) || typeof value.revision !== "string" || !value.revision || !Number.isSafeInteger(value.bytes) || value.bytes < 0 || value.bytes > 64 * 1024 * 1024 || typeof value.contentType !== "string" || typeof value.annotationReady !== "boolean" || typeof value.wholeAnnotationReady !== "boolean" || typeof value.download !== "boolean" || value.currentness !== "accepted") throw new Error("资源缺少准确版本、交付长度或权限，未读取替换内容。");
  const metadata = value.metadata;
  if (!metadata || !["image", "pdf", "audio", "video", "unsupported"].includes(metadata.kind)) throw new Error("资源元数据不可用。");
  if (metadata.kind === "image" || metadata.kind === "video") { if (!safePositive(metadata.width) || !safePositive(metadata.height)) throw new Error("资源原始像素尺寸不可用。"); }
  if (metadata.kind === "pdf" && (!Array.isArray(metadata.pages) || !metadata.pages.length || metadata.pages.some(page => !Number.isFinite(page.width) || !Number.isFinite(page.height) || page.width <= 0 || page.height <= 0 || !Number.isSafeInteger(page.rotation)))) throw new Error("准确 PDF 页面坐标不可用。");
  if ((metadata.kind === "audio" || metadata.kind === "video") && (!safePositive(metadata.timebase?.numerator) || !safePositive(metadata.timebase?.denominator) || !safePositive(metadata.durationTick))) throw new Error("媒体采样时钟缺少准确整数，不能猜测时间范围。");
  return structuredClone(value);
}
export function mediaFingerprint(value: HostedMediaDescriptor): string { return JSON.stringify([value.asset, value.owner, value.resource, value.revision, value.name, value.title, value.bytes, value.contentType, value.metadata, value.lifecycle, value.annotationReady, value.wholeAnnotationReady]); }
export function mediaViewerDescriptor(value: HostedMediaDescriptor): MediaResource { return { resourceId: value.asset, resource: value.resource, name: value.name, revision: value.revision, bytes: value.bytes, contentType: value.contentType, metadata: value.metadata, annotationReady: value.annotationReady, currentness: value.currentness }; }
export function mediaTicket(expected: HostedMediaDescriptor, purpose: MediaPurpose, value: HostedMediaTicket): HostedMediaTicket {
  if (!value || typeof value.ticket !== "string" || !value.ticket || value.purpose !== purpose || !/^[a-f0-9]{64}$/.test(value.sha256) || !Number.isSafeInteger(value.expiresInMs) || value.expiresInMs <= 0 || value.expiresInMs > 900000 || value.currentness !== "accepted") throw new Error("资源票据没有准确用途、期限或摘要，未交付字节。");
  const descriptor = decodeMediaDescriptor({ ...value, status: "ready", download: expected.download }); if (descriptor.status !== "ready" || mediaFingerprint(descriptor) !== mediaFingerprint(expected)) throw new Error("媒体读取结果已改变原资源、版本或元数据，未交付替换资源。");
  return structuredClone(value);
}
export function mediaChunk(ticket: HostedMediaTicket, offset: number, value: HostedMediaChunk): Uint8Array {
  const empty = ticket.bytes === 0 && offset === 0 && value?.length === 0 && value.done === true;
  if (!value || !Number.isSafeInteger(offset) || offset < 0 || value.ticket !== ticket.ticket || value.purpose !== ticket.purpose || value.offset !== offset || value.total !== ticket.bytes || value.sha256 !== ticket.sha256 || !Number.isSafeInteger(value.length) || (value.length <= 0 && !empty) || value.length > 65536 || offset + value.length > ticket.bytes || value.done !== (offset + value.length === ticket.bytes) || typeof value.bytesBase64 !== "string" || value.bytesBase64.length > 87384) throw new Error("资源块不对应原票据、用途或准确版本，未交付文件。");
  const bytes = Uint8Array.from(atob(value.bytesBase64), char => char.charCodeAt(0)); if (bytes.length !== value.length) throw new Error("资源块长度不完整。"); return bytes;
}
export function mediaResolved(value: HostedMediaResolved, reference: string, revision?: string): HostedMediaResolved {
  if (typeof reference !== "string" || !/^weftext-resource:\/\//.test(reference)) throw new Error("这里只接受稳定资源引用，不按本地路径猜测目标。");
  const descriptor = decodeMediaDescriptor(value); if (descriptor.status !== "ready" || revision !== undefined && descriptor.revision !== revision || value.viewerTarget?.resource !== descriptor.resource || value.viewerTarget?.revision !== descriptor.revision) throw new Error("资源链接未解析到原准确版本，未猜测最新版本。");
  if (reference.split("#", 1)[0] !== descriptor.resource) throw new Error("解析结果不对应原稳定资源引用，未打开另一资源。");
  const target = value.viewerTarget, pageIndex = target.resourceFragment?.status === "found" ? target.resourceFragment.pageIndex : undefined;
  if (["missing", "unavailable"].includes(target.resourceFragment?.status ?? "") || reference.includes("#") && (target.resourceFragment?.status !== "found" || !Number.isSafeInteger(pageIndex))) throw new Error("链接的准确资源位置当前不可用；未退回首页或其他位置。");
  const error = resourceOpenError({ resource: target.resource, revision: target.revision, pageIndex }, mediaViewerDescriptor(descriptor)); if (error) throw new Error(error);
  return { ...descriptor, viewerTarget: { ...target, pageIndex } };
}
export function mediaIntentTarget(selection: HostedMediaSelection, whole = false): Record<string, unknown> {
  const { descriptor, target } = selection; if (descriptor.lifecycle !== "live" || !(whole ? descriptor.wholeAnnotationReady : descriptor.annotationReady)) throw new Error("当前资源没有可写批注目标资格；回收站资源仅供查看。");
  if (whole) return { kind: "whole_resource", asset: descriptor.asset, revision: descriptor.revision };
  if (!target || validateMediaRegion(target) || target.resource !== descriptor.resource || target.revision !== descriptor.revision) throw new Error("请在真实准确媒体版本中选择有效目标，原批注输入保留。");
  const metadata = descriptor.metadata;
  const facts = target.kind === "image_region" ? metadata.kind === "image" && target.width === metadata.width && target.height === metadata.height
    : target.kind === "pdf_region" ? metadata.kind === "pdf" && target.pageIndex !== undefined && metadata.pages[target.pageIndex] && target.width === metadata.pages[target.pageIndex].width && target.height === metadata.pages[target.pageIndex].height && target.rotation === metadata.pages[target.pageIndex].rotation
    : (metadata.kind === "audio" || metadata.kind === "video") && target.timebase === metadata.timebase.numerator && (target.timebaseDenominator ?? 1) === metadata.timebase.denominator && target.durationTick === metadata.durationTick && (metadata.kind !== "video" || target.width === metadata.width && target.height === metadata.height);
  if (!facts) throw new Error("媒体目标的像素、页面或时钟依据与准确资源不一致，未提交猜测坐标。");
  const { resource: _, ...geometry } = target; return { ...geometry, asset: descriptor.asset };
}
export function mediaAnnotationSelection(target: unknown): MediaRegionInput | null { return storedMediaSelection(target); }
