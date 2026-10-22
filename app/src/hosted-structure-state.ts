import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { DataSchema, NativeValue } from "./data-panel";
export type StructureKind = "create" | "rename" | "move" | "trash" | "restore" | "copy" | "template";
export interface StructureSource {
    resource: string;
    title: string;
    actions: StructureKind[];
    status: "ready" | "unavailable";
}
export type StructureNamePolicy = {
    kind: "fixed";
    name: string;
} | {
    kind: "uniqueChildren";
    prefix: string;
    maxMembers: number;
};
export interface StructureSlot {
    resource: string;
    title: string;
    namePolicy: StructureNamePolicy;
    parent: string;
    parentTitle?: string;
    create: boolean;
    receive: boolean;
    status: "ready" | "unavailable";
    types?: string[];
    capture?: DataSchema & {
        roleRef: string;
        semanticDigest: string;
    };
    captureStatus?: "available" | "unavailable";
}
export interface StructureTemplate {
    id: string;
    title: string;
    source?: string;
    builtin?: boolean;
    status: "ready" | "unavailable";
    parameters?: {
        name: string;
        kind: string;
        nullable?: boolean;
        default?: unknown;
    }[];
}
export interface StructureCatalog {
    sources: StructureSource[];
    slots: StructureSlot[];
    templates: StructureTemplate[];
    scope: "configuredAuthorizedStructure";
    coverage: "complete" | "partial";
}
export interface StructureCapture {
    title?: string;
    body?: string;
    bodyMode?: "text" | "markup";
    types?: string[];
    fields?: Record<string, NativeValue>;
    formBasis?: {
        schemaRef: string;
        semanticDigest: string;
    };
}
export interface StructureRequest {
    resource: string;
    operationId: string;
    kind: StructureKind;
    source?: string;
    destination?: string;
    trashOperationId?: string;
    template?: string;
    parameters?: Record<string, unknown>;
    capture?: StructureCapture;
    inputs?: string[];
}
export interface StructureReceipt {
    resource: string;
    operationId: string;
    status: string;
    planId?: string | null;
    preview?: {
        summary?: string | string[];
        losses?: {
            code?: string;
            message: string;
        }[];
        [key: string]: unknown;
    };
    nodeRefs?: {
        workspaceId: string;
        nodeId: string;
    }[];
    resourceRefs?: string[];
    commitAllowed: boolean;
    result?: {
        resource: string;
        nodeRef: {
            workspaceId: string;
            nodeId: string;
        };
        canOpen: boolean;
        status: string;
    };
}
export interface StructureDraft {
    kind: StructureKind;
    source: string;
    destination: string;
    template: string;
    trashOperationId: string;
    title: string;
    body: string;
    bodyMode: "text" | "markup";
    parametersText: string;
    types: string[];
    fields: Record<string, NativeValue>;
    rawInputs: Record<string, string>;
    inputs: string[];
    revision: number;
    pending?: {
        request: StructureRequest;
        input: string;
        revision: number;
        selector: string;
        receipt?: StructureReceipt;
        acceptLosses: boolean;
        commit?: {
            resource: string;
            operationId: string;
            planId: string;
            acceptLosses: boolean;
        };
    };
}
const kinds: StructureKind[] = ["create", "rename", "move", "trash", "restore", "copy", "template"];
const states = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export const structureEnded = (status?: string) => ["committed", "resolved", "cancelled"].includes(status ?? "");
export function initialStructureDraft(): StructureDraft { return { kind: "create", source: "", destination: "", template: "native-note", trashOperationId: "", title: "", body: "", bodyMode: "text", parametersText: "{}", types: [], fields: {}, rawInputs: {}, inputs: [], revision: 0 }; }
export const structureInput = (draft: StructureDraft) => JSON.stringify([draft.kind, draft.source, draft.destination, draft.template, draft.trashOperationId, draft.title, draft.body, draft.bodyMode, draft.parametersText, draft.types, draft.fields, draft.rawInputs, draft.inputs]);
export function clearStructureCapture(draft: StructureDraft): StructureDraft { return { ...draft, types: [], fields: {}, rawInputs: {}, revision: draft.revision + 1 }; }
export function decodeStructureCatalog(value: StructureCatalog): StructureCatalog {
    if (!value || value.scope !== "configuredAuthorizedStructure" || !["complete", "partial"].includes(value.coverage) || !Array.isArray(value.sources) || !Array.isArray(value.slots) || !Array.isArray(value.templates))
        throw new UiError(uiMessage("hostedStructureState.catalogScopeMissing"));
    for (const item of value.sources)
        if (!item.resource || typeof item.title !== "string" || !["ready", "unavailable"].includes(item.status) || !Array.isArray(item.actions) || item.actions.some(kind => !kinds.includes(kind)))
            throw new UiError(uiMessage("hostedStructureState.sourceActionEligibilityMissing"));
    for (const item of value.slots) {
        if ("name" in item || ![item.resource, item.title, item.parent].every(part => typeof part === "string" && !!part) || !["ready", "unavailable"].includes(item.status) || typeof item.create !== "boolean" || typeof item.receive !== "boolean")
            throw new Error("结构目标没有批准的命名策略或位置。");
        const policy = item.namePolicy;
        if (!policy || typeof policy !== "object" || Array.isArray(policy))
            throw new Error("结构目标没有封闭且有界的命名授权。");
        const valid = policy.kind === "fixed"
            ? Object.keys(policy).sort().join() === "kind,name" && typeof policy.name === "string" && !!policy.name
            : policy.kind === "uniqueChildren" && Object.keys(policy).sort().join() === "kind,maxMembers,prefix" && typeof policy.prefix === "string" && !!policy.prefix && Number.isSafeInteger(policy.maxMembers) && policy.maxMembers >= 1 && policy.maxMembers <= 5000 && !item.receive;
        if (!valid)
            throw new Error("结构目标没有封闭且有界的命名授权。");
    }
    for (const item of value.templates)
        if (!item.id || typeof item.title !== "string" || !["ready", "unavailable"].includes(item.status))
            throw new UiError(uiMessage("hostedStructureState.templateDescriptionUnavailable"));
    return structuredClone(value);
}
export function structureSelectors(draft: StructureDraft, catalog: StructureCatalog): string {
    const source = draft.kind === "create" || draft.kind === "template" && draft.template === "native-note" ? undefined : catalog.sources.find(item => item.resource === draft.source), slot = ["create", "rename", "move", "restore", "copy", "template"].includes(draft.kind) ? catalog.slots.find(item => item.resource === draft.destination) : undefined, template = draft.kind === "template" ? catalog.templates.find(item => item.id === draft.template) : undefined;
    return JSON.stringify([source ?? null, slot ?? null, template ?? null]);
}
export function freezeStructureRequest(draft: StructureDraft, catalog: StructureCatalog, operationId: string): StructureRequest {
    if (draft.pending)
        throw new UiError(uiMessage("hostedStructureState.originalOperationMustBeResolved"));
    const source = catalog.sources.find(item => item.resource === draft.source), slot = catalog.slots.find(item => item.resource === draft.destination), template = catalog.templates.find(item => item.id === draft.template);
    const builtin = draft.kind === "template" && draft.template === "native-note";
    if (draft.kind !== "create" && !builtin && (!source || draft.kind !== "restore" && source.status !== "ready" || !source.actions.includes(draft.kind)))
        throw new UiError(uiMessage("hostedStructureState.sourceActionNotPermitted"));
    const unique = slot?.namePolicy.kind === "uniqueChildren";
    if (unique && !["create", "template"].includes(draft.kind))
        throw new Error("唯一子项授权仅允许新建或获权模板创建。");
    if (draft.kind !== "trash" && (!slot || slot.status !== "ready" || !(draft.kind === "create" || builtin || unique && draft.kind === "template" ? slot.create : slot.receive)))
        throw new UiError(uiMessage("hostedStructureState.selectAuthorizedFixedSlot"));
    const request: StructureRequest = { resource: ["create", "copy", "template"].includes(draft.kind) ? draft.destination : draft.source, operationId, kind: draft.kind };
    if (draft.inputs.length)
        request.inputs = [...draft.inputs];
    if (draft.kind !== "create" && !builtin)
        request.source = draft.source;
    if (draft.kind !== "trash")
        request.destination = draft.destination;
    if (draft.kind === "restore") {
        if (!draft.trashOperationId.trim())
            throw new UiError(uiMessage("hostedStructureState.restoreOriginalRecycleIdRequired"));
        request.trashOperationId = draft.trashOperationId;
    }
    if (Object.keys(draft.rawInputs).length)
        throw new UiError(uiMessage("hostedStructureState.propertyOriginalIncomplete"));
    if (draft.kind === "create" && (draft.types.length || Object.keys(draft.fields).length)) {
        if (!slot?.capture || slot.captureStatus === "unavailable" || draft.types.some(type => !(slot.types ?? []).includes(type)) || Object.keys(draft.fields).some(id => !slot.capture!.fields.some(field => field.id === id)))
            throw new UiError(uiMessage("hostedStructureState.typeOrPropertyFormNotPermitted"));
    }
    if (draft.kind === "create" && (draft.title || draft.body || draft.bodyMode !== "text" || draft.types.length || Object.keys(draft.fields).length))
        request.capture = { ...(draft.title ? { title: draft.title } : {}), body: draft.body, bodyMode: draft.bodyMode, ...(draft.types.length ? { types: [...draft.types] } : {}), ...(Object.keys(draft.fields).length ? { fields: structuredClone(draft.fields) } : {}) };
    if (request.capture && (draft.types.length || Object.keys(draft.fields).length)) {
        if (!slot?.capture?.roleRef || !/^[a-f0-9]{64}$/.test(slot.capture.semanticDigest))
            throw new Error("获权表单没有准确 Schema 依据，原字段输入保留。");
        request.capture.formBasis = { schemaRef: slot.capture.roleRef, semanticDigest: slot.capture.semanticDigest };
    }
    if (draft.kind === "template") {
        if (!template || template.status !== "ready")
            throw new UiError(uiMessage("hostedStructureState.selectAvailableTemplate"));
        let parameters: unknown;
        try {
            parameters = JSON.parse(draft.parametersText);
        }
        catch {
            throw new UiError(uiMessage("hostedStructureState.templateParametersIncompleteJson"));
        }
        if (!parameters || typeof parameters !== "object" || Array.isArray(parameters))
            throw new UiError(uiMessage("hostedStructureState.templateParametersObjectRequired"));
        request.template = draft.template;
        request.parameters = parameters as Record<string, unknown>;
    }
    return structuredClone(request);
}
export function structureReceipt(request: StructureRequest, incoming: StructureReceipt, previous?: StructureReceipt): StructureReceipt {
    if (!incoming || incoming.resource !== request.resource || incoming.operationId !== request.operationId || !states.has(incoming.status) || typeof incoming.commitAllowed !== "boolean")
        throw new UiError(uiMessage("hostedStructureState.receiptObjectOrIdMismatch"));
    if (previous?.planId && incoming.planId && incoming.planId !== previous.planId || previous?.status === "committed" && incoming.status !== "committed")
        throw new UiError(uiMessage("hostedStructureState.originalFactsChanged"));
    if (incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId || !incoming.preview))
        throw new UiError(uiMessage("hostedStructureState.exactPlanPreviewMissing"));
    // Current authorization filtering replaces every old preview and locator.
    return structuredClone(incoming);
}
export function canCommitStructure(draft: StructureDraft, catalog: StructureCatalog): boolean {
    const pending = draft.pending;
    return !!pending && pending.receipt?.status === "prepared" && pending.receipt.commitAllowed && !!pending.receipt.planId && pending.revision === draft.revision && pending.input === structureInput(draft) && pending.selector === structureSelectors(draft, catalog);
}
export function decodeStructureDraft(raw: string): StructureDraft {
    const draft = JSON.parse(raw) as StructureDraft;
    if (!draft || !kinds.includes(draft.kind) || ![draft.source, draft.destination, draft.template, draft.trashOperationId, draft.title, draft.body, draft.parametersText].every(part => typeof part === "string") || !["text", "markup"].includes(draft.bodyMode) || !Array.isArray(draft.types) || draft.types.some(type => typeof type !== "string") || !Array.isArray(draft.inputs) || draft.inputs.some(id => typeof id !== "string") || !draft.fields || typeof draft.fields !== "object" || Array.isArray(draft.fields) || !draft.rawInputs || typeof draft.rawInputs !== "object" || Object.values(draft.rawInputs).some(text => typeof text !== "string") || !Number.isSafeInteger(draft.revision) || draft.revision < 0)
        throw new UiError(uiMessage("hostedStructureState.recoveryUnparseable"));
    if (draft.pending) {
        const pending = draft.pending;
        if (!pending.request?.resource || !pending.request.operationId || !kinds.includes(pending.request.kind) || typeof pending.input !== "string" || typeof pending.selector !== "string" || !Number.isSafeInteger(pending.revision) || pending.revision < 0 || typeof pending.acceptLosses !== "boolean")
            throw new UiError(uiMessage("hostedStructureState.originalRequestUnrecoverable"));
        if (pending.receipt)
            structureReceipt(pending.request, pending.receipt);
        if (pending.commit && (pending.commit.resource !== pending.request.resource || pending.commit.operationId !== pending.request.operationId || !pending.commit.planId || pending.receipt?.planId && pending.commit.planId !== pending.receipt.planId || typeof pending.commit.acceptLosses !== "boolean"))
            throw new UiError(uiMessage("hostedStructureState.commitRequestUnrecoverable"));
    }
    return draft;
}
