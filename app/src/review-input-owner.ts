import type { AuthorInputOwner } from "./author-input";

export interface ReviewMediaIdentity { readonly asset: string; readonly revision: string }
export interface ReviewThreadIdentity { readonly id: string; readonly resourceLocation?: { readonly asset?: string; readonly revision: string } }

/** The new proposal's target cannot lend its media identity to another thread. */
export function reviewInputOwner(resource: string, target: string, media: ReviewMediaIdentity | undefined, thread?: ReviewThreadIdentity): AuthorInputOwner {
    const actualMedia = thread ? thread.resourceLocation : ["media", "mediaWhole"].includes(target) ? media : undefined;
    return { kind: "review", resource, ...(thread ? { annotationId: thread.id } : {}), ...(actualMedia?.asset && actualMedia.revision ? { asset: actualMedia.asset, revision: actualMedia.revision } : {}) };
}
