import type { AuthorInputFactory, AuthorInputSession } from "./author-input";
import type { DocumentBasis } from "./data-panel";

export interface PanelFieldInputRequest {
    readonly basis: DocumentBasis;
    readonly fieldId: string;
    /** The exact proposal still owns this form. A newer proposal never adopts its events. */
    readonly current: () => boolean;
    readonly revision?: () => unknown;
}
export type PanelFieldInputFactory = (request: PanelFieldInputRequest) => AuthorInputSession | undefined;
export interface CurrentFieldInputOwner {
    /** The host's actual resource identity, independent of visible paths and labels. */
    readonly resource: string;
    /** Includes the actual document session, field grant and captured source basis. */
    readonly current: () => boolean;
}

/** The host resolves field authority; the proposal contributes only its own lifetime. */
export function createPanelFieldInputs(
    factory: AuthorInputFactory,
    resolve: (basis: Readonly<DocumentBasis>, fieldId: string) => CurrentFieldInputOwner | undefined,
): PanelFieldInputFactory {
    return request => {
        const basis = Object.freeze({ ...request.basis }), fieldId = request.fieldId;
        if (!fieldId || !request.current()) return;
        const owner = resolve(basis, fieldId);
        if (!owner?.resource || !owner.current()) return;
        const current = request.current, revision = request.revision;
        return factory({ kind: "field", resource: owner.resource, fieldId }, () => owner.current() && current(), undefined, revision);
    };
}
