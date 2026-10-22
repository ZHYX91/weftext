import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { TagTree, type TreeTag } from "./tag-tree";
export type RegistryKind = "admitDirectory" | "createTag" | "renameTag" | "moveTag";
export type RegistryIntent = {
    kind: "admitDirectory";
    slot: string;
} | {
    kind: "createTag";
    basis: string;
    displayName: string;
    parentRef: string | null;
    assignTo?: {
        resource: string;
        fieldId: string;
        basis: string;
    };
} | {
    kind: "renameTag";
    basis: string;
    reference: string;
    displayName: string;
} | {
    kind: "moveTag";
    basis: string;
    reference: string;
    parentRef: string | null;
};
export type RegistryRequest = {
    operationId: string;
    intent: RegistryIntent;
};
export type RegistryResult = {
    kind: "directory" | "tag";
    reference: string;
    status: string;
    canResolve?: boolean;
    title?: string;
    readOnly?: boolean;
};
export type RegistryReceipt = {
    operationId: string;
    status: string;
    planId?: string | null;
    preview: {
        kind?: string;
        title?: string;
        displayName?: string;
        beforeName?: string;
        parentRef?: string | null;
        beforeParentRef?: string | null;
    };
    commitAllowed: boolean;
    navigationRef?: string | null;
    result?: RegistryResult | null;
};
export type RegistryDraft = {
    kind: RegistryKind;
    slot: string;
    reference: string;
    displayName: string;
    parentRef: string | null;
};
export type RegistryCatalog = {
    directoryScope: "configuredAuthorizedDirectories";
    directoryCoverage: "complete" | "partial";
    directories: {
        directory: string;
        title: string;
        reference: string;
        status: "available" | "unavailable";
        canResolve: boolean;
    }[];
    admissionSlots: {
        slot: string;
        title: string;
    }[];
    tags: {
        read: boolean;
        create: boolean;
        rename: boolean;
        move: boolean;
        coverage: "complete" | "partial" | "unavailable";
        items: TreeTag[];
        basis: string | null;
    };
};
export function registryIntent(draft: RegistryDraft, catalog: RegistryCatalog): RegistryIntent {
    if (draft.kind === "admitDirectory") {
        if (!catalog.admissionSlots.some(slot => slot.slot === draft.slot))
            throw new UiError(uiMessage("hostedRegistryState.selectAuthorizedDirectorySlot"));
        return { kind: draft.kind, slot: draft.slot };
    }
    if (!catalog.tags.basis || !catalog.tags.read || catalog.tags.coverage !== "complete")
        throw new UiError(uiMessage("hostedRegistryState.completeTagRegistryUnavailable"));
    const tree = new TagTree(catalog.tags.items, true), tag = draft.kind === "createTag" ? undefined : tree.byRef.get(draft.reference);
    if (draft.kind !== "createTag" && !tag)
        throw new UiError(uiMessage("hostedRegistryState.originalTagUnavailable"));
    if (draft.kind === "renameTag") {
        if (!catalog.tags.rename || !draft.displayName.trim())
            throw new UiError(uiMessage("hostedRegistryState.tagRenameUnavailable"));
        return { kind: draft.kind, basis: catalog.tags.basis, reference: draft.reference, displayName: draft.displayName };
    }
    if (draft.parentRef !== null) {
        const parent = tree.byRef.get(draft.parentRef);
        if (!parent || tag && tree.contains(tag.tagId, parent.tagId))
            throw new UiError(uiMessage("hostedRegistryState.parentUnavailableOrDescendant"));
    }
    if (draft.kind === "createTag") {
        if (!catalog.tags.create || !draft.displayName.trim())
            throw new UiError(uiMessage("hostedRegistryState.tagCreationUnavailable"));
        return { kind: draft.kind, basis: catalog.tags.basis, displayName: draft.displayName, parentRef: draft.parentRef };
    }
    if (!catalog.tags.move)
        throw new UiError(uiMessage("hostedRegistryState.tagMovePermissionMissing"));
    return { kind: draft.kind, basis: catalog.tags.basis, reference: draft.reference, parentRef: draft.parentRef };
}
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export function registryReceipt(request: RegistryRequest, incoming: RegistryReceipt, previous?: RegistryReceipt): RegistryReceipt {
    if (!incoming || incoming.operationId !== request.operationId || !statuses.has(incoming.status) || typeof incoming.commitAllowed !== "boolean" || !incoming.preview || typeof incoming.preview !== "object" || incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId))
        throw new UiError(uiMessage("hostedRegistryState.receiptOperationMismatch"));
    if (previous?.planId && incoming.planId && incoming.planId !== previous.planId)
        throw new UiError(uiMessage("hostedRegistryState.originalPlanIdChanged"));
    if (previous?.status === "committed" && incoming.status !== "committed")
        throw new UiError(uiMessage("hostedRegistryState.committedCannotReplay"));
    for (const key of ["kind", "title", "displayName", "beforeName"] as const)
        if (incoming.preview[key] !== undefined && typeof incoming.preview[key] !== "string")
            throw new UiError(uiMessage("hostedRegistryState.previewInvalid"));
    for (const key of ["parentRef", "beforeParentRef"] as const)
        if (incoming.preview[key] !== undefined && incoming.preview[key] !== null && typeof incoming.preview[key] !== "string")
            throw new UiError(uiMessage("hostedRegistryState.parentPreviewInvalid"));
    if (incoming.result && (incoming.status !== "committed" || !["tag", "directory"].includes(incoming.result.kind) || typeof incoming.result.reference !== "string"))
        throw new UiError(uiMessage("hostedRegistryState.targetCommittedEligibilityMissing"));
    // A filtered receipt never inherits an old private target/name from cache.
    return incoming;
}
