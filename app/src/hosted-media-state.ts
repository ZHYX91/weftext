import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { MediaResource, ResourceMetadata } from "./media-viewer";
import type { MediaRegionInput } from "./knowledge-state";
import type { ResourceOpenTarget } from "./media-selection";
import { resourceOpenError, storedMediaSelection } from "./media-selection";
import { validateMediaRegion } from "./knowledge-state";
import { decodeResourceMetadata, mediaRecord, mediaInteger, invalidMediaResponse } from "./media-wire";
import { exactKeys } from "./ui-diagnostics";
import type { HostedResourceCollection } from "./hosted-media-import-state";
export { resourceCellReferences as mediaCellReferences } from "./media-selection";
export type MediaPurpose = "view" | "download";
export interface HostedMediaUnavailable {
    asset: string;
    owner?: string;
    title?: string;
    name?: string;
    lifecycle?: "live" | "trash";
    collection?: string;
    status: "unavailable";
}
export interface HostedMediaDescriptor extends Omit<HostedMediaUnavailable, "status"> {
    status: "ready";
    owner: string;
    title: string;
    name: string;
    lifecycle: "live" | "trash";
    resourceId: string;
    resource: string;
    revision: string | null;
    bytes: number;
    contentType: string;
    metadata: ResourceMetadata;
    annotationReady: boolean;
    wholeAnnotationReady: boolean;
    currentness: "accepted" | "observed";
    download: boolean;
    collection?: string;
}
export interface HostedMediaCatalog {
    resources: (HostedMediaDescriptor | HostedMediaUnavailable)[];
    collections: HostedResourceCollection[];
    coverage: "complete" | "partial";
    scope: "configuredAuthorizedResources";
}
export interface HostedMediaResolved extends HostedMediaDescriptor {
    revision: string;
    currentness: "accepted";
    viewerTarget: ResourceOpenTarget & {
        fragment: string | null;
        resourceFragment: { status: "none" | "missing" | "unavailable" } | { status: "found"; pageIndex: number };
    };
}
export interface HostedMediaTicket extends HostedMediaDescriptor {
    revision: string;
    currentness: "accepted";
    ticket: string;
    purpose: MediaPurpose;
    sha256: string;
    expiresInMs: number;
}
export interface HostedMediaChunk {
    ticket: string;
    purpose: MediaPurpose;
    bytesBase64: string;
    offset: number;
    length: number;
    total: number;
    sha256: string;
    done: boolean;
}
export interface HostedMediaSelection {
    descriptor: HostedMediaDescriptor;
    target: MediaRegionInput | null;
}
function descriptor(raw: unknown, extensions: readonly string[] = []): HostedMediaDescriptor | HostedMediaUnavailable {
    const record = mediaRecord(raw);
    const common = ["asset", "owner", "title", "name", "lifecycle", "status"];
    const ready = ["resourceId", "resource", "revision", "bytes", "contentType", "metadata", "annotationReady", "wholeAnnotationReady", "currentness", "download"];
    if (record.status === "unavailable") {
        if (!exactKeys(record, ["asset", "status"], ["owner", "title", "name", "lifecycle", "collection"]) || typeof record.asset !== "string" || !record.asset || ["owner", "title", "name", "collection"].some(key => Object.hasOwn(record, key) && typeof record[key] !== "string") || Object.hasOwn(record, "lifecycle") && !["live", "trash"].includes(String(record.lifecycle))) return invalidMediaResponse();
        return { ...record } as unknown as HostedMediaUnavailable;
    }
    if (!exactKeys(record, [...common, ...ready], ["collection", ...extensions])) return invalidMediaResponse();
    const value = record as unknown as HostedMediaDescriptor;
    if (!value || ![value.asset, value.owner, value.name, value.title].every(item => typeof item === "string" && item.length > 0) || !["live", "trash"].includes(value.lifecycle))
        throw new UiError(uiMessage("hostedMediaState.catalogIdentityMissing"));
    if (value.status !== "ready" || value.resourceId !== value.asset || !/^weftext-resource:\/\//.test(value.resource) || value.revision !== null && (typeof value.revision !== "string" || !value.revision) || !Number.isSafeInteger(value.bytes) || value.bytes < 0 || value.bytes > 64 * 1024 * 1024 || typeof value.contentType !== "string" || typeof value.annotationReady !== "boolean" || typeof value.wholeAnnotationReady !== "boolean" || typeof value.download !== "boolean" || !["accepted", "observed"].includes(value.currentness) || value.collection !== undefined && typeof value.collection !== "string" || (value.revision === null ? value.currentness !== "observed" || value.annotationReady || value.wholeAnnotationReady : value.currentness !== "accepted"))
        throw new UiError(uiMessage("hostedMediaState.exactResourceDeliveryMissing"));
    return { asset: value.asset, owner: value.owner, title: value.title, name: value.name, lifecycle: value.lifecycle, status: "ready", resourceId: value.resourceId, resource: value.resource, revision: value.revision, bytes: value.bytes, contentType: value.contentType, metadata: decodeResourceMetadata(value.metadata), annotationReady: value.annotationReady, wholeAnnotationReady: value.wholeAnnotationReady, currentness: value.currentness, download: value.download, ...(value.collection !== undefined ? { collection: value.collection } : {}) };
}
export function decodeMediaDescriptor(value: unknown): HostedMediaDescriptor | HostedMediaUnavailable { return descriptor(value); }
export function mediaFingerprint(value: HostedMediaDescriptor): string { return JSON.stringify([value.asset, value.owner, value.resource, value.revision, value.name, value.title, value.bytes, value.contentType, value.metadata, value.lifecycle, value.annotationReady, value.wholeAnnotationReady, value.currentness, value.download, value.collection]); }
export function mediaViewerDescriptor(value: HostedMediaDescriptor): MediaResource { return { resourceId: value.asset, resource: value.resource, name: value.name, revision: value.revision, bytes: value.bytes, contentType: value.contentType, metadata: value.metadata, annotationReady: value.annotationReady, currentness: value.currentness }; }
export function mediaTicket(expected: HostedMediaDescriptor, purpose: MediaPurpose, value: HostedMediaTicket): HostedMediaTicket {
    if (!value || typeof value.ticket !== "string" || !value.ticket || value.purpose !== purpose || !/^[a-f0-9]{64}$/.test(value.sha256) || !Number.isSafeInteger(value.expiresInMs) || value.expiresInMs <= 0 || value.expiresInMs > 900000 || value.currentness !== "accepted")
        throw new UiError(uiMessage("hostedMediaState.ticketPurposeExpiryDigestMissing"));
    const decoded = descriptor(value, ["ticket", "purpose", "sha256", "expiresInMs"]);
    if (decoded.status !== "ready" || !decoded.revision || decoded.currentness !== "accepted" || mediaFingerprint(decoded) !== mediaFingerprint(expected))
        throw new UiError(uiMessage("hostedMediaState.readResourceMetadataMismatch"));
    return { ...decoded, revision: decoded.revision, currentness: "accepted", ticket: value.ticket, purpose: value.purpose, sha256: value.sha256, expiresInMs: value.expiresInMs };
}
export function mediaChunk(ticket: HostedMediaTicket, offset: number, value: HostedMediaChunk): Uint8Array {
    if (!exactKeys(mediaRecord(value), ["ticket", "purpose", "bytesBase64", "offset", "length", "total", "sha256", "done"])) return invalidMediaResponse();
    const empty = ticket.bytes === 0 && offset === 0 && value?.length === 0 && value.done === true;
    if (!value || !Number.isSafeInteger(offset) || offset < 0 || value.ticket !== ticket.ticket || value.purpose !== ticket.purpose || value.offset !== offset || value.total !== ticket.bytes || value.sha256 !== ticket.sha256 || !Number.isSafeInteger(value.length) || (value.length <= 0 && !empty) || value.length > 65536 || offset + value.length > ticket.bytes || value.done !== (offset + value.length === ticket.bytes) || typeof value.bytesBase64 !== "string" || value.bytesBase64.length > 87384)
        throw new UiError(uiMessage("hostedMediaState.chunkTicketMismatch"));
    if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value.bytesBase64)) return invalidMediaResponse();
    const bytes = Uint8Array.from(atob(value.bytesBase64), char => char.charCodeAt(0));
    if (bytes.length !== value.length)
        throw new UiError(uiMessage("hostedMediaState.chunkLengthIncomplete"));
    return bytes;
}
export function mediaResolved(value: HostedMediaResolved, reference: string, revision?: string): HostedMediaResolved {
    if (typeof reference !== "string" || !/^weftext-resource:\/\//.test(reference))
        throw new UiError(uiMessage("hostedMediaState.stableReferenceRequired"));
    const decoded = descriptor(value, ["viewerTarget"]);
    if (decoded.status !== "ready" || !decoded.revision || decoded.currentness !== "accepted" || revision !== undefined && decoded.revision !== revision || value.viewerTarget?.resource !== decoded.resource || value.viewerTarget?.revision !== decoded.revision)
        throw new UiError(uiMessage("hostedMediaState.linkExactRevisionMismatch"));
    if (reference.split("#", 1)[0] !== decoded.resource)
        throw new UiError(uiMessage("hostedMediaState.resolvedReferenceMismatch"));
    const target = value.viewerTarget, targetRecord = mediaRecord(target), fragment = mediaRecord(target.resourceFragment);
    if (!exactKeys(targetRecord, ["resource", "revision", "fragment", "resourceFragment"]) || target.fragment !== null && typeof target.fragment !== "string" || !["none", "found", "missing", "unavailable"].includes(String(fragment.status)) || !exactKeys(fragment, fragment.status === "found" ? ["status", "pageIndex"] : ["status"]) || fragment.status === "found" && !mediaInteger(fragment.pageIndex)) return invalidMediaResponse();
    const pageIndex = target.resourceFragment?.status === "found" ? target.resourceFragment.pageIndex : undefined;
    if (["missing", "unavailable"].includes(target.resourceFragment?.status ?? "") || reference.includes("#") && (target.resourceFragment?.status !== "found" || !Number.isSafeInteger(pageIndex)))
        throw new UiError(uiMessage("hostedMediaState.exactLinkedLocationUnavailable"));
    const error = resourceOpenError({ resource: target.resource, revision: target.revision, pageIndex }, mediaViewerDescriptor(decoded));
    if (error)
        throw new UiError(error);
    return { ...decoded, revision: decoded.revision, currentness: "accepted", viewerTarget: { ...target, pageIndex } };
}
export function mediaIntentTarget(selection: HostedMediaSelection, whole = false): Record<string, unknown> {
    const { descriptor, target } = selection;
    if (descriptor.lifecycle !== "live" || !(whole ? descriptor.wholeAnnotationReady : descriptor.annotationReady))
        throw new UiError(uiMessage("hostedMediaState.annotationWriteNotPermitted"));
    if (whole)
        return { kind: "whole_resource", asset: descriptor.asset, revision: descriptor.revision };
    if (!target || validateMediaRegion(target) || target.resource !== descriptor.resource || target.revision !== descriptor.revision)
        throw new UiError(uiMessage("hostedMediaState.selectValidExactMediaTarget"));
    const metadata = descriptor.metadata;
    const facts = target.kind === "image_region" ? metadata.kind === "image" && target.width === metadata.width && target.height === metadata.height
        : target.kind === "pdf_region" ? metadata.kind === "pdf" && target.pageIndex !== undefined && metadata.pages[target.pageIndex] && target.width === metadata.pages[target.pageIndex].width && target.height === metadata.pages[target.pageIndex].height && target.rotation === metadata.pages[target.pageIndex].rotation
            : (metadata.kind === "audio" || metadata.kind === "video") && target.timebase === metadata.timebase.numerator && (target.timebaseDenominator ?? 1) === metadata.timebase.denominator && target.durationTick === metadata.durationTick && (metadata.kind !== "video" || target.width === metadata.width && target.height === metadata.height);
    if (!facts)
        throw new UiError(uiMessage("hostedMediaState.targetResourceBasisMismatch"));
    const { resource: _, ...geometry } = target;
    return { ...geometry, asset: descriptor.asset };
}
export function mediaAnnotationSelection(target: unknown): MediaRegionInput | null { return storedMediaSelection(target); }
