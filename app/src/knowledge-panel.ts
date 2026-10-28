import { ProposalCloseGuard } from "./proposal-close-guard";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, parseUiMessage, UiError, type UiText } from "./ui-locale";
import { disposeUiTree, joinUiText } from "./ui-locale";
import { apiErrorUiText, type ApiErrorBody } from "./api-error";
import { coreDescriptionUiText, descriptionUiText, engineDiagnosticUiText, type CoreDescription, type ErrorDescription } from "./ui-diagnostics";
import "./knowledge-view.css";
import { committedDeliveryComplete, originalCommitted } from "./original-operation";
import { ProofreadingPanel, type ProofreadOptions, type ProofreadResult, type ProcessingDefaultsSuggestion } from "./proofreading-panel";
import { canApplySuggestion, decodeKnowledgeDelivery, decodeKnowledgeDiagnosticFields, decodeKnowledgeLoss, decodeKnowledgeTemplatePreview, isGraphemeRange, KnowledgeReadGate, knowledgeDeliveryErrors, knowledgeLossMessage, layoutMentions, nodeIdentity, retainPendingKnowledgePlan, sameKnowledgeBasis, uncertainKnowledgeOperation, validateMediaRegion } from "./knowledge-state";
import type { AnnotationResolution, KnowledgeBasis, KnowledgeLoss, KnowledgeMention, KnowledgeNodeRef, MediaRegionInput, KnowledgeReadError, KnowledgeTemplatePreview } from "./knowledge-state";
import type { Origin, RenderNode } from "./types";
import { storedMediaSelection } from "./media-selection";
import { trashRestoreDraft, type LibraryTrashEntry } from "./library-trash";
import { citationInsertionByte, decodeNativeCitationSnapshot, newNativeCitationInputs, renderNativeCitations, type NativeCitationSnapshot } from "./native-citations";
import { PersonalRelationRules, RelationshipDiagnosticsView, visibleCandidate, type RelationDiagnostics, type RelationNode, type RelationIssue } from "./relationship-diagnostics";
import { nodeSourceHash } from "./hosted-navigation-state";
import { RelationshipGraphView } from "./relationship-graph-view";
import type { GraphSnapshot, GraphEdge, GraphNode } from "./relationship-graph-state";
import type { HeadingSnapshot } from "./heading-structure-state";
export interface KnowledgeContext extends KnowledgeBasis {
    inputId: string;
    inputRevision?: number;
    root: string;
    nodeId?: string;
    workspaceId?: string;
    dirty: boolean;
    selection?: {
        from: number;
        to: number;
        visibleExcerpt?: string;
        elementRole?: string;
        nodeKey?: string;
    };
}
export interface KnowledgeLink {
    sourceHash?: string;
    source: KnowledgeNodeRef;
    occurrence: {
        from: number;
        to: number;
        utf16From: number;
        utf16To: number;
        target: string;
        kind: string;
        raw?: string;
        destinationSlot?: unknown;
    };
    resolution: {
        status: string;
        target?: KnowledgeNodeRef;
        fragment?: string;
        form?: string;
    };
}
export interface KnowledgeAnnotation {
    id: string;
    annotationId?: string;
    body: string;
    purpose?: string;
    reviewState?: "open" | "resolved";
    suggestionState?: "pending" | "accepted" | "rejected";
    replacement?: string;
    visibleExcerpt?: string | {
        text: string;
    };
    targetResolution?: AnnotationResolution;
    targetSummary?: string;
    sourceLocation?: {
        from: number;
        to: number;
    };
    navigateTarget?: string;
    resourceLocation?: {
        asset?: string;
        resource: string;
        revision: string;
        target: unknown;
    };
    author?: string;
    createdAt?: string;
    verified?: boolean;
    replies?: {
        id?: string;
        body: string;
        author?: string;
        createdAt?: string;
    }[];
    target?: unknown;
}
export interface KnowledgeTemplate {
    id: string;
    title: string;
    description?: string;
    source?: string;
    sourceNode?: KnowledgeNodeRef;
    sourceRevision?: string;
    available?: boolean;
    parameters?: {
        name: string;
        title?: string;
        type?: string;
        kind?: "markup" | "string" | "bool" | "int";
        required?: boolean;
        nullable?: boolean;
        default?: string | boolean | null;
    }[];
}
export type KnowledgeTemplateValue = string | boolean | null;
export interface KnowledgeSnapshot {
    graph?: GraphSnapshot;
    headingStructure?: HeadingSnapshot;
    diagnostics?: {
        coverage: "complete" | "partial";
        scannedNodes: number;
        issues: KnowledgeLink[];
        propertyIssues?: {
            source: KnowledgeNodeRef;
            fieldId: string;
            itemPath: string[];
            address: string;
            resolution: KnowledgeLink["resolution"];
        }[];
        nodes: {
            node: KnowledgeNodeRef;
            incoming: number;
            outgoing: number;
            invalidOutgoing: number;
            state: string;
        }[];
    };
    links?: {
        generation: string | number;
        coverage: "complete" | "partial";
        outgoing: KnowledgeLink[];
        incomingId: KnowledgeLink[];
        incomingPath: KnowledgeLink[];
    };
    renderedLinks?: {
        compileIdentity: string;
        target: string;
        projectionGeneration: string;
        occurrenceOrdinal: number;
        fragments: string[];
        destination: string;
        origin: string;
    }[];
    propertyReferences?: {
        coverage: "complete" | "partial";
        items: {
            source: KnowledgeNodeRef;
            fieldId: string;
            fieldTitle?: string;
            itemPath: string[];
            resolution: KnowledgeLink["resolution"];
        }[];
        diagnostics: readonly ErrorDescription[];
    };
    citations?: NativeCitationSnapshot;
    mentions?: {
        generation: string | number;
        coverage: "complete" | "partial";
        fragments: {
            id: string;
            text?: string;
            nodeKey?: string;
            coverage?: string;
            hits: KnowledgeMention[];
        }[];
    };
    annotations?: KnowledgeAnnotation[];
    annotationStatus?: string;
    annotationDiagnostics?: readonly ApiErrorBody[];
    templates?: KnowledgeTemplate[];
    trash?: LibraryTrashEntry[];
    warnings?: readonly CoreDescription[];
    errors?: readonly KnowledgeReadError[];
}
export function decodeKnowledgeSnapshot(value: unknown): KnowledgeSnapshot {
    const record = decodeKnowledgeDiagnosticFields(value);
    return { ...record, ...(record.citations !== null ? { citations: decodeNativeCitationSnapshot(record.citations) } : { citations: undefined }) } as unknown as KnowledgeSnapshot;
}
export interface KnowledgePlan {
    operationId: string;
    planId: string;
    kind?: string;
    status?: string;
    changes?: {
        path?: string;
        action?: string;
        bytes?: number;
        before?: string;
        after?: string;
    }[];
    preview?: unknown;
    losses?: readonly KnowledgeLoss[];
    [key: string]: unknown;
}
export interface KnowledgeReceipt {
    status: string;
    operationId?: string;
    path?: string;
    deliveryError?: ApiErrorBody;
    documentError?: ApiErrorBody;
    affectedDocuments?: readonly { documentError?: ApiErrorBody }[];
    [key: string]: unknown;
}
export interface KnowledgeIntent {
    kind: string;
    [key: string]: unknown;
}
export interface KnowledgeTextMapping {
    nodeKey: string;
    node: Text | HTMLElement;
    text: string;
    origin?: Origin | null;
}
export interface KnowledgeHost {
    processingDefaults?(): Promise<ProcessingDefaultsSuggestion>;
    proofread?(context: KnowledgeContext, options: ProofreadOptions): Promise<ProofreadResult>;
    context(): KnowledgeContext | null;
    load(context: KnowledgeContext): Promise<KnowledgeSnapshot>;
    /** A fresh preparation must qualify the current Draft and freeze every effect. */
    prepare(intent: KnowledgeIntent, context: KnowledgeContext, operationId: string): Promise<KnowledgePlan>;
    commit(plan: KnowledgePlan, context: KnowledgeContext): Promise<KnowledgeReceipt>;
    navigate(target: string | KnowledgeNodeRef): void | Promise<void>;
    notify(message: UiText, error?: boolean): void;
    inspectOperation?(operationId: string, context: KnowledgeContext): Promise<KnowledgeReceipt>;
    previewTemplate?(id: string, parameters: Record<string, KnowledgeTemplateValue>, context: KnowledgeContext): Promise<KnowledgeTemplatePreview>;
    linkCopy?(link: KnowledgeLink, form: string, context: KnowledgeContext): Promise<{
        address: string;
        fullLink: string;
    }>;
    mountMedia?(container: HTMLElement, selected: (target: MediaRegionInput | null) => void, initial?: MediaRegionInput): {
        dispose(): void;
    };
    selectSource?(from: number, to: number): void;
    copy?(text: string): Promise<void>;
    beforeAction?(): Promise<void>;
    canEditSource?(): boolean;
}
export type KnowledgePanelTab = "links" | "graph" | "citations" | "mentions" | "annotations" | "proofreading" | "files" | "templates";
export interface KnowledgePanel {
    assertCloseReady(): void;
    freezeForClose(): boolean;
    thawAfterClose(): void;
    backgroundEnabled(): boolean;
    environmentChanged(root: string): void;
    refresh(): Promise<void>;
    selectTab(tab: KnowledgePanelTab): void;
    attachSurface(surface: HTMLElement | null, mappings?: KnowledgeTextMapping[]): void;
    dispose(): void;
}
/** Read sharing keeps the exact logical owner; a Draft ACK does not change a Knowledge read. */
function sameKnowledgeReadContext(a: KnowledgeContext, b: KnowledgeContext | null): boolean {
    return sameKnowledgeBasis(a, b) && b !== null
        && a.root === b.root
        && a.workspaceId === b.workspaceId && a.nodeId === b.nodeId && a.dirty === b.dirty
        && a.selection?.from === b.selection?.from && a.selection?.to === b.selection?.to
        && a.selection?.visibleExcerpt === b.selection?.visibleExcerpt
        && a.selection?.elementRole === b.selection?.elementRole && a.selection?.nodeKey === b.selection?.nodeKey;
}
const resolutionLabels: Record<string, UiText> = { exact: uiMessage("hostedReview.exactTarget"), mapped: uiMessage("knowledge.locationMapped"), stale: uiMessage("knowledge.locationNeedsRelocation"), ambiguous: uiMessage("knowledge.locationAmbiguous"), orphaned: uiMessage("knowledge.targetExpired"), unavailable: uiMessage("knowledge.targetUnavailable"), resolved: uiMessage("knowledge.targetValid"), missing: uiMessage("knowledge.targetMissing"), fragmentMissing: uiMessage("knowledge.paragraphLabelMissing"), invalid: uiMessage("knowledge.addressInvalid"), external: uiMessage("knowledge.externalLink") };
const formLabels: Record<string, UiText> = { id: uiMessage("knowledge.formStableId"), relative: uiMessage("knowledge.formSourceRelative"), workspace: uiMessage("knowledge.formLibraryPath"), file: uiMessage("knowledge.formAbsolutePath") };
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: UiText): HTMLElementTagNameMap[K] {
    const value = document.createElement(tag);
    value.className = className;
    if (text !== undefined)
        setUiText(value, text);
    return value;
}
function createAction(label: UiText, run: () => unknown | Promise<unknown>, className = "knowledge-button"): HTMLButtonElement {
    const value = element("button", className, label);
    value.type = "button";
    value.onclick = () => { void Promise.resolve(run()).catch(() => { }); };
    return value;
}
function field(label: UiText, control: HTMLElement): HTMLLabelElement { const value = element("label", "knowledge-field"); value.append(element("span", "", label), control); return value; }
function input(value = "", placeholder: UiText = ""): HTMLInputElement { const control = element("input", "knowledge-input"); control.value = value; setUiAttr(control, "placeholder", placeholder); return control; }
function textarea(value = "", placeholder: UiText = ""): HTMLTextAreaElement { const control = element("textarea", "knowledge-input"); control.value = value; setUiAttr(control, "placeholder", placeholder); control.rows = 3; return control; }
function select(options: [
    string,
    UiText
][]): HTMLSelectElement {
    const control = element("select", "knowledge-input");
    for (const [id, label] of options) {
        const option = element("option", "", label);
        option.value = id;
        control.append(option);
    }
    return control;
}
function nodeLabel(ref: KnowledgeNodeRef): UiText { return ref.title || ref.path?.replaceAll("\\", "/").split("/").at(-1) || uiMessage("knowledge.untitledNoteFallback"); }
function excerpt(annotation: KnowledgeAnnotation): string { return typeof annotation.visibleExcerpt === "string" ? annotation.visibleExcerpt : annotation.visibleExcerpt?.text ?? ""; }
function nativeTree(node: RenderNode): Node {
    if (node.kind === "text")
        return uiTextNode(node.text ?? "");
    if (node.kind === "tag")
        return document.createDocumentFragment();
    const allowed = new Set(["p", "div", "span", "strong", "em", "s", "blockquote", "ul", "ol", "li", "table", "thead", "tbody", "tr", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "br"]);
    const value = element(allowed.has(node.tag ?? "") ? node.tag as "div" : "div");
    if (node.kind === "frame")
        setUiText(value, uiMessage("knowledge.layoutNeedsPagedPreview"));
    else {
        for (const child of node.children ?? [])
            value.append(nativeTree(child));
        if (node.text)
            value.append(uiTextNode(node.text));
    }
    return value;
}
/** Mount beside the shared editor. No independent source, history or persistence store. */
export function mountKnowledgePanel(container: HTMLElement, host: KnowledgeHost): KnowledgePanel {
    const closeGuard = new ProposalCloseGuard();
    closeGuard.bind(container);
    // Input events belong to this owner; mounting or scanning controls is never proof of a draft.
    let authorProposal = false;
    const authoredInput = (): void => {
        closeGuard.assertMutable();
        if (["annotations", "files", "templates", "citations"].includes(tab) || tab === "links" && relationFilter === "citations")
            authorProposal = true;
    };
    container.addEventListener("input", authoredInput, true);
    container.addEventListener("change", authoredInput, true);
    function action(label: UiText, run: () => unknown | Promise<unknown>, className = "knowledge-button"): HTMLButtonElement {
        return createAction(label, () => closeGuard.run(async () => {
            if (disposed) return;
            return await run();
        }).catch(report), className);
    }
    container.classList.add("knowledge-panel");
    setUiAttr(container, "aria-label", uiMessage("knowledge.title"));
    const gate = new KnowledgeReadGate();
    type PendingRead = { context: KnowledgeContext; generation: number; completion: Promise<boolean> };
    let pendingRead: PendingRead | null = null;
    function invalidateRead(): void { gate.invalidate(); pendingRead = null; }
    const graph = new RelationshipGraphView();
    let graphSession = "";
    const proofreading = new ProofreadingPanel({ resultScope: "local", context: host.context, scan: (context, options) => host.proofread ? host.proofread(context, options) : Promise.reject(new UiError(uiMessage("knowledge.proofreadingApiUnavailable"))), perform, selectSource: host.selectSource, beforeAction: host.beforeAction, processingDefaults: host.processingDefaults, notify: message => host.notify(message, true) });
    let snapshot: KnowledgeSnapshot | null = null;
    let snapshotBasis: KnowledgeContext | null = null;
    let tab = "links";
    let enabled = localStorage.getItem("weftext-potential-links") === "true";
    let problemHints = localStorage.getItem("weftext-link-problem-hints") === "true", composing = false;
    const hintedLinks = new Map<HTMLAnchorElement, {
        title: string | null;
        description: string | null;
        hint: HTMLElement;
    }>();
    let ruleScope = "", rules = new PersonalRelationRules(), diagnosticView: RelationshipDiagnosticsView;
    let mentionDomain = "all", exclusionScope = "note";
    function relationNode(ref: KnowledgeNodeRef): RelationNode { return { key: nodeIdentity(ref), title: nodeLabel(ref), path: ref.path, reference: `weftext-node://${ref.workspaceId}/${ref.nodeId}` }; }
    function ensureRules(): void {
        const scope = host.context()?.workspaceId ?? "";
        if (scope === ruleScope && diagnosticView)
            return;
        rules.closePreviews();
        ruleScope = scope;
        const key = `weftext-relations-v1:${scope}`;
        let stored: string | null = null;
        try {
            stored = localStorage.getItem(key);
        }
        catch { /* Saving reports its own error. */ }
        rules = new PersonalRelationRules(next => localStorage.setItem(key, JSON.stringify(next)), stored);
        diagnosticView = new RelationshipDiagnosticsView(rules, node => closeGuard.run(() => Promise.resolve(host.navigate(node.reference))), async (issue) => closeGuard.run(async () => {
            if (!sameKnowledgeBasis(snapshotBasis, host.context()))
                throw new UiError(uiMessage("knowledge.diagnosticBasisChanged"));
            const current = host.context();
            if (!current || `${current.workspaceId}/${current.nodeId}` !== issue.source.key)
                await host.navigate(issue.source.reference);
            if (issue.field) {
                host.notify(uiMessage("knowledge.propertyOwnerOpenedNoSource"));
                return;
            }
            const target = host.context();
            if (!target || `${target.workspaceId}/${target.nodeId}` !== issue.source.key || !issue.sourceHash || await nodeSourceHash(target.source) !== issue.sourceHash) {
                host.notify(uiMessage("knowledge.sourceOpenedBodyChanged"), true);
                return;
            }
            if (issue.from !== undefined && issue.to !== undefined)
                host.selectSource?.(issue.from, issue.to);
        }), report);
    }
    let surface: HTMLElement | null = null;
    let mappings: KnowledgeTextMapping[] = [];
    let overlay: HTMLElement | null = null;
    let popover: HTMLElement | null = null;
    let popoverAnchor: HTMLElement | null = null;
    type PendingPreparation = { operationId: string; title: UiText; intent: KnowledgeIntent; context: KnowledgeContext; prepared: boolean };
    let pendingPreparation: PendingPreparation | null = null;
    let pendingPlan: {
        plan: KnowledgePlan;
        context: KnowledgeContext;
        receipt?: KnowledgeReceipt;
    } | null = null;
    let disposed = false, busy = false, loading = false;
    let annotationBody = "", suggestionReplacement = "", annotationKind = "comment", annotationTarget = "whole_document";
    let annotationSelection: KnowledgeContext["selection"];
    let annotationSelectionBasis: KnowledgeContext | null = null;
    const replies = new Map<string, string>();
    const citationInputs = new Map<string, ReturnType<typeof newNativeCitationInputs>>();
    let relationFilter = "all";
    const reconfirmations = new Map<string, {
        replacement: string;
        basis: KnowledgeContext | null;
        selection?: KnowledgeContext["selection"];
    }>();
    const fileDrafts = new Map<string, {
        name: string;
        parent: string;
        backup: string;
        allowLosses: boolean;
        restores: Map<string, {
            name: string;
            parent: string;
        }>;
    }>();
    const templateDrafts = new Map<string, {
        name: string;
        parent: string;
        parameters: Record<string, KnowledgeTemplateValue>;
        presence: Record<string, "omitted" | "value" | "none">;
    }>();
    type RegistrationSlot = {
        name: string;
        kind: "markup" | "string" | "bool" | "int";
        from: number;
        to: number;
        nullable?: boolean;
        default?: KnowledgeTemplateValue;
    };
    const registrationDrafts = new Map<string, {
        title: string;
        description: string;
        templateId?: string;
        basis: KnowledgeContext | null;
        parameters: RegistrationSlot[];
        open: boolean;
        slotsOpen: boolean;
        slot: {
            name: string;
            kind: RegistrationSlot["kind"];
            nullable: boolean;
            defaultMode: "omitted" | "value" | "none";
            defaultValue: KnowledgeTemplateValue;
        };
    }>();
    let renderedBasis: KnowledgeContext | null = null;
    let deferredRender = false;
    let mediaDraft: MediaRegionInput | null = null;
    const mediaContainer = element("div", "knowledge-media-viewer");
    let mediaController: {
        dispose(): void;
    } | null = null;
    let mediaSession: string | null = null;
    const status = element("div", "knowledge-status");
    status.setAttribute("aria-live", "polite");
    const onBlur = (): void => {
        if (deferredRender)
            queueMicrotask(() => {
                if (!disposed && !(container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))) {
                    deferredRender = false;
                    render();
                }
            });
    };
    container.addEventListener("focusout", onBlur);
    function report(error: unknown): void { const message = error instanceof Error ? errorUiText(error) : typeof error === "string" ? error : parseUiMessage(error) ?? uiMessage("error.invalidResponse"); setUiText(status, message); host.notify(message, true); }
    async function copy(text: string): Promise<void> {
        return closeGuard.run(() => copyActive(text));
    }
    async function copyActive(text: string): Promise<void> {
        try {
            if (host.copy)
                await host.copy(text);
            else
                await navigator.clipboard.writeText(text);
            host.notify(uiMessage("knowledge.copied"));
        }
        catch (error) {
            report(error);
        }
    }
    function navigate(target: string | KnowledgeNodeRef): void { void closeGuard.run(() => Promise.resolve(host.navigate(target))).catch(report); }
    function section(title: UiText, description?: UiText): HTMLElement {
        const value = element("section", "knowledge-section");
        value.append(element("h3", "", title));
        if (description)
            value.append(element("p", "knowledge-help", description));
        return value;
    }
    function closePopover(restoreFocus = false): void {
        popover?.remove();
        popover = null;
        const previous = popoverAnchor;
        popoverAnchor = null;
        if (restoreFocus && previous?.isConnected)
            previous.focus();
    }
    function clearHints(): void {
        for (const [anchor, old] of hintedLinks) {
            if (old.title === null)
                anchor.removeAttribute("title");
            else
                setUiAttr(anchor, "title", old.title);
            if (old.description === null)
                anchor.removeAttribute("aria-describedby");
            else
                anchor.setAttribute("aria-describedby", old.description);
            anchor.classList.remove("knowledge-link-problem");
            old.hint.remove();
        }
        hintedLinks.clear();
    }
    function clearOverlay(): void { closePopover(); overlay?.remove(); overlay = null; clearHints(); }
    function knownMentions(): {
        fragment: NonNullable<KnowledgeSnapshot["mentions"]>["fragments"][number];
        hit: KnowledgeMention;
    }[] {
        ensureRules();
        const source = `${host.context()?.workspaceId}/${host.context()?.nodeId}`;
        return snapshot?.mentions?.fragments.flatMap(fragment => layoutMentions(fragment.hits.map(hit => {
            const candidates = (hit.candidates ?? hit.targets.map(target => ({ target, term: hit.terms[0], provenance: "title", fragment: undefined }))).filter(c => (mentionDomain === "all" || (mentionDomain === "fragments") === !!c.fragment) && visibleCandidate(rules.rules, source, c.term, nodeIdentity(c.target)));
            return { ...hit, candidates, targets: candidates.map(c => c.target) };
        }).filter(hit => hit.candidates.length)).map(hit => ({ fragment, hit }))) ?? [];
    }
    function mentionIntent(fragmentId: string, hit: KnowledgeMention, target: KnowledgeNodeRef, fragment?: string | null): KnowledgeIntent {
        return { kind: "mentionConvert", generation: snapshot?.mentions?.generation, fragmentId, startScalar: hit.startScalar, endScalar: hit.endScalar, target, fragment };
    }
    async function confirmPlan(title: UiText, plan: KnowledgePlan): Promise<boolean> {
        const dialog = element("dialog", "knowledge-dialog");
        dialog.append(element("h2", "", title), element("p", "knowledge-help", uiMessage("knowledge.checkCompleteChanges")));
        const preview = element("div", "knowledge-plan-preview");
        const verbs: Record<string, UiText> = { create: uiMessage("knowledge.operationCreate"), write: uiMessage("knowledge.operationWrite"), update: uiMessage("knowledge.operationUpdate"), replace: uiMessage("editor.replace"), copy: uiMessage("tableGrid.copy"), move: uiMessage("hostedStructure.moveKind"), rename: uiMessage("tagNavigation.rename"), trash: uiMessage("knowledge.operationRecycle"), restore: uiMessage("knowledge.operationRestore"), delete: uiMessage("knowledge.operationRemoveOriginal"), remove: uiMessage("knowledge.operationRemoveOriginal"), relocateFrom: uiMessage("knowledge.operationMoveOut"), relocateTo: uiMessage("knowledge.operationMoveToNew"), backup: uiMessage("knowledge.operationBackupTo"), import: uiMessage("knowledge.operationWriteImport"), style: uiMessage("knowledge.operationUpdateStyle") };
        if (plan.kind === "trashNode")
            preview.append(element("p", "knowledge-warning", uiMessage("knowledge.recyclePreviewDescription")));
        if (plan.kind === "copyNode" || plan.kind === "backupLibrary")
            preview.append(element("p", "knowledge-help", uiMessage("knowledge.copyPreviewDescription")));
        if (plan.draftOnly === true)
            preview.append(element("p", "knowledge-help", uiMessage("knowledge.sharedDraftPreviewDescription")));
        const fileChanges = plan.changes?.filter(change => Boolean(change.path)) ?? [];
        if (fileChanges.length) {
            preview.append(element("h3", "", displayMessage("knowledge.fileChangeCount", { p0: fileChanges.length })));
            const paths = element("ul", "knowledge-change-list");
            for (const change of fileChanges) {
                const item = element("li");
                item.append(element("strong", "", verbs[change.action ?? ""] ?? uiMessage("knowledge.changeFallback")), element("span", "knowledge-address", change.path));
                if (change.bytes !== undefined)
                    item.append(element("small", "knowledge-help", displayMessage("knowledge.byteCount", { p0: uiDisplayParameter(change.bytes.toLocaleString()) })));
                paths.append(item);
            }
            preview.append(paths);
        }
        const currentSource = host.context()?.source;
        const edit = plan.edit as {
            from?: number;
            to?: number;
            text?: string;
        } | undefined;
        if (currentSource !== undefined && edit && typeof edit.from === "number" && typeof edit.to === "number" && typeof edit.text === "string") {
            preview.append(element("h3", "", uiMessage("knowledge.selectedRangeChanges")));
            const pair = element("div", "knowledge-before-after");
            const before = element("div");
            before.append(element("strong", "", uiMessage("knowledge.originalContent")), element("pre", "knowledge-replacement", currentSource.slice(edit.from, edit.to)));
            const after = element("div");
            after.append(element("strong", "", uiMessage("knowledge.afterApply")), element("pre", "knowledge-replacement", edit.text));
            pair.append(before, after);
            preview.append(pair);
        }
        const detail = plan.preview;
        if (typeof detail === "string")
            preview.append(element("p", "knowledge-annotation-body", detail));
        else if (detail && typeof detail === "object") {
            const previewRecord = detail as Record<string, unknown>;
            const asRecord = (value: unknown): Record<string, unknown> => value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
            const stateName = (value: unknown): UiText => typeof value === "string" ? ({ open: uiMessage("hostedReview.discussionUnresolved"), resolved: uiMessage("hostedReview.discussionResolved"), pending: uiMessage("hostedReview.suggestionPending"), accepted: uiMessage("hostedReview.suggestionAccepted"), rejected: uiMessage("hostedReview.suggestionRejected") } as Record<string, UiText>)[value] ?? value : "";
            const targetSummary = (value: unknown): UiText => {
                const target = asRecord(value), anchor = asRecord(target.anchor), range = asRecord(anchor.sourceRange);
                const kind = String(target.kind ?? "");
                if (["sourceRange", "source_range", "sourceElement", "source_element"].includes(kind))
                    return () => `${uiTextValue(kind === "sourceElement" || kind === "source_element" ? uiMessage("knowledge.targetSourceElement") : uiMessage("knowledge.targetSourceRange"))} UTF-16 [${uiTextValue(range.utf16From ?? target.from ?? "?")}, ${uiTextValue(range.utf16To ?? target.to ?? "?")})${uiTextValue(typeof anchor.visibleExcerpt === "string" ? `：${anchor.visibleExcerpt}` : "")}`;
                if (["wholeDocument", "whole_document"].includes(kind))
                    return uiMessage("hostedReview.wholeNoteTarget");
                if (kind === "pdfRegion" || kind === "pdf_region") {
                    const rect = asRecord(target.rect);
                    return displayMessage("knowledge.targetPdfRectangle", { p0: uiDisplayParameter(Number(target.pageIndex ?? 0) + 1), p1: uiDisplayParameter(rect.x ?? target.x), p2: uiDisplayParameter(rect.y ?? target.y), p3: uiDisplayParameter(rect.width ?? target.rectWidth), p4: uiDisplayParameter(rect.height ?? target.rectHeight) });
                }
                if (kind === "imageRegion" || kind === "image_region") {
                    const rect = asRecord(target.rect);
                    return displayMessage("knowledge.targetImageRectangle", { p0: uiDisplayParameter(rect.x ?? target.x), p1: uiDisplayParameter(rect.y ?? target.y), p2: uiDisplayParameter(rect.width ?? target.rectWidth), p3: uiDisplayParameter(rect.height ?? target.rectHeight) });
                }
                if (kind === "mediaInterval" || kind === "media_interval") {
                    const timebase = asRecord(target.timebase);
                    return displayMessage("knowledge.targetMediaTicks", { p0: uiDisplayParameter(target.startTick), p1: uiDisplayParameter(target.endTick), p2: uiDisplayParameter(timebase.numerator ?? target.timebase), p3: uiDisplayParameter(timebase.denominator ?? target.timebaseDenominator ?? 1) });
                }
                return uiMessage("knowledge.exactTargetFrozenByPreparation");
            };
            const pair = (title: UiText, before: UiText, after: UiText): void => {
                const group = section(title), columns = element("div", "knowledge-before-after");
                for (const [label, text] of [[uiMessage("knowledge.beforeChange"), before], [uiMessage("knowledge.afterChange"), after]] as const) {
                    const column = element("div");
                    column.append(element("strong", "", label), element("pre", "knowledge-replacement", text));
                    columns.append(column);
                }
                group.append(columns);
                preview.append(group);
            };
            const annotationText = (record: Record<string, unknown>): UiText => {
                const lines: UiText[] = [];
                if (typeof record.body === "string")
                    lines.push(displayMessage("knowledge.discussionBody", { p0: uiDisplayParameter(record.body) }));
                if (typeof record.reviewState === "string")
                    lines.push(displayMessage("knowledge.discussionState", { p0: uiDisplayParameter(stateName(record.reviewState)) }));
                if (record.target)
                    lines.push(displayMessage("hostedReview.targetSummary", { p0: uiDisplayParameter(targetSummary(record.target)) }));
                const suggestion = asRecord(record.suggestion);
                if (typeof suggestion.replacement === "string")
                    lines.push(displayMessage("knowledge.suggestedText", { p0: uiDisplayParameter(suggestion.replacement) }));
                if (typeof suggestion.state === "string")
                    lines.push(displayMessage("knowledge.suggestionState", { p0: uiDisplayParameter(stateName(suggestion.state)) }));
                return () => lines.map(uiTextValue).join("\n");
            };
            const annotationRecords = (source: unknown): Record<string, unknown>[] | null => {
                if (typeof source !== "string")
                    return null;
                try {
                    const value = JSON.parse(source);
                    return Array.isArray(value?.annotations) ? value.annotations.map(asRecord) : null;
                }
                catch {
                    return null;
                }
            };
            const templateText = (value: unknown): UiText => {
                const template = asRecord(value);
                if (!Object.keys(template).length)
                    return uiMessage("knowledge.notRegistered");
                const lines: UiText[] = [displayMessage("knowledge.templateName", { p0: uiDisplayParameter(typeof template.title === "string" ? template.title : "") })];
                if (typeof template.description === "string" && template.description)
                    lines.push(displayMessage("knowledge.templatePurpose", { p0: uiDisplayParameter(template.description) }));
                const sourceNode = asRecord(template.sourceNode);
                const context = host.context();
                if (typeof sourceNode.path === "string")
                    lines.push(displayMessage("knowledge.sourcePath", { p0: uiDisplayParameter(sourceNode.path) }));
                else if (context && sourceNode.nodeId === context.nodeId && sourceNode.workspaceId === context.workspaceId)
                    lines.push(displayMessage("knowledge.sourcePath", { p0: uiDisplayParameter(context.path) }));
                const slots = Array.isArray(template.parameters) ? template.parameters : Array.isArray(template.slots) ? template.slots : [];
                if (!slots.length)
                    lines.push(uiMessage("knowledge.wholeTemplateNoParameters"));
                for (const entry of slots) {
                    const slot = asRecord(entry), range = asRecord(slot.range);
                    const kind = String(slot.kind ?? "markup");
                    const defaults = !("default" in slot) ? uiMessage("knowledge.parameterRequired") : slot.default === null ? uiMessage("knowledge.parameterDefaultNone") : typeof slot.default === "boolean" ? displayMessage("knowledge.parameterDefaultBoolean", { p0: uiDisplayParameter(slot.default ? uiMessage("nativeValue.booleanYes") : uiMessage("nativeValue.booleanNo")) }) : typeof slot.default === "string" ? displayMessage("knowledge.parameterDefaultValue", { p0: uiDisplayParameter(slot.default === "" ? uiMessage("knowledge.emptyString") : slot.default) }) : uiMessage("knowledge.parameterDefaultInPreparedResult");
                    lines.push(displayMessage("knowledge.parameterSummary", { p0: uiDisplayParameter(slot.name), p1: uiDisplayParameter(({ markup: uiMessage("knowledge.parameterMarkup"), string: uiMessage("knowledge.parameterString"), bool: uiMessage("knowledge.parameterBoolean"), int: uiMessage("knowledge.parameterInteger") } as Record<string, UiText>)[kind] ?? kind), p2: uiDisplayParameter(defaults), p3: uiDisplayParameter(range.utf16From !== undefined ? `；UTF-16 [${range.utf16From}, ${range.utf16To})` : "") }));
                }
                return () => lines.map(uiTextValue).join("\n");
            };
            if (previewRecord.templateAfter)
                pair(uiMessage("knowledge.templateRegistrationChanges"), templateText(previewRecord.templateBefore), templateText(previewRecord.templateAfter));
            if (Array.isArray(previewRecord.files))
                for (const entry of previewRecord.files) {
                    const file = asRecord(entry);
                    if (file.action === "readDependency")
                        continue;
                    const path = typeof file.path === "string" ? file.path : uiMessage("knowledge.fileContent");
                    if (previewRecord.templateAfter && typeof path === "string" && path.replaceAll("\\", "/").endsWith("/.weftext-settings.json"))
                        continue;
                    const before = typeof file.before === "string" ? file.before : "", after = typeof file.after === "string" ? file.after : "";
                    const annotationsAfter = annotationRecords(file.after), annotationsBefore = annotationRecords(file.before) ?? [];
                    if (annotationsAfter) {
                        const previous = new Map(annotationsBefore.map(record => [record.annotationId, record]));
                        for (const record of annotationsAfter) {
                            const old = previous.get(record.annotationId);
                            if (JSON.stringify(old) !== JSON.stringify(record))
                                pair(uiMessage("knowledge.annotationChanges"), old ? annotationText(old) : uiMessage("knowledge.newAnnotation"), annotationText(record));
                        }
                    }
                    else if (before || after)
                        pair(path, before, after);
                }
            const source = asRecord(previewRecord.source);
            if (typeof source.before === "string" && typeof source.after === "string" && !Array.isArray(previewRecord.files))
                pair(uiMessage("knowledge.bodyChanges"), source.before, source.after);
            const suggestion = asRecord(previewRecord.suggestion);
            if (typeof suggestion.before === "string" && typeof suggestion.after === "string")
                pair(uiMessage("knowledge.suggestionStatus"), stateName(suggestion.before), stateName(suggestion.after));
            if (previewRecord.reconfirmation === true)
                preview.append(element("p", "knowledge-warning", uiMessage("knowledge.suggestionReconfirmDescription")));
            if (previewRecord.targetAfter)
                preview.append(element("p", "knowledge-annotation-body", displayMessage("knowledge.newTarget", { p0: uiDisplayParameter(targetSummary(previewRecord.targetAfter)) })));
            if (typeof previewRecord.replacementAfter === "string")
                preview.append(element("pre", "knowledge-replacement", previewRecord.replacementAfter));
            const labels: Record<string, UiText> = { beforeSource: uiMessage("knowledge.sourceBefore"), sourceBefore: uiMessage("knowledge.sourceBefore"), afterSource: uiMessage("knowledge.sourceAfter"), sourceAfter: uiMessage("knowledge.sourceAfter"), annotationBefore: uiMessage("knowledge.annotationBefore"), annotationAfter: uiMessage("knowledge.annotationAfter"), annotationsBefore: uiMessage("knowledge.discussionBefore"), annotationsAfter: uiMessage("knowledge.discussionAfter"), before: uiMessage("knowledge.beforeChange"), after: uiMessage("knowledge.afterChange"), summary: uiMessage("knowledge.changeDescription"), message: uiMessage("office.previewDescription"), target: uiMessage("knowledge.target"), excerpt: uiMessage("knowledge.selectedText"), replacement: uiMessage("hostedReview.replacementLabel"), address: uiMessage("shell.linkTargetAddress"), fullLink: uiMessage("knowledge.completeNativeLink"), reviewState: uiMessage("knowledge.discussionStatus"), suggestionState: uiMessage("knowledge.suggestionStatus") };
            for (const [key, value] of Object.entries(detail)) {
                if (!labels[key])
                    continue;
                const group = section(labels[key]);
                if (typeof value === "string")
                    group.append(element("pre", "knowledge-replacement", value));
                else if (value && typeof value === "object") {
                    const record = value as Record<string, unknown>;
                    for (const [fieldName, label] of [["body", uiMessage("hostedReview.bodyLabel")], ["visibleExcerpt", uiMessage("knowledge.storedExcerpt")], ["replacement", uiMessage("knowledge.suggestionText")], ["reviewState", uiMessage("knowledge.discussionStatus")], ["suggestionState", uiMessage("knowledge.suggestionStatus")]] as const) {
                        const fieldValue = record[fieldName];
                        if (typeof fieldValue === "string") {
                            const stateValue = fieldName === "reviewState" || fieldName === "suggestionState"
                                ? ({ open: uiMessage("hostedReview.discussionUnresolved"), resolved: uiMessage("hostedReview.discussionResolved"), pending: uiMessage("hostedReview.suggestionPending"), accepted: uiMessage("hostedReview.suggestionAccepted"), rejected: uiMessage("hostedReview.suggestionRejected") } as Record<string, UiText>)[fieldValue] ?? fieldValue
                                : fieldValue;
                            group.append(element("p", "knowledge-annotation-body", () => `${uiTextValue(label)}：${uiTextValue(stateValue)}`));
                        }
                    }
                }
                if (group.children.length > 1)
                    preview.append(group);
            }
        }
        if (plan.losses?.length) {
            const losses = section(uiMessage("knowledge.lossesAndUnsupportedContent"));
            for (const loss of plan.losses)
                losses.append(element("p", "knowledge-warning", knowledgeLossMessage(loss)));
            preview.append(losses);
        }
        if (!preview.children.length)
            preview.append(element("p", "knowledge-help", uiMessage("knowledge.preparedResultFixed")));
        dialog.append(preview);
        const diagnostics = element("details", "knowledge-diagnostics");
        diagnostics.append(element("summary", "", uiMessage("knowledge.viewPreparedDetails")));
        const raw = element("pre", "knowledge-plan", JSON.stringify(plan, null, 2));
        diagnostics.append(raw);
        dialog.append(diagnostics);
        const actions = element("div", "knowledge-actions");
        dialog.append(actions);
        document.body.append(dialog);
        return new Promise(resolve => {
            const finish = (value: boolean): void => { dialog.close(); disposeUiTree(dialog); dialog.remove(); resolve(value); };
            actions.append(action(uiMessage("common.cancel"), () => finish(false)), action(uiMessage("knowledge.confirmApply"), () => finish(true), "knowledge-button primary"));
            dialog.addEventListener("cancel", event => { event.preventDefault(); finish(false); });
            dialog.showModal();
        });
    }
    async function perform(title: UiText, intent: KnowledgeIntent, retry?: PendingPreparation): Promise<boolean> {
        return closeGuard.run(() => performActive(title, intent, retry));
    }
    async function performActive(title: UiText, intent: KnowledgeIntent, retry?: PendingPreparation): Promise<boolean> {
        if (busy)
            return false;
        if (pendingPlan || pendingPreparation && pendingPreparation !== retry) {
            host.notify(uiMessage("knowledge.previousOperationUnconfirmed"), true);
            return false;
        }
        busy = true;
        setUiText(status, uiMessage("knowledge.preparingExactChanges"));
        try {
            const actionBasis = host.context();
            if (!retry) await host.beforeAction?.();
            const context = retry?.context ?? host.context();
            if (!context)
                throw new UiError(uiMessage("knowledge.openCurrentNoteFirst"));
            if ((["mentionConvert", "linkConvert"].includes(intent.kind) || intent.kind.startsWith("nativeCitation") || intent.kind.startsWith("nativeReference")) && host.canEditSource?.() === false)
                throw new UiError(uiMessage("knowledge.switchToEditingForReferences"));
            if (!sameKnowledgeBasis(actionBasis, context) || !sameKnowledgeBasis(context, host.context()))
                throw new UiError(uiMessage("knowledge.inputEnteredSharedDraft"));
            const original = retry ?? { operationId: crypto.randomUUID(), title, intent: structuredClone(intent), context: structuredClone(context), prepared: false };
            pendingPreparation = original;
            const deliveredPlan = await host.prepare(structuredClone(original.intent), original.context, original.operationId);
            if (deliveredPlan.operationId !== original.operationId) throw new UiError(uiMessage("error.invalidResponse"));
            original.prepared = true;
            if (deliveredPlan.losses !== undefined && !Array.isArray(deliveredPlan.losses))
                throw new UiError(uiMessage("error.invalidResponse"));
            const plan = { ...deliveredPlan, ...(deliveredPlan.losses ? { losses: deliveredPlan.losses.map(decodeKnowledgeLoss) } : {}) };
            if (disposed)
                return false;
            if (!sameKnowledgeBasis(context, host.context()))
                throw new UiError(uiMessage("knowledge.draftOrNoteChangedDuringPreparation"));
            if (!await confirmPlan(title, plan)) {
                pendingPreparation = null;
                setUiText(status, uiMessage("knowledge.cancelledInputRetained"));
                return false;
            }
            if (disposed)
                return false;
            if (!sameKnowledgeBasis(context, host.context()))
                throw new UiError(uiMessage("knowledge.draftChangedBeforeConfirmation"));
            pendingPlan = { plan, context };
            pendingPreparation = null;
            const receipt = decodeKnowledgeDelivery(await host.commit(plan, context));
            if (receipt.operationId !== plan.operationId) throw new UiError(uiMessage("error.invalidResponse"));
            pendingPlan.receipt = receipt;
            if (uncertainKnowledgeOperation(receipt.status)) {
                setUiText(status, uiMessage("knowledge.operationUnconfirmedRetained"));
                render();
                return false;
            }
            if (!originalCommitted(receipt))
                throw new UiError(joinUiText([displayMessage("knowledge.operationIncomplete", { p0: receipt.status }), ...knowledgeDeliveryErrors(receipt)], "\n"));
            invalidateRead();
            if (!committedDeliveryComplete(receipt))
                throw new UiError(joinUiText([uiMessage("knowledge.committedDeliveryPending"), ...knowledgeDeliveryErrors(receipt), uiMessage("common.confirmOriginalOperation")], "\n"));
            await refresh(true);
            pendingPlan = null;
            const completion = joinUiText([uiMessage("knowledge.operationCompleted"), ...knowledgeDeliveryErrors(receipt)], "\n");
            setUiText(status, completion);
            host.notify(completion);
            return true;
        }
        catch (error) {
            if (pendingPreparation && !pendingPreparation.prepared && !retainPendingKnowledgePlan(false, error)) pendingPreparation = null;
            if (pendingPlan && !retainPendingKnowledgePlan(pendingPlan.plan.draftOnly === true, error))
                pendingPlan = null;
            report(error);
            render();
            return false;
        }
        finally {
            busy = false;
            render();
        }
    }
    async function inspectPreparation(): Promise<void> {
        const original = pendingPreparation;
        if (!original || !host.inspectOperation || busy) return;
        busy = true;
        try {
            const receipt = decodeKnowledgeDelivery(await host.inspectOperation(original.operationId, original.context));
            if (receipt.operationId !== original.operationId) throw new UiError(uiMessage("error.invalidResponse"));
            if (receipt.status === "prepared") original.prepared = true;
            else if (receipt.status === "resolved" || committedDeliveryComplete(receipt)) pendingPreparation = null;
            setUiText(status, displayMessage("knowledge.originalOperationStatus", { p0: receipt.status }));
        } catch (error) { report(error); }
        finally { busy = false; render(); }
    }
    async function inspectPending(): Promise<void> {
        return closeGuard.run(() => inspectPendingActive());
    }
    async function inspectPendingActive(): Promise<void> {
        if (busy || !pendingPlan || !host.inspectOperation)
            return;
        const pending = pendingPlan;
        busy = true;
        try {
            const receipt = decodeKnowledgeDelivery(await host.inspectOperation(pending.plan.operationId, pending.context));
            if (pendingPlan !== pending) return;
            if (receipt.operationId !== pending.plan.operationId) throw new UiError(uiMessage("error.invalidResponse"));
            if (pending.receipt && originalCommitted(pending.receipt) && !originalCommitted(receipt))
                throw new UiError(uiMessage("knowledge.committedStatusMismatch"));
            pending.receipt = receipt;
            if (originalCommitted(receipt)) invalidateRead();
            if (committedDeliveryComplete(receipt)) {
                await refresh(true);
                pendingPlan = null;
            }
            else if (receipt.status === "resolved") pendingPlan = null;
            else if (receipt.deliveryError || receipt.documentError)
                throw new UiError(joinUiText([uiMessage("knowledge.committedDeliveryPending"), ...knowledgeDeliveryErrors(receipt)], "\n"));
            setUiText(status, joinUiText([displayMessage("knowledge.originalOperationStatus", { p0: receipt.status }), ...knowledgeDeliveryErrors(receipt)], "\n"));
        }
        catch (error) {
            report(error);
        }
        finally { busy = false; render(); }
    }
    function chooser(fragmentId: string, hit: KnowledgeMention, text: string): HTMLElement {
        const value = element("div", "knowledge-chooser");
        value.append(element("strong", "", text || hit.terms.join(" / ")));
        let selected: NonNullable<KnowledgeMention["candidates"]>[number] | null = null;
        const convert = action(uiMessage("knowledge.convertNativeLink"), async () => {
            if (selected)
                await perform(uiMessage("knowledge.convertPotentialLink"), mentionIntent(fragmentId, hit, selected.target, selected.fragment));
        }, "knowledge-button primary");
        convert.disabled = true;
        const open = action(uiMessage("knowledge.openSelectedCandidate"), async () => {
            if (selected && sameKnowledgeBasis(snapshotBasis, host.context()))
                await host.navigate(`weftext-node://${selected.target.workspaceId}/${selected.target.nodeId}${selected.fragment ? `#${encodeURIComponent(selected.fragment)}` : ""}`);
        });
        open.disabled = true;
        const group = element("div", "knowledge-targets");
        group.setAttribute("role", "radiogroup");
        setUiAttr(group, "aria-label", uiMessage("knowledge.selectExactLinkTarget"));
        const name = `mention-${crypto.randomUUID()}`;
        for (const candidate of hit.candidates ?? []) {
            const target = candidate.target;
            const radio = input();
            radio.type = "radio";
            radio.name = name;
            radio.value = nodeIdentity(target);
            const label = element("label", "knowledge-target");
            label.append(radio, element("span", "", () => `${uiTextValue(nodeLabel(target))}${uiTextValue(candidate.fragment ? ` #${candidate.fragment}` : "")} · ${uiTextValue(({ title: uiMessage("knowledge.noteTitle"), nameField: uiMessage("knowledge.nameField"), heading: uiMessage("knowledge.headingText"), stableLabel: uiMessage("knowledge.stableLabel") })[candidate.provenance] ?? candidate.provenance)}`));
            if (target.path)
                label.append(element("small", "", target.path));
            radio.onchange = () => { selected = candidate; open.disabled = false; convert.disabled = !hit.writable || host.canEditSource?.() === false || !sameKnowledgeBasis(snapshotBasis, host.context()); };
            group.append(label);
            label.append(action(uiMessage("knowledge.excludeTarget"), () => { const source = exclusionScope === "note" ? `${host.context()?.workspaceId}/${host.context()?.nodeId}` : undefined; rules.preview({ kind: "target", value: nodeIdentity(target), source }, displayMessage("knowledge.excludeExactTargetDescription", { p0: uiDisplayParameter(nodeLabel(target)), p1: uiDisplayParameter(nodeIdentity(target)), p2: uiDisplayParameter(source ? uiMessage("knowledge.scopeThisNote") : uiMessage("knowledge.scopePersonalLibrary")) }), knownMentions().filter(m => m.hit.targets.some(t => nodeIdentity(t) === nodeIdentity(target))).length, () => { closePopover(); render(); drawOverlay(); }); }));
        }
        value.append(group);
        if (!hit.writable)
            value.append(element("p", "knowledge-help", uiMessage("knowledge.readOnlyDiscoveryNoWritableRange")));
        else if (host.canEditSource?.() === false)
            value.append(element("p", "knowledge-help", uiMessage("knowledge.readingCanViewCandidates")));
        value.append(open, convert, action(uiMessage("knowledge.ignoreTerm"), () => { const term = text || hit.terms[0], source = exclusionScope === "note" ? `${host.context()?.workspaceId}/${host.context()?.nodeId}` : undefined; rules.preview({ kind: "term", value: term, source }, displayMessage("knowledge.ignoreExactTermDescription", { p0: uiDisplayParameter(term), p1: uiDisplayParameter(source ? uiMessage("knowledge.scopeThisNote") : uiMessage("knowledge.scopePersonalLibrary")) }), knownMentions().filter(m => m.hit.terms.includes(term)).length, () => { closePopover(); render(); drawOverlay(); }); }));
        return value;
    }
    function showMentionPopover(anchor: HTMLElement, fragmentId: string, hit: KnowledgeMention, text: string): void {
        if (popover?.dataset.mention === `${fragmentId}:${hit.startScalar}:${hit.endScalar}`)
            return;
        closePopover();
        popoverAnchor = anchor;
        popover = element("div", "knowledge-popover");
        popover.dataset.mention = `${fragmentId}:${hit.startScalar}:${hit.endScalar}`;
        popover.setAttribute("role", "dialog");
        setUiAttr(popover, "aria-label", uiMessage("knowledge.potentialLinkTargets"));
        popover.append(chooser(fragmentId, hit, text), action(uiMessage("common.close"), () => closePopover(true)));
        document.body.append(popover);
        const rect = anchor.getBoundingClientRect();
        popover.style.left = `${Math.min(Math.max(8, rect.left), Math.max(8, innerWidth - 330))}px`;
        popover.style.top = `${Math.min(rect.bottom + 5, Math.max(8, innerHeight - popover.offsetHeight - 8))}px`;
    }
    function textRange(mapping: KnowledgeTextMapping, start: number, end: number): Range | null {
        if (!isGraphemeRange(mapping.text, start, end) || mapping.node instanceof HTMLTextAreaElement)
            return null;
        const node = mapping.node instanceof Text ? mapping.node : mapping.node.firstChild;
        if (!(node instanceof Text) || node.textContent !== mapping.text)
            return null;
        const range = document.createRange();
        range.setStart(node, start);
        range.setEnd(node, end);
        return range;
    }
    function drawOverlay(): void {
        clearOverlay();
        if (composing || !surface || !sameKnowledgeBasis(snapshotBasis, host.context()) || !surface.isConnected)
            return;
        if (problemHints)
            for (const anchor of surface.querySelectorAll<HTMLAnchorElement>("a[href]")) {
                const issue = snapshot?.diagnostics?.issues.find(i => i.source.nodeId === host.context()?.nodeId && i.occurrence.target === anchor.getAttribute("href"));
                if (!issue)
                    continue;
                const hint = element("span", "knowledge-problem-description", displayMessage("knowledge.linkProblemHint", { p0: uiDisplayParameter(resolutionLabels[issue.resolution.status] ?? issue.resolution.status) }));
                hint.id = `link-problem-${crypto.randomUUID()}`;
                hint.hidden = true;
                hintedLinks.set(anchor, { title: anchor.getAttribute("title"), description: anchor.getAttribute("aria-describedby"), hint });
                setUiAttr(anchor, "title", hint.textContent!);
                anchor.setAttribute("aria-describedby", `${anchor.getAttribute("aria-describedby") ?? ""} ${hint.id}`.trim());
                anchor.classList.add("knowledge-link-problem");
                surface.append(hint);
            }
        if (!enabled)
            return;
        const base = surface.getBoundingClientRect();
        overlay = element("div", "knowledge-overlays");
        setUiAttr(overlay, "aria-label", uiMessage("knowledge.potentialLinkHints"));
        surface.append(overlay);
        const overflow = new Map<string, {
            rect: DOMRect;
            count: number;
        }>();
        for (const { fragment, hit } of knownMentions()) {
            const mapping = mappings.find(item => item.nodeKey === (fragment.nodeKey ?? fragment.id));
            if (!mapping)
                continue;
            const range = textRange(mapping, hit.startUtf16, hit.endUtf16);
            if (!range)
                continue;
            const rects = [...range.getClientRects()];
            if (hit.lane === undefined) {
                const key = `${fragment.id}:${hit.component}`, previous = overflow.get(key);
                const rect = rects.at(-1);
                if (rect)
                    overflow.set(key, { rect, count: (previous?.count ?? 0) + 1 });
                continue;
            }
            for (const rect of rects) {
                if (rect.width <= 0)
                    continue;
                const underline = action("", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16)), "knowledge-underline");
                setUiAttr(underline, "aria-label", displayMessage("knowledge.mentionCandidateTargetCount", { p0: uiDisplayParameter(hit.terms.join(" / ")), p1: hit.targets.length }));
                underline.style.left = `${rect.left - base.left + surface.scrollLeft}px`;
                underline.style.top = `${rect.bottom - base.top + surface.scrollTop + hit.lane * 5 - 3}px`;
                underline.style.width = `${rect.width}px`;
                underline.addEventListener("mouseenter", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16)));
                underline.addEventListener("focus", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16)));
                overlay.append(underline);
                underline.addEventListener("keydown", event => {
                    if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16));
                        popover?.querySelector<HTMLInputElement>('input[type="radio"]')?.focus();
                    }
                });
            }
        }
        for (const { rect, count } of overflow.values()) {
            const more = action(`+${count}`, () => { tab = "mentions"; render(); container.querySelector<HTMLElement>(".knowledge-mentions")?.focus(); }, "knowledge-overflow");
            setUiAttr(more, "aria-label", displayMessage("knowledge.overlappingRangeOverflow", { p0: count }));
            more.style.left = `${rect.right - base.left}px`;
            more.style.top = `${rect.bottom - base.top + 7}px`;
            overlay.append(more);
        }
    }
    const resize = (): void => {
        if (!disposed)
            drawOverlay();
    };
    const compositionStart = (): void => { composing = true; clearOverlay(); };
    const compositionEnd = (): void => { composing = false; };
    const escape = (event: KeyboardEvent): void => {
        if (event.key === "Escape" && popover) {
            closePopover(true);
            event.stopPropagation();
        }
    };
    window.addEventListener("resize", resize);
    window.addEventListener("keydown", escape);
    window.addEventListener("compositionstart", compositionStart);
    window.addEventListener("compositionend", compositionEnd);
    function renderLinks(body: HTMLElement): void {
        ensureRules();
        const d = snapshot?.diagnostics;
        const diagnostics: RelationDiagnostics | undefined = d && { ...d, issues: d.issues.map<RelationIssue>(i => ({ source: relationNode(i.source), target: i.resolution.target ? relationNode(i.resolution.target) : undefined, address: i.occurrence.target, status: i.resolution.status, from: i.occurrence.utf16From, to: i.occurrence.utf16To, sourceHash: i.sourceHash })).concat((d.propertyIssues ?? []).map(p => ({ source: relationNode(p.source), target: p.resolution.target ? relationNode(p.resolution.target) : undefined, address: p.address, status: p.resolution.status, from: undefined, to: undefined, sourceHash: undefined, field: [p.fieldId, ...p.itemPath].join(" / ") }))), nodes: d.nodes.map(n => ({ ...n, node: relationNode(n.node) })) };
        diagnosticView.render(body, diagnostics);
        const hintSetting = element("label", "knowledge-setting"), hintCheck = input();
        hintCheck.type = "checkbox";
        hintCheck.checked = problemHints;
        hintCheck.onchange = () => {
            try {
                localStorage.setItem("weftext-link-problem-hints", String(hintCheck.checked));
                problemHints = hintCheck.checked;
                drawOverlay();
            }
            catch {
                hintCheck.checked = problemHints;
                report(uiMessage("knowledge.issueHintPreferenceSaveFailed"));
            }
        };
        hintSetting.append(hintCheck, uiTextNode(uiMessage("knowledge.showBodyIssueHints")));
        body.append(hintSetting);
        const data = snapshot?.links;
        if (!data) {
            body.append(element("p", "knowledge-help", uiMessage("knowledge.linkIndexUnavailable")));
            return;
        }
        body.append(element("p", "knowledge-help", data.coverage === "partial" ? uiMessage("knowledge.authorizedScanPartialKnownLinks") : uiMessage("knowledge.authorizedScanComplete")));
        const filter = select([["all", uiMessage("graph.allRelationships")], ["links", uiMessage("knowledge.ordinaryNoteLinks")], ["citations", uiMessage("knowledge.bibliographyCitations")], ["document", uiMessage("knowledge.currentNoteCrossReferences")], ["properties", uiMessage("knowledge.propertyRelationships")]] as const);
        filter.value = relationFilter;
        setUiAttr(filter, "aria-label", uiMessage("knowledge.relationshipCategory"));
        filter.onchange = () => { relationFilter = filter.value; render(); };
        body.append(filter);
        if (relationFilter === "citations") {
            const context = host.context()!;
            let state = citationInputs.get(context.sessionId);
            if (!state) {
                state = newNativeCitationInputs();
                citationInputs.set(context.sessionId, state);
            }
            renderNativeCitations(body, snapshot?.citations, state, { writable: host.canEditSource?.() !== false, at: () => citationInsertionByte(context.source, context.selection?.from ?? context.source.length), perform: (title, intent) => perform(title, intent as KnowledgeIntent), select: range => host.selectSource?.(range.utf16From, range.utf16To), navigate, repaint: render });
            return;
        }
        if (relationFilter === "all" || relationFilter === "document") {
            const group = section(uiMessage("knowledge.inDocumentAndUnclassifiedReferences"), uiMessage("knowledge.referenceClassificationDescription"));
            for (const ref of snapshot?.citations?.editor.crossReferences ?? []) {
                if (ref.renderedKind === "citation")
                    continue;
                group.append(action(ref.label, () => host.selectSource?.(ref.range.utf16From, ref.range.utf16To)), element("span", "knowledge-badge", ref.renderedKind === "document" ? uiMessage("knowledge.currentNativeInternalReference") : ref.kind === "link" ? uiMessage("knowledge.internalLinkSourceSlot") : uiMessage("knowledge.nativeReferenceSourceUnclassified")));
            }
            body.append(group);
        }
        for (const [title, entries] of (relationFilter === "all" || relationFilter === "links" ? [[uiMessage("knowledge.outgoingLinks"), data.outgoing], [uiMessage("knowledge.idBacklinks"), data.incomingId], [uiMessage("knowledge.currentPathResolvedIncoming"), data.incomingPath]] as const : [])) {
            const group = section(title);
            body.append(group);
            if (!entries.length) {
                group.append(element("p", "knowledge-empty", data.coverage === "complete" ? uiMessage("knowledge.scopeHasNoLinks") : uiMessage("knowledge.noneFoundScanIncomplete")));
                continue;
            }
            for (const entry of entries) {
                const row = element("article", "knowledge-card");
                const target = entry.resolution.target;
                if (target)
                    setUiAttr(row, "title", displayMessage("knowledge.authorizedTitleOriginalBodyText", { p0: uiDisplayParameter(nodeLabel(target)) }));
                row.append(action(title === uiMessage("knowledge.outgoingLinks") ? target ? nodeLabel(target) : entry.occurrence.target : nodeLabel(entry.source), () => navigate(title === uiMessage("knowledge.outgoingLinks") ? entry.occurrence.target : entry.source), "knowledge-link-title"));
                row.append(element("span", "knowledge-badge", resolutionLabels[entry.resolution.status] ?? entry.resolution.status), element("p", "knowledge-address", entry.occurrence.target));
                const controls = element("div", "knowledge-actions");
                controls.append(action(uiMessage("knowledge.copyAddress"), () => copy(entry.occurrence.target)));
                const raw = entry.occurrence.raw ?? (snapshotBasis && entry.source.nodeId === snapshotBasis.nodeId ? snapshotBasis.source.slice(entry.occurrence.utf16From, entry.occurrence.utf16To) : undefined);
                if (raw !== undefined)
                    controls.append(action(uiMessage("knowledge.copyCompleteLink"), () => copy(raw)));
                if (title === uiMessage("knowledge.outgoingLinks")) {
                    const format = select(Object.entries(formLabels));
                    setUiAttr(format, "aria-label", uiMessage("knowledge.linkConversionForm"));
                    if (entry.resolution.form)
                        format.value = entry.resolution.form;
                    const convert = action(uiMessage("knowledge.convert"), () => perform(uiMessage("knowledge.convertLinkForm"), { kind: "linkConvert", generation: data.generation, occurrence: entry.occurrence, form: format.value }));
                    convert.disabled = entry.resolution.status !== "resolved" || !entry.occurrence.destinationSlot || host.canEditSource?.() === false;
                    controls.append(format, convert);
                    if (host.linkCopy) {
                        const copyForm = async (full: boolean): Promise<void> => {
                            const context = host.context();
                            if (!context || !sameKnowledgeBasis(snapshotBasis, context)) {
                                report(uiMessage("knowledge.linkChangedBeforeCopy"));
                                return;
                            }
                            try {
                                const result = await host.linkCopy!(entry, format.value, context);
                                await copy(full ? result.fullLink : result.address);
                            }
                            catch (error) {
                                report(error);
                            }
                        };
                        controls.append(action(uiMessage("knowledge.copySelectedAddress"), () => copyForm(false)), action(uiMessage("knowledge.copySelectedCompleteLink"), () => copyForm(true)));
                        const stableCopy = async (full: boolean): Promise<void> => {
                            const context = host.context();
                            if (!context || !sameKnowledgeBasis(snapshotBasis, context))
                                throw new UiError(uiMessage("knowledge.linkBasisChangedBeforeCopy"));
                            try {
                                const result = await host.linkCopy!(entry, "id", context);
                                await copy(full ? result.fullLink : result.address);
                            }
                            catch (error) {
                                report(error);
                            }
                        };
                        controls.append(action(uiMessage("knowledge.copyStableAppUri"), () => stableCopy(false)), action(uiMessage("knowledge.copyStableNativeTypstLink"), () => stableCopy(true)));
                        const web = action(uiMessage("knowledge.copyWebDeepLinkUnavailable"), () => { });
                        web.disabled = true;
                        setUiAttr(web, "title", uiMessage("knowledge.webDeepLinkRoutesUnconfigured"));
                        controls.append(web);
                    }
                    const updateText = action(uiMessage("knowledge.updateLinkTextToCurrentTitle"), () => perform(uiMessage("knowledge.updateSelectedLinkDisplayText"), { kind: "nativeCitationLinkText", generation: snapshot?.citations?.generation, occurrence: entry.occurrence }));
                    updateText.disabled = host.canEditSource?.() === false || !target || !entry.occurrence.destinationSlot;
                    controls.append(updateText);
                    if (convert.disabled)
                        row.append(element("p", "knowledge-help", host.canEditSource?.() === false ? uiMessage("knowledge.readingCanCopyLinks") : uiMessage("knowledge.linkQuickConversionUnavailable")));
                }
                row.append(controls);
                group.append(row);
            }
        }
        const displayed = section(uiMessage("knowledge.linksInReadingPreview"), uiMessage("knowledge.compiledDisplayLinksDescription"));
        if (relationFilter === "all" || relationFilter === "links")
            body.append(displayed);
        const rendered = snapshot?.renderedLinks ?? [];
        if (!rendered.length)
            displayed.append(element("p", "knowledge-empty", uiMessage("knowledge.previewNoDisplayLinks")));
        for (const link of rendered) {
            const card = element("article", "knowledge-card");
            card.append(action(link.destination, () => navigate(link.destination), "knowledge-link-title"), element("span", "knowledge-badge", displayMessage("knowledge.readOnlyDisplayOccurrence", { p0: link.occurrenceOrdinal + 1 })), action(uiMessage("knowledge.copyDisplayedAddress"), () => copy(link.destination)));
            displayed.append(card);
        }
        const properties = snapshot?.propertyReferences;
        const context = host.context();
        const identity = context?.workspaceId && context.nodeId ? `${context.workspaceId}/${context.nodeId}` : null;
        if (properties && identity && (relationFilter === "all" || relationFilter === "properties")) {
            const own = properties.items.filter(item => nodeIdentity(item.source) === identity);
            const incoming = properties.items.filter(item => item.resolution.target && nodeIdentity(item.resolution.target) === identity);
            for (const [title, items, outgoing] of [[uiMessage("knowledge.outgoingPropertyReferences"), own, true], [uiMessage("knowledge.incomingPropertyReferences"), incoming, false]] as const) {
                const group = section(title, properties.coverage === "complete" ? uiMessage("knowledge.structuredReferencesDescription") : uiMessage("knowledge.propertyReferenceScanPartial"));
                body.append(group);
                if (!items.length)
                    group.append(element("p", "knowledge-empty", properties.coverage === "complete" ? uiMessage("knowledge.scopeHasNoPropertyReferences") : uiMessage("knowledge.noneFoundScanIncomplete")));
                for (const item of items) {
                    const target = outgoing ? item.resolution.target : item.source;
                    const card = element("article", "knowledge-card");
                    card.append(element("strong", "", item.fieldTitle || uiMessage("knowledge.structuredProperty")), element("span", "knowledge-badge", resolutionLabels[item.resolution.status] ?? item.resolution.status));
                    if (target)
                        card.append(action(nodeLabel(target), () => navigate(target), "knowledge-link-title"));
                    const details = element("details", "knowledge-diagnostics");
                    details.append(element("summary", "", uiMessage("knowledge.fieldLocationDetails")), element("pre", "knowledge-address", [item.fieldId, ...item.itemPath].join(" / ")));
                    card.append(details);
                    group.append(card);
                }
            }
            for (const diagnostic of properties.diagnostics ?? [])
                body.append(element("p", "knowledge-warning", descriptionUiText(diagnostic)));
        }
    }
    function renderMentions(body: HTMLElement): void {
        ensureRules();
        const setting = element("label", "knowledge-setting");
        const check = input();
        check.type = "checkbox";
        check.checked = enabled;
        check.onchange = () => {
            try {
                localStorage.setItem("weftext-potential-links", String(check.checked));
                enabled = check.checked;
                drawOverlay();
            }
            catch {
                check.checked = enabled;
                report(uiMessage("knowledge.mentionHintPreferenceSaveFailed"));
            }
        };
        setting.append(check, uiTextNode(uiMessage("knowledge.showBodyDoubleUnderlineHints")));
        body.append(setting);
        const domain = select([["all", uiMessage("knowledge.titlesNamesStableFragments")], ["notes", uiMessage("knowledge.noteTitlesNamesOnly")], ["fragments", uiMessage("knowledge.stableFragmentsOnly")]] as const);
        setUiAttr(domain, "aria-label", uiMessage("knowledge.candidateScope"));
        domain.value = mentionDomain;
        domain.onchange = () => { mentionDomain = domain.value; render(); drawOverlay(); };
        const scope = select([["note", uiMessage("knowledge.scopeThisNote")], ["library", uiMessage("knowledge.scopePersonalLibrary")]] as const);
        setUiAttr(scope, "aria-label", uiMessage("knowledge.exclusionRuleScope"));
        scope.value = exclusionScope;
        scope.onchange = () => { exclusionScope = scope.value; };
        body.append(field(uiMessage("knowledge.candidateScope"), domain), field(uiMessage("knowledge.exclusionRuleScope"), scope), element("p", "knowledge-help", uiMessage("knowledge.mentionScanDescription")));
        rules.controls(body, () => { render(); drawOverlay(); });
        body.append(element("p", "knowledge-help", uiMessage("knowledge.allMentionTargetsListed")));
        const list = element("div", "knowledge-mentions");
        list.tabIndex = -1;
        body.append(list);
        if (!snapshot?.mentions) {
            list.append(element("p", "knowledge-empty", uiMessage("knowledge.mentionScanUnavailable")));
            return;
        }
        list.append(element("p", "knowledge-help", snapshot.mentions.coverage === "partial" ? uiMessage("knowledge.mentionScanPartialAllKnown") : uiMessage("knowledge.authorizedScanCompleted")));
        const hits = knownMentions();
        if (!hits.length)
            list.append(element("p", "knowledge-empty", snapshot.mentions.coverage === "complete" ? uiMessage("knowledge.scopeHasNoPotentialLinks") : uiMessage("knowledge.noneFoundScanIncomplete")));
        for (const { fragment, hit } of hits) {
            const card = element("article", "knowledge-card");
            card.append(element("span", "knowledge-badge", () => `[${uiTextValue(hit.startScalar)}, ${uiTextValue(hit.endScalar)})${uiTextValue(hit.lane === undefined ? uiMessage("knowledge.moreOverlapsSuffix") : "")}`), chooser(fragment.id, hit, fragment.text?.slice(hit.startUtf16, hit.endUtf16) ?? hit.terms.join(" / ")));
            list.append(card);
        }
    }
    function mediaEntry(body: HTMLElement): void {
        const group = section(uiMessage("knowledge.resourceRegion"), uiMessage("knowledge.selectActualResourceRegionDescription"));
        body.append(group);
        const sessionId = host.context()?.sessionId ?? null;
        if (sessionId !== mediaSession) {
            mediaController?.dispose();
            mediaController = null;
            mediaContainer.replaceChildren();
            mediaSession = sessionId;
            mediaDraft = null;
        }
        if (host.mountMedia && !mediaController)
            mediaController = host.mountMedia(mediaContainer, target => { closeGuard.assertMutable(); mediaDraft = target; authorProposal = true; }, mediaDraft ?? undefined);
        if (!host.mountMedia)
            group.append(element("p", "knowledge-empty", uiMessage("knowledge.resourcePreviewUnavailable")));
        else
            group.append(mediaContainer);
    }
    function renderAnnotations(body: HTMLElement): void {
        const diagnostics = snapshot?.annotationDiagnostics ?? [];
        if (snapshot?.annotationStatus && !["ready", "valid", "available", "missing"].includes(snapshot.annotationStatus))
            body.append(element("p", "knowledge-warning", displayMessage("knowledge.annotationFileStatus", { p0: uiDisplayParameter(snapshot.annotationStatus) })));
        for (const diagnostic of diagnostics)
            body.append(element("p", "knowledge-warning", apiErrorUiText(diagnostic)));
        const create = section(uiMessage("knowledge.addIndependentAnnotation"));
        body.append(create);
        const target = select([["whole_document", uiMessage("hostedReview.wholeNoteTarget")], ["source_range", uiMessage("knowledge.currentExactSelection")], ["source_element", uiMessage("knowledge.currentSelectedElement")], ["media", uiMessage("knowledge.mediaRegionTarget")]] as const);
        target.value = annotationTarget;
        target.onchange = () => {
            annotationTarget = target.value;
            if (annotationTarget === "source_range" || annotationTarget === "source_element") {
                annotationSelectionBasis = host.context();
                annotationSelection = annotationSelectionBasis?.selection ? { ...annotationSelectionBasis.selection } : undefined;
            }
            render();
        };
        create.append(field(uiMessage("knowledge.target"), target));
        if (annotationTarget === "source_range" || annotationTarget === "source_element") {
            const capture = action(uiMessage("knowledge.captureCurrentSelection"), () => { annotationSelectionBasis = host.context(); annotationSelection = annotationSelectionBasis?.selection ? { ...annotationSelectionBasis.selection } : undefined; render(); });
            create.append(capture);
            create.append(element("p", "knowledge-help", annotationSelection ? displayMessage("knowledge.sourceRangeValidationHint", { p0: annotationSelection.from, p1: annotationSelection.to, p2: uiDisplayParameter(host.context()?.dirty ? uiMessage("knowledge.saveSourceThenReselect") : uiMessage("knowledge.revalidateExactSourceOnCommit")) }) : uiMessage("knowledge.selectExactSourceRange")));
            if (annotationSelection?.visibleExcerpt)
                create.append(element("blockquote", "knowledge-excerpt", annotationSelection.visibleExcerpt));
        }
        if (annotationTarget === "media")
            mediaEntry(create);
        const kind = select([["comment", uiMessage("hostedReview.commentPurpose")], ["mark", uiMessage("hostedReview.markPurpose")], ["suggestion", uiMessage("hostedReview.suggestionPurpose")]] as const);
        kind.value = annotationKind;
        kind.onchange = () => { annotationKind = kind.value; render(); };
        create.append(field(uiMessage("knowledge.purpose"), kind));
        const entry = textarea(annotationBody, uiMessage("knowledge.discussionPlaceholder"));
        entry.oninput = () => { annotationBody = entry.value; };
        create.append(field(uiMessage("hostedReview.bodyLabel"), entry));
        if (annotationKind === "suggestion") {
            const replacement = textarea(suggestionReplacement, uiMessage("knowledge.replacementTextPlaceholder"));
            replacement.oninput = () => { suggestionReplacement = replacement.value; };
            create.append(field(uiMessage("hostedReview.replacementLabel"), replacement), element("p", "knowledge-help", uiMessage("knowledge.acceptSuggestionTransactionDescription")));
        }
        create.append(action(uiMessage("knowledge.prepareSaveAnnotation"), async () => {
            if (!annotationBody.trim()) {
                report(uiMessage("knowledge.annotationContentRequired"));
                return;
            }
            let annotationAnchor: unknown = { kind: "whole_document" };
            if (annotationTarget === "media") {
                if (!mediaDraft) {
                    report(uiMessage("knowledge.selectExactResourceRegion"));
                    return;
                }
                const error = validateMediaRegion(mediaDraft);
                if (error) {
                    report(error);
                    return;
                }
                annotationAnchor = { ...mediaDraft };
            }
            else if (annotationTarget !== "whole_document") {
                const context = host.context();
                if (!context || !annotationSelection || !sameKnowledgeBasis(annotationSelectionBasis, context) || !isGraphemeRange(context.source, annotationSelection.from, annotationSelection.to)) {
                    report(uiMessage("knowledge.selectionSourceVersionChanged"));
                    return;
                }
                if (annotationTarget === "source_element" && !annotationSelection.elementRole) {
                    report(uiMessage("knowledge.selectionElementRoleUnverified"));
                    return;
                }
                annotationAnchor = { kind: annotationTarget, ...annotationSelection };
            }
            if (await perform(uiMessage("knowledge.saveIndependentAnnotation"), { kind: "annotationCreate", target: annotationAnchor, body: annotationBody, purpose: annotationKind, replacement: annotationKind === "suggestion" ? suggestionReplacement : undefined })) {
                annotationBody = "";
                suggestionReplacement = "";
                render();
            }
        }, "knowledge-button primary"));
        const threads = section(uiMessage("knowledge.discussionsAndSuggestions"));
        body.append(threads);
        if (!snapshot?.annotations?.length)
            threads.append(element("p", "knowledge-empty", uiMessage("knowledge.noReadableAnnotations")));
        for (const annotation of snapshot?.annotations ?? []) {
            const id = annotation.annotationId ?? annotation.id;
            const card = element("article", "knowledge-card");
            card.append(element("span", "knowledge-badge", annotation.reviewState === "resolved" ? uiMessage("hostedReview.discussionResolved") : uiMessage("hostedReview.discussionUnresolved")), element("span", `knowledge-badge ${annotation.targetResolution === "exact" || annotation.targetResolution === "mapped" ? "" : "warning"}`, resolutionLabels[annotation.targetResolution ?? "unavailable"] ?? uiMessage("knowledge.targetUnavailable")));
            if (annotation.targetSummary)
                card.append(element("p", "knowledge-help", annotation.targetSummary));
            if (excerpt(annotation))
                card.append(element("blockquote", "knowledge-excerpt", excerpt(annotation)));
            card.append(element("p", "knowledge-annotation-body", annotation.body));
            if (annotation.author || annotation.createdAt)
                card.append(element("p", "knowledge-help", [annotation.author, annotation.createdAt].filter(Boolean).join(" · ")));
            if (annotation.purpose === "suggestion") {
                card.append(element("span", "knowledge-badge", ({ pending: uiMessage("hostedReview.suggestionPending"), accepted: uiMessage("hostedReview.suggestionAccepted"), rejected: uiMessage("hostedReview.suggestionRejected") })[annotation.suggestionState ?? "pending"]));
                if (annotation.replacement !== undefined)
                    card.append(element("pre", "knowledge-replacement", annotation.replacement));
            }
            const controls = element("div", "knowledge-actions");
            controls.append(action(annotation.reviewState === "resolved" ? uiMessage("knowledge.reopenDiscussion") : uiMessage("knowledge.resolveDiscussion"), () => perform(uiMessage("knowledge.updateDiscussionStatus"), { kind: "annotationResolve", annotationId: id, resolved: annotation.reviewState !== "resolved" })));
            if (annotation.targetResolution === "exact" || annotation.targetResolution === "mapped") {
                if (annotation.sourceLocation && host.selectSource)
                    controls.append(action(uiMessage("knowledge.returnToTarget"), () => {
                        if (!sameKnowledgeBasis(snapshotBasis, host.context())) {
                            report(uiMessage("knowledge.sourceVersionChangedDiscussionRetained"));
                            return;
                        }
                        host.selectSource!(annotation.sourceLocation!.from, annotation.sourceLocation!.to);
                    }));
                else if (annotation.navigateTarget)
                    controls.append(action(uiMessage("knowledge.returnToTarget"), () => navigate(annotation.navigateTarget!)));
                else if (annotation.resourceLocation)
                    controls.append(action(uiMessage("knowledge.returnToMediaTarget"), () => {
                        if (!sameKnowledgeBasis(snapshotBasis, host.context())) {
                            report(uiMessage("knowledge.currentNoteChangedDiscussionRetained"));
                            return;
                        }
                        const location = annotation.resourceLocation!, target = storedMediaSelection(location.target);
                        if (!target || target.resource !== location.resource || target.revision !== location.revision) {
                            report(uiMessage("knowledge.exactMediaTargetUnavailable"));
                            return;
                        }
                        mediaController?.dispose();
                        mediaController = null;
                        mediaContainer.replaceChildren();
                        mediaDraft = target; authorProposal = true;
                        annotationTarget = "media";
                        tab = "annotations";
                        render();
                        mediaContainer.scrollIntoView({ block: "nearest" });
                    }));
            }
            else if (!(annotation.purpose === "suggestion" && annotation.suggestionState === "pending"))
                controls.append(action(uiMessage("knowledge.relocateToCurrentTarget"), async () => {
                    if (annotationTarget === "media") {
                        if (!mediaDraft || validateMediaRegion(mediaDraft)) {
                            report(uiMessage("knowledge.selectCurrentExactResourceRegion"));
                            return;
                        }
                        await perform(uiMessage("hostedReview.reattachOperation"), { kind: "annotationReattach", annotationId: id, target: { ...mediaDraft } });
                        return;
                    }
                    const selection = host.context()?.selection;
                    if (!selection) {
                        report(uiMessage("knowledge.selectCurrentExactTarget"));
                        return;
                    }
                    await perform(uiMessage("hostedReview.reattachOperation"), { kind: "annotationReattach", annotationId: id, target: { kind: "source_range", ...selection } });
                }));
            if (annotation.purpose === "suggestion" && annotation.suggestionState === "pending") {
                const accept = action(uiMessage("knowledge.checkAndAccept"), () => perform(uiMessage("knowledge.applySuggestionAndAcceptance"), { kind: "annotationAccept", annotationId: id }));
                accept.disabled = !canApplySuggestion(annotation);
                controls.append(accept, action(uiMessage("hostedReview.rejectOperation"), () => perform(uiMessage("knowledge.rejectChangeSuggestion"), { kind: "annotationReject", annotationId: id })));
                let draft = reconfirmations.get(id);
                if (!draft) {
                    draft = { replacement: annotation.replacement ?? "", basis: null };
                    reconfirmations.set(id, draft);
                }
                const reconsider = element("details", "knowledge-reconfirm");
                reconsider.append(element("summary", "", uiMessage("knowledge.reconfirmSuggestionTargetText")), element("p", "knowledge-help", uiMessage("knowledge.reconfirmSuggestionDescription")));
                const replacement = textarea(draft.replacement);
                replacement.oninput = () => { draft!.replacement = replacement.value; };
                reconsider.append(field(uiMessage("knowledge.reconfirmedSuggestionText"), replacement));
                reconsider.append(action(uiMessage("knowledge.captureCurrentExactSelection"), () => {
                    const context = host.context();
                    draft!.basis = context;
                    draft!.selection = context?.selection ? { ...context.selection } : undefined;
                    const caption = reconsider.querySelector<HTMLElement>(".knowledge-reconfirm-selection");
                    if (caption)
                        setUiText(caption, draft!.selection ? displayMessage("knowledge.selectedUtf16Range", { p0: draft!.selection.from, p1: draft!.selection.to }) : uiMessage("knowledge.noExactSelection"));
                }));
                reconsider.append(element("p", "knowledge-help knowledge-reconfirm-selection", draft.selection ? displayMessage("knowledge.selectedUtf16Range", { p0: draft.selection.from, p1: draft.selection.to }) : uiMessage("knowledge.selectTargetBeforeCapture")));
                reconsider.append(action(uiMessage("knowledge.prepareReconfirmation"), async () => {
                    const context = host.context();
                    const selection = draft!.selection;
                    if (!context || context.dirty || !selection || !sameKnowledgeBasis(draft!.basis, context) || !isGraphemeRange(context.source, selection.from, selection.to, true) || selection.from === selection.to && !draft!.replacement) {
                        report(uiMessage("knowledge.saveSourceReselectSuggestion"));
                        return;
                    }
                    await perform(uiMessage("knowledge.reconfirmSuggestionTargetAndText"), { kind: "annotationReconfirm", annotationId: id, target: { kind: selection.from === selection.to ? "source_point" : "source_range", ...selection }, replacement: draft!.replacement });
                }));
                card.append(reconsider);
            }
            card.append(controls);
            for (const reply of annotation.replies ?? []) {
                const value = element("div", "knowledge-reply");
                value.append(element("p", "knowledge-annotation-body", reply.body));
                if (reply.author || reply.createdAt)
                    value.append(element("small", "", [reply.author, reply.createdAt].filter(Boolean).join(" · ")));
                card.append(value);
            }
            const reply = textarea(replies.get(id) ?? "", uiMessage("knowledge.replyPlaceholder"));
            setUiAttr(reply, "aria-label", displayMessage("knowledge.replyAnnotationLabel", { p0: uiDisplayParameter(id) }));
            reply.oninput = () => replies.set(id, reply.value);
            card.append(reply, action(uiMessage("knowledge.saveReply"), async () => {
                const text = replies.get(id) ?? "";
                if (!text.trim())
                    return;
                if (await perform(uiMessage("knowledge.saveAnnotationReply"), { kind: "annotationReply", annotationId: id, body: text })) {
                    replies.delete(id);
                    render();
                }
            }));
            threads.append(card);
        }
    }
    function renderFiles(body: HTMLElement): void {
        const context = host.context();
        if (!context)
            return;
        const managed = Boolean(context.nodeId && context.workspaceId);
        const nodeDirectory = context.path.replace(/[/\\][^/\\]+$/, "");
        let draft = fileDrafts.get(context.sessionId);
        if (!draft) {
            draft = { name: context.path.replaceAll("\\", "/").split("/").at(-1)?.replace(/\.typ$/i, "") ?? "", parent: context.root, backup: "", allowLosses: false, restores: new Map() };
            fileDrafts.set(context.sessionId, draft);
        }
        const group = section(uiMessage("knowledge.noteNodeActions"), managed ? uiMessage("knowledge.nodeActionsDescription") : uiMessage("knowledge.nodeActionsRequireLibraryNote"));
        body.append(group);
        const name = input(draft.name);
        name.oninput = () => { draft!.name = name.value; };
        const parent = input(draft.parent);
        parent.oninput = () => { draft!.parent = parent.value; };
        group.append(field(uiMessage("knowledge.newOrCopyName"), name), field(uiMessage("knowledge.targetParentFullPath"), parent));
        const lossChoice = element("label", "knowledge-setting");
        const allowLosses = input();
        allowLosses.type = "checkbox";
        allowLosses.checked = draft.allowLosses;
        allowLosses.onchange = () => { draft!.allowLosses = allowLosses.checked; };
        lossChoice.append(allowLosses, uiTextNode(uiMessage("knowledge.copyRetainUnmigratedReferences")));
        group.append(lossChoice, element("p", "knowledge-help", uiMessage("knowledge.copyLossDescription")));
        if (managed && context.dirty)
            group.append(element("p", "knowledge-warning", uiMessage("knowledge.saveDraftBeforeNodeAction")));
        const actions = element("div", "knowledge-actions");
        for (const [label, kind] of [[uiMessage("tagNavigation.rename"), "renameNode"], [uiMessage("hostedStructure.moveKind"), "moveNode"], [uiMessage("knowledge.copyIndependentNode"), "copyNode"], [uiMessage("knowledge.operationRecycle"), "trashNode"]] as const) {
            const control = action(label, () => perform(label, { kind, source: nodeDirectory, ...(kind === "renameNode" || kind === "copyNode" ? { name: name.value } : {}), ...(kind === "moveNode" || kind === "copyNode" ? { parent: parent.value } : {}), ...(kind === "copyNode" && draft!.allowLosses ? { allowLosses: true } : {}) }));
            control.disabled = !managed || context.dirty;
            actions.append(control);
        }
        group.append(actions);
        const backup = section(uiMessage("knowledge.libraryBackup"), uiMessage("knowledge.libraryBackupDescription"));
        const destination = input(draft.backup, uiMessage("knowledge.backupPathPlaceholder"));
        destination.oninput = () => { draft!.backup = destination.value; };
        backup.append(field(uiMessage("knowledge.backupTargetFullPath"), destination), action(uiMessage("knowledge.prepareBackup"), () => perform(uiMessage("knowledge.createLibraryBackup"), { kind: "backupLibrary", destination: destination.value })));
        body.append(backup);
        const trash = section(uiMessage("knowledge.recycleBin"), uiMessage("knowledge.recycleBinDescription"));
        body.append(trash);
        if (!snapshot?.trash?.length)
            trash.append(element("p", "knowledge-empty", uiMessage("knowledge.noReadableRecoveryRecords")));
        for (const entry of snapshot?.trash ?? []) {
            const card = element("article", "knowledge-card");
            const restoreDraft = trashRestoreDraft(draft.restores, context.root, entry);
            const restoreName = input(restoreDraft.name), restoreParent = input(restoreDraft.parent);
            restoreName.oninput = () => { restoreDraft.name = restoreName.value; };
            restoreParent.oninput = () => { restoreDraft.parent = restoreParent.value; };
            card.append(element("strong", "", entry.name));
            if (entry.path)
                card.append(element("p", "knowledge-address", entry.path));
            card.append(field(uiMessage("knowledge.restoreName"), restoreName), field(uiMessage("knowledge.restoreParentDirectory"), restoreParent), action(uiMessage("knowledge.prepareRestore"), () => perform(uiMessage("knowledge.restoreNoteNode"), { kind: "restoreNode", trashOperationId: entry.operationId, ...(entry.nodeId !== undefined ? { nodeId: entry.nodeId } : {}), parent: restoreParent.value, name: restoreName.value })));
            trash.append(card);
        }
    }
    function templateValueControl(kind: string, value: KnowledgeTemplateValue | undefined, changed: (value: KnowledgeTemplateValue) => void): HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement {
        if (kind === "bool") {
            const control = select([["false", uiMessage("nativeValue.booleanNo")], ["true", uiMessage("nativeValue.booleanYes")]] as const);
            control.value = value === true ? "true" : "false";
            control.onchange = () => changed(control.value === "true");
            return control;
        }
        if (kind === "int") {
            const control = input(typeof value === "string" ? value : "");
            control.inputMode = "numeric";
            control.pattern = "[+-]?[0-9]+";
            setUiAttr(control, "placeholder", uiMessage("knowledge.exactIntegerPlaceholder"));
            control.oninput = () => changed(control.value);
            return control;
        }
        const control = textarea(typeof value === "string" ? value : "");
        control.rows = kind === "markup" ? 4 : 2;
        control.oninput = () => changed(control.value);
        return control;
    }
    function renderTemplateRegistration(body: HTMLElement): void {
        const context = host.context();
        if (!context)
            return;
        let draft = registrationDrafts.get(context.sessionId);
        if (!draft) {
            draft = { title: "", description: "", basis: null, parameters: [], open: false, slotsOpen: false, slot: { name: "", kind: "markup", nullable: false, defaultMode: "omitted", defaultValue: "" } };
            registrationDrafts.set(context.sessionId, draft);
        }
        const registration = element("details", "knowledge-template-registration");
        registration.open = draft.open;
        registration.ontoggle = () => {
            if (registration.isConnected)
                draft!.open = registration.open;
        };
        registration.append(element("summary", "", draft.templateId ? uiMessage("knowledge.updateNativeTemplateRegistration") : uiMessage("knowledge.registerCurrentNoteAsTemplate")));
        body.append(registration);
        const group = section(uiMessage("knowledge.nativeTemplateRegistration"), uiMessage("knowledge.templateRegistrationDescription"));
        registration.append(group);
        const title = input(draft.title), description = textarea(draft.description);
        title.oninput = () => { draft!.title = title.value; };
        description.oninput = () => { draft!.description = description.value; };
        group.append(field(uiMessage("knowledge.templateNameLabel"), title), field(uiMessage("knowledge.templatePurposeLabel"), description));
        if (context.dirty)
            group.append(element("p", "knowledge-warning", uiMessage("knowledge.saveBeforeTemplateRegistration")));
        const details = element("details");
        details.open = draft.slotsOpen;
        details.ontoggle = () => {
            if (details.isConnected)
                draft!.slotsOpen = details.open;
        };
        details.append(element("summary", "", displayMessage("knowledge.parameterRangeCount", { p0: draft.parameters.length })));
        group.append(details);
        for (const [index, parameter] of draft.parameters.entries()) {
            const row = element("article", "knowledge-card");
            row.append(element("strong", "", parameter.name), element("p", "knowledge-help", () => `${uiTextValue(({ markup: uiMessage("knowledge.parameterMarkup"), string: uiMessage("knowledge.parameterStringLiteral"), bool: uiMessage("knowledge.parameterBooleanLiteral"), int: uiMessage("knowledge.parameterIntegerLiteral") })[parameter.kind])} · UTF-16 [${uiTextValue(parameter.from)}, ${uiTextValue(parameter.to)})`), action(uiMessage("knowledge.removeParameter"), () => { authorProposal = true; draft!.parameters.splice(index, 1); render(); }));
            details.append(row);
        }
        const slotName = input(draft.slot.name, uiMessage("knowledge.parameterNamePlaceholder"));
        slotName.oninput = () => { draft!.slot.name = slotName.value; };
        const kind = select([["markup", uiMessage("knowledge.parameterMarkup")], ["string", uiMessage("knowledge.completeStringLiteral")], ["bool", uiMessage("knowledge.parameterBooleanLiteral")], ["int", uiMessage("knowledge.parameterIntegerLiteral")]] as const);
        kind.dataset.knowledgeFocus = "template-register-kind";
        kind.value = draft.slot.kind;
        kind.onchange = () => {
            draft!.slot.kind = kind.value as RegistrationSlot["kind"];
            draft!.slot.defaultValue = kind.value === "bool" ? false : "";
            if (kind.value === "markup") {
                draft!.slot.nullable = false;
                if (draft!.slot.defaultMode === "none")
                    draft!.slot.defaultMode = "omitted";
            }
            render();
        };
        details.append(field(uiMessage("knowledge.parameterName"), slotName), field(uiMessage("knowledge.parameterType"), kind));
        const nullable = element("label", "knowledge-setting"), nullCheck = input();
        nullCheck.dataset.knowledgeFocus = "template-register-nullable";
        nullCheck.type = "checkbox";
        nullCheck.checked = draft.slot.nullable;
        nullCheck.disabled = draft.slot.kind === "markup";
        nullCheck.onchange = () => {
            draft!.slot.nullable = nullCheck.checked;
            if (!nullCheck.checked && draft!.slot.defaultMode === "none")
                draft!.slot.defaultMode = "omitted";
            render();
        };
        nullable.append(nullCheck, uiTextNode(uiMessage("knowledge.allowNone")));
        details.append(nullable);
        const defaultChoices: [
            string,
            UiText
        ][] = [["omitted", uiMessage("knowledge.requiredNoDefault")], ["value", uiMessage("knowledge.provideDefault")]] as const;
        if (draft.slot.nullable)
            defaultChoices.push(["none", uiMessage("knowledge.defaultNone")]);
        const mode = select(defaultChoices);
        mode.dataset.knowledgeFocus = "template-register-default-mode";
        mode.value = draft.slot.defaultMode;
        mode.onchange = () => { draft!.slot.defaultMode = mode.value as "omitted" | "value" | "none"; render(); };
        details.append(field(uiMessage("knowledge.defaultValue"), mode));
        if (draft.slot.defaultMode === "value")
            details.append(field(uiMessage("knowledge.defaultContent"), templateValueControl(draft.slot.kind, draft.slot.defaultValue, value => { draft!.slot.defaultValue = value; })));
        details.append(action(uiMessage("knowledge.addSelectionAsParameter"), () => {
            const current = host.context(), selection = current?.selection;
            if (!current || current.dirty || !selection || !isGraphemeRange(current.source, selection.from, selection.to)) {
                report(uiMessage("knowledge.saveSelectCompleteParameterRange"));
                return;
            }
            if (draft!.basis && !sameKnowledgeBasis(draft!.basis, current)) {
                report(uiMessage("knowledge.existingParameterSourceChanged"));
                return;
            }
            if (!draft!.slot.name.trim() || draft!.parameters.some(parameter => parameter.name === draft!.slot.name)) {
                report(uiMessage("knowledge.unusedParameterNameRequired"));
                return;
            }
            if (draft!.parameters.some(parameter => parameter.from < selection.to && selection.from < parameter.to)) {
                report(uiMessage("knowledge.parameterRangeOverlap"));
                return;
            }
            if (draft!.slot.kind === "int" && draft!.slot.defaultMode === "value" && (typeof draft!.slot.defaultValue !== "string" || !/^[+-]?\d+$/.test(draft!.slot.defaultValue))) {
                report(uiMessage("knowledge.defaultIntegerExactDecimal"));
                return;
            }
            authorProposal = true;
            draft!.parameters.push({ name: draft!.slot.name, kind: draft!.slot.kind, from: selection.from, to: selection.to, nullable: draft!.slot.nullable, ...(draft!.slot.defaultMode === "value" ? { default: draft!.slot.defaultValue } : draft!.slot.defaultMode === "none" ? { default: null } : {}) });
            draft!.basis = current;
            draft!.slot.name = "";
            render();
        }));
        if (draft.parameters.length)
            details.append(action(uiMessage("knowledge.clearParameterRangesReselect"), () => { draft!.parameters = []; draft!.basis = null; render(); }));
        const save = action(draft.templateId ? uiMessage("knowledge.prepareTemplateRegistrationUpdate") : uiMessage("knowledge.prepareTemplateRegistration"), async () => {
            const current = host.context();
            if (!current || current.dirty || !current.nodeId || !current.workspaceId) {
                report(uiMessage("knowledge.openSaveNativeNoteFirst"));
                return;
            }
            if (!draft!.title.trim()) {
                report(uiMessage("knowledge.templateNameRequired"));
                return;
            }
            if (draft!.parameters.length && !sameKnowledgeBasis(draft!.basis, current)) {
                report(uiMessage("knowledge.parameterSourceVersionChanged"));
                return;
            }
            await perform(uiMessage("knowledge.registerCurrentNativeNote"), { kind: "templateRegister", ...(draft!.templateId ? { templateId: draft!.templateId } : {}), title: draft!.title, description: draft!.description, parameters: draft!.parameters });
        }, "knowledge-button primary");
        save.disabled = context.dirty || !context.nodeId || !context.workspaceId;
        group.append(save);
        if (draft.templateId)
            group.append(action(uiMessage("knowledge.registerNewTemplateInstead"), () => { draft!.templateId = undefined; render(); }));
    }
    function renderTemplates(body: HTMLElement): void {
        const templates = snapshot?.templates ?? [];
        body.append(element("p", "knowledge-help", uiMessage("knowledge.templateCreationDescription")));
        renderTemplateRegistration(body);
        if (!templates.length) {
            body.append(element("p", "knowledge-empty", uiMessage("knowledge.noCreationTemplates")));
            return;
        }
        for (const template of templates) {
            const key = `${host.context()?.sessionId}:${template.id}`;
            let draft = templateDrafts.get(key);
            if (!draft) {
                draft = { name: "", parent: host.context()?.root ?? "", parameters: {}, presence: {} };
                for (const parameter of template.parameters ?? [])
                    draft.presence[parameter.name] = parameter.required && parameter.default === undefined ? "value" : "omitted";
                templateDrafts.set(key, draft);
            }
            const card = section(template.title, template.description);
            const name = input(draft.name, uiMessage("recognition.newNoteName")), parent = input(draft.parent);
            name.oninput = () => { draft!.name = name.value; };
            parent.oninput = () => { draft!.parent = parent.value; };
            card.append(field(uiMessage("recognition.newNoteName"), name), field(uiMessage("knowledge.targetParentDirectory"), parent));
            if (template.sourceNode)
                card.append(action(displayMessage("knowledge.sourcePath", { p0: uiDisplayParameter(nodeLabel(template.sourceNode)) }), () => navigate(template.sourceNode!), "knowledge-link-title"));
            if (template.available === false)
                card.append(element("p", "knowledge-warning", uiMessage("knowledge.templateSourceVersionUnavailable")));
            const context = host.context();
            if (template.sourceNode && context?.nodeId === template.sourceNode.nodeId && context.workspaceId === template.sourceNode.workspaceId)
                card.append(action(uiMessage("knowledge.updateCurrentTemplateRegistration"), () => {
                    const registration = registrationDrafts.get(context.sessionId);
                    if (!registration)
                        return;
                    authorProposal = true;
                    registration.templateId = template.id;
                    registration.title = template.title;
                    registration.description = template.description ?? "";
                    registration.parameters = [];
                    registration.basis = null;
                    registration.open = true;
                    registration.slotsOpen = true;
                    render();
                    container.querySelector<HTMLElement>(".knowledge-body")?.scrollTo({ top: 0 });
                }));
            const parameters = draft.parameters;
            const currentParameters = (): Record<string, KnowledgeTemplateValue> => {
                const values: Record<string, KnowledgeTemplateValue> = {};
                for (const parameter of template.parameters ?? []) {
                    const mode = draft!.presence[parameter.name] ?? "omitted";
                    if (mode === "value")
                        values[parameter.name] = parameter.kind === "bool" ? parameters[parameter.name] === true : parameters[parameter.name] ?? "";
                    else if (mode === "none")
                        values[parameter.name] = null;
                }
                return values;
            };
            for (const parameter of template.parameters ?? []) {
                const mode = draft.presence[parameter.name] ?? "omitted";
                const control = templateValueControl(parameter.kind ?? "markup", parameter.name in parameters ? parameters[parameter.name] : mode === "omitted" ? parameter.default : undefined, value => { parameters[parameter.name] = value; });
                control.required = Boolean(parameter.required);
                control.disabled = mode !== "value";
                const choices: [
                    string,
                    UiText
                ][] = [];
                if (!parameter.required || parameter.default !== undefined)
                    choices.push(["omitted", parameter.default !== undefined ? uiMessage("knowledge.useTemplateDefault") : uiMessage("knowledge.omitParameter")]);
                choices.push(["value", uiMessage("knowledge.provideText")]);
                if (parameter.nullable)
                    choices.push(["none", uiMessage("knowledge.useNone")]);
                if (choices.length > 1) {
                    const presence = select(choices);
                    presence.value = mode;
                    setUiAttr(presence, "aria-label", displayMessage("knowledge.parameterInputMode", { p0: uiDisplayParameter(parameter.title ?? parameter.name) }));
                    presence.onchange = () => {
                        draft!.presence[parameter.name] = presence.value as "omitted" | "value" | "none";
                        control.disabled = presence.value !== "value";
                        if (presence.value === "value" && !(parameter.name in parameters))
                            parameters[parameter.name] = parameter.kind === "bool" ? control.value === "true" : control.value;
                    };
                    card.append(field(parameter.title ?? parameter.name, presence));
                }
                card.append(field(choices.length > 1 ? uiMessage("knowledge.parameterValue") : parameter.title ?? parameter.name, control));
            }
            const preview = element("div", "knowledge-template-preview");
            if (template.source) {
                const source = element("pre", "knowledge-template-source", template.source);
                const details = element("details");
                details.append(element("summary", "", uiMessage("knowledge.nativeTemplateSource")), source);
                card.append(details);
            }
            const actions = element("div", "knowledge-actions");
            const inspect = action(uiMessage("knowledge.previewTemplate"), async () => {
                const context = host.context();
                if (!context || !host.previewTemplate)
                    return;
                const values = currentParameters();
                try {
                    const result = decodeKnowledgeTemplatePreview(await host.previewTemplate(template.id, values, context));
                    if (!sameKnowledgeBasis(context, host.context()) || JSON.stringify(values) !== JSON.stringify(currentParameters())) {
                        report(uiMessage("knowledge.templateParametersOrDraftChanged"));
                        return;
                    }
                    preview.replaceChildren();
                    if (result.tree)
                        preview.append(nativeTree(result.tree));
                    else if (result.source)
                        preview.append(element("pre", "knowledge-template-source", result.source));
                    for (const diagnostic of result.diagnostics ?? [])
                        preview.append(element("p", "knowledge-warning", engineDiagnosticUiText(diagnostic)));
                }
                catch (error) {
                    report(error);
                }
            });
            inspect.disabled = !host.previewTemplate || template.available === false;
            const create = action(uiMessage("knowledge.prepareCreateAndOpen"), () => perform(uiMessage("knowledge.createNoteFromNativeTemplate"), { kind: "templateCreate", templateId: template.id, parent: parent.value, name: name.value, parameters: currentParameters() }), "knowledge-button primary");
            create.disabled = template.available === false;
            actions.append(inspect, create);
            card.append(actions, preview);
            body.append(card);
        }
    }
    async function openGraphSource(edge: GraphEdge, node: GraphNode): Promise<void> {
        return closeGuard.run(() => openGraphSourceActive(edge, node));
    }
    async function openGraphSourceActive(edge: GraphEdge, node: GraphNode): Promise<void> {
        if (!sameKnowledgeBasis(snapshotBasis, host.context()))
            throw new UiError(uiMessage("knowledge.graphBasisChanged"));
        await host.navigate(node.reference);
        const current = host.context();
        if (!current || `${current.workspaceId}/${current.nodeId}` !== edge.authorSource || await nodeSourceHash(current.source) !== edge.sourceHash) {
            host.notify(uiMessage("knowledge.authorSourceOpenedBodyChanged"), true);
            return;
        }
        if (edge.range)
            host.selectSource?.(edge.range.utf16From, edge.range.utf16To);
        else
            host.notify(displayMessage("knowledge.authorSourceOpenedEditField", { p0: uiDisplayParameter(edge.fieldId ?? "") }));
    }
    function discardAuthorProposals(): void {
        closeGuard.assertReady(busy || pendingPlan !== null || pendingPreparation !== null || composing);
        closeGuard.assertMutable();
        annotationBody = "";
        suggestionReplacement = "";
        annotationKind = "comment";
        annotationTarget = "whole_document";
        annotationSelection = undefined;
        annotationSelectionBasis = null;
        replies.clear();
        citationInputs.clear();
        reconfirmations.clear();
        fileDrafts.clear();
        templateDrafts.clear();
        registrationDrafts.clear();
        mediaDraft = null;
        mediaController?.dispose();
        mediaController = null;
        mediaSession = null;
        authorProposal = false;
        render();
    }
    function assertCloseReady(): void {
        try {
            closeGuard.assertReady(authorProposal || busy || pendingPlan !== null || pendingPreparation !== null || composing);
            proofreading.assertCloseReady();
        } catch (error) { render(); throw error; }
    }
    function freezeForClose(): boolean {
        try { assertCloseReady(); } catch { return false; }
        if (!closeGuard.freeze(false)) return false;
        if (!proofreading.freezeForClose()) { closeGuard.thaw(); return false; }
        invalidateRead();
        return true;
    }
    function thawAfterClose(): void { proofreading.thawAfterClose(); closeGuard.thaw(); }
    function render(): void {
        if (disposed || closeGuard.frozen)
            return;
        deferredRender = false;
        const focused = document.activeElement instanceof HTMLElement && container.contains(document.activeElement) ? document.activeElement : null;
        const focusKey = sameKnowledgeBasis(renderedBasis, host.context()) ? focused?.dataset.knowledgeFocus : undefined;
        renderedBasis = host.context();
        const scroll = container.querySelector(".knowledge-body")?.scrollTop ?? 0;
        container.replaceChildren();
        const heading = element("div", "knowledge-heading");
        heading.append(element("strong", "", uiMessage("knowledge.title")), action(uiMessage("reminder.refresh"), refresh, "knowledge-refresh"));
        if (loading)
            heading.append(action(uiMessage("knowledge.stopWaiting"), () => { invalidateRead(); loading = false; clearOverlay(); setUiText(status, uiMessage("knowledge.scanWaitStopped")); render(); }));
        if (authorProposal) {
            heading.append(createAction(uiMessage("processingSettings.discardInput"), () => {
                try { discardAuthorProposals(); } catch (error) { report(error); }
            }));
        }
        container.append(heading);
        const tabs = element("div", "knowledge-tabs");
        tabs.setAttribute("role", "tablist");
        setUiAttr(tabs, "aria-label", uiMessage("knowledge.knowledgePanel"));
        for (const [id, label] of [["links", uiMessage("knowledge.relationships")], ["graph", uiMessage("knowledge.graph2d")], ["citations", uiMessage("knowledge.insertCitation")], ["mentions", uiMessage("knowledge.potentialLinks")], ["annotations", uiMessage("hostedShell.panelAnnotations")], ["proofreading", uiMessage("knowledge.proofreading")], ["files", uiMessage("knowledge.files")], ["templates", uiMessage("knowledge.templates")]] as const) {
            const control = action(label, () => { tab = id; render(); }, "knowledge-tab");
            control.setAttribute("role", "tab");
            control.setAttribute("aria-selected", String(tab === id));
            control.tabIndex = tab === id ? 0 : -1;
            control.onkeydown = event => {
                if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key))
                    return;
                event.preventDefault();
                const ids = ["links", "graph", "citations", "mentions", "annotations", "proofreading", "files", "templates"];
                const index = ids.indexOf(tab);
                tab = event.key === "Home" ? ids[0] : event.key === "End" ? ids.at(-1)! : ids[(index + (event.key === "ArrowRight" ? 1 : ids.length - 1)) % ids.length];
                render();
                container.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
            };
            tabs.append(control);
        }
        container.append(tabs, status);
        if (pendingPreparation) {
            const original = pendingPreparation;
            const pending = element("div", "knowledge-warning", displayMessage("knowledge.originalOperationUnconfirmed", { p0: uiDisplayParameter(original.operationId) }));
            pending.append(action(uiMessage("common.retry"), () => perform(original.title, original.intent, original)));
            if (host.inspectOperation) pending.append(action(uiMessage("knowledge.checkOriginalOperation"), inspectPreparation));
            if (original.prepared) pending.append(action(uiMessage("common.cancel"), () => {
                if (busy || pendingPreparation !== original) return;
                pendingPreparation = null;
                render();
            }));
            container.append(pending);
        }
        if (pendingPlan) {
            const pending = element("div", "knowledge-warning", displayMessage("knowledge.originalOperationUnconfirmed", { p0: uiDisplayParameter(pendingPlan.plan.operationId) }));
            if (host.inspectOperation)
                pending.append(action(uiMessage("knowledge.checkOriginalOperation"), inspectPending));
            container.append(pending);
        }
        const body = element("div", "knowledge-body");
        body.setAttribute("role", "tabpanel");
        container.append(body);
        if (!host.context()) {
            body.append(element("p", "knowledge-empty", uiMessage("knowledge.openNoteToViewKnowledge")));
            return;
        }
        body.append(element("p", "knowledge-help knowledge-current", host.context()!.path));
        for (const warning of snapshot?.warnings ?? [])
            body.append(element("p", "knowledge-warning", coreDescriptionUiText(warning)));
        for (const failure of snapshot?.errors ?? []) {
            const card = element("section", "knowledge-warning");
            if (failure.path !== null) card.append(element("pre", "knowledge-address", failure.path));
            card.append(element("p", "", apiErrorUiText(failure.error)));
            body.append(card);
        }
        if (tab === "links") {
            body.append(action(uiMessage("knowledge.openCurrentNoteLocalGraph"), () => { tab = "graph"; render(); }));
            renderLinks(body);
        }
        else if (tab === "graph")
            graph.render(body, sameKnowledgeBasis(snapshotBasis, host.context()) ? snapshot?.graph : undefined, `${host.context()?.workspaceId}/${host.context()?.nodeId}`, { open: node => closeGuard.run(() => Promise.resolve(host.navigate(node.reference))), source: openGraphSource, error: report });
        else if (tab === "citations") {
            const context = host.context()!;
            let state = citationInputs.get(context.sessionId);
            if (!state) {
                state = newNativeCitationInputs();
                citationInputs.set(context.sessionId, state);
            }
            renderNativeCitations(body, sameKnowledgeBasis(snapshotBasis, context) ? snapshot?.citations : undefined, state, { writable: host.canEditSource?.() !== false, at: () => {
                    const current = host.context();
                    if (!current || !sameKnowledgeBasis(snapshotBasis, current))
                        throw new UiError(uiMessage("knowledge.citationBasisChanged"));
                    return citationInsertionByte(current.source, current.selection?.from ?? current.source.length);
                }, perform: (title, intent) => perform(title, intent as KnowledgeIntent), select: range => host.selectSource?.(range.utf16From, range.utf16To), navigate, repaint: render });
        }
        else if (tab === "mentions")
            renderMentions(body);
        else if (tab === "annotations")
            renderAnnotations(body);
        else if (tab === "proofreading")
            proofreading.render(body);
        else if (tab === "files")
            renderFiles(body);
        else
            renderTemplates(body);
        body.scrollTop = scroll;
        if (focusKey)
            container.querySelector<HTMLElement>(`[data-knowledge-focus="${CSS.escape(focusKey)}"]`)?.focus();
    }
    async function refresh(strict = false): Promise<void> {
        if (closeGuard.frozen || disposed) return;
        // Keep the exact captured inputs even if a host later mutates its object.
        const context = structuredClone(host.context());
        if (context && pendingRead && gate.accepts(pendingRead.generation, pendingRead.context, context) && sameKnowledgeReadContext(pendingRead.context, context)) {
            await finishRead(pendingRead, strict);
            return;
        }
        invalidateRead();
        if ((context?.sessionId ?? "") !== graphSession) {
            graphSession = context?.sessionId ?? "";
            graph.clear();
        }
        if (!context) {
            snapshot = null;
            snapshotBasis = null;
            clearOverlay();
            render();
            if (strict) throw new UiError(uiMessage("knowledge.readBasisChanged"));
            return;
        }
        if (!sameKnowledgeBasis(snapshotBasis, context)) {
            rules.closePreviews();
            snapshot = null;
            snapshotBasis = null;
            clearOverlay();
            if (sameKnowledgeBasis(renderedBasis, context) && container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))
                deferredRender = true;
            else
                render();
        }
        setUiText(status, uiMessage("knowledge.readingAuthorizedKnowledge"));
        loading = true;
        if (!(container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select")))
            render();
        const generation = gate.begin();
        const read = { context, generation, completion: readSnapshot(context, generation) };
        pendingRead = read;
        await finishRead(read, strict);
    }
    /** Every caller waits independently, including strict post-commit readback.
     * The shared completion installs/reports once and never swallows its error. */
    async function finishRead(read: PendingRead, strict: boolean): Promise<void> {
        try {
            const installed = await read.completion;
            if (strict && !installed) throw new UiError(uiMessage("knowledge.readBasisChanged"));
        }
        catch (error) { if (strict) throw error; }
        finally { if (pendingRead === read) pendingRead = null; }
    }
    async function readSnapshot(context: KnowledgeContext, generation: number): Promise<boolean> {
        try {
            const next = await host.load(context);
            // Sharing compares all inputs; delivery retains the source/owner gate.
            // Core knowledge reads do not consume caret/selection or Draft ACKs.
            if (closeGuard.frozen || disposed || !gate.accepts(generation, context, host.context()))
                return false;
            snapshot = decodeKnowledgeSnapshot(next);
            snapshotBasis = context;
            loading = false;
            setUiText(status, "");
            if (container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))
                deferredRender = true;
            else
                render();
            drawOverlay();
            return true;
        }
        catch (error) {
            if (!closeGuard.frozen && !disposed && gate.accepts(generation, context, host.context())) {
                loading = false;
                snapshotBasis = null;
                clearOverlay();
                report(error);
                setUiText(status, displayMessage("knowledge.scanFailedLastResultStale", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse")) }));
                if (container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))
                    deferredRender = true;
                else
                    render();
            }
            throw error;
        }
    }
    /** Library sidecars, settings and other notes can change with equal Draft bytes. */
    function environmentChanged(root: string): void {
        if (closeGuard.frozen || disposed) return;
        const coordinate = (value: string): string => value.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
        if (![host.context()?.root, pendingRead?.context.root, snapshotBasis?.root].some(value => value !== undefined && coordinate(value) === coordinate(root))) return;
        invalidateRead();
        loading = false;
        snapshot = null;
        snapshotBasis = null;
        rules.closePreviews();
        clearOverlay();
        setUiText(status, "");
        if (container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))
            deferredRender = true;
        else
            render();
    }
    render();
    return {
        assertCloseReady, freezeForClose, thawAfterClose,
        backgroundEnabled: () => enabled || problemHints,
        environmentChanged,
        refresh,
        selectTab(next) { closeGuard.assertMutable(); tab = next; render(); },
        attachSurface(next, nextMappings = []) {
            closeGuard.assertMutable();
            clearOverlay();
            surface = next;
            mappings = nextMappings;
            if (surface)
                surface.classList.add("knowledge-surface");
            drawOverlay();
        },
        dispose() { assertCloseReady(); disposed = true; invalidateRead(); graph.clear(); rules.closePreviews(); clearOverlay(); mediaController?.dispose(); snapshot = null; snapshotBasis = null; window.removeEventListener("resize", resize); window.removeEventListener("keydown", escape); window.removeEventListener("compositionstart", compositionStart); window.removeEventListener("compositionend", compositionEnd); container.removeEventListener("focusout", onBlur); container.removeEventListener("input", authoredInput, true); container.removeEventListener("change", authoredInput, true); disposeUiTree(container); container.replaceChildren(); },
    };
}
