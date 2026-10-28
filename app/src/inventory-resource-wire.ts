import { decodeResourceMetadata, invalidMediaResponse, mediaInteger, mediaRecord } from "./media-wire";
import { exactKeys } from "./ui-diagnostics";
import type { MediaResource } from "./media-viewer";

/** A transient directory observation never becomes an authored resource ref. */
export function decodeInventoryResource(value: unknown): MediaResource {
    const item = mediaRecord(value);
    const fields = ["resourceId", "resource", "name", "revision", "bytes", "contentType", "metadata", "annotationReady", "currentness"];
    if (!exactKeys(item, fields) || typeof item.resourceId !== "string" || !/^[a-f0-9]{64}$/.test(item.resourceId)
        || item.resource !== `weftext-inventory-resource://${item.resourceId}`
        || item.revision !== `observed-inventory-resource:v1:${item.resourceId}`
        || typeof item.name !== "string" || !item.name || typeof item.contentType !== "string"
        || !mediaInteger(item.bytes) || item.bytes > 64 * 1024 * 1024
        || item.annotationReady !== false || item.currentness !== "observed") return invalidMediaResponse();
    return { resourceId: item.resourceId, resource: item.resource as string, revision: item.revision as string,
        name: item.name, bytes: item.bytes, contentType: item.contentType,
        metadata: decodeResourceMetadata(item.metadata), annotationReady: false, currentness: "observed" };
}
