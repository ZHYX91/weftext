import { api, apiBlob } from "./api";
import { decodeInventoryResource } from "./inventory-resource-wire";
import type { MediaResource } from "./media-viewer";

export async function inventoryResource(root: string, directory: string, path: string): Promise<{
    descriptor: MediaResource; read(): Promise<Blob>;
}> {
    const coordinates = Object.freeze({ root, directory, path });
    const descriptor = decodeInventoryResource(await api("/api/navigation/resource", coordinates));
    return { descriptor, read: () => apiBlob("/api/navigation/resource/read", { ...coordinates, revision: descriptor.revision }) };
}
