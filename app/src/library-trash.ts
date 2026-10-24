import { ProposalCloseGuard } from "./proposal-close-guard";
import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, disposeUiTree, UiError, type UiText } from "./ui-locale";
import { decodeKnowledgeDelivery, decodeKnowledgeLoss, knowledgeDeliveryErrors, knowledgeLossMessage } from "./knowledge-state";
import { committedDeliveryComplete, deliverCommitted } from "./original-operation";
import type { KnowledgePlan, KnowledgeReceipt } from "./knowledge-panel";
import type { OpenedDocument } from "./types";
export interface LibraryTrashEntry {
    operationId: string;
    nodeId?: string;
    name: string;
    path?: string;
}
export interface LibraryTrashDraft {
    name: string;
    parent: string;
}
/** A single retired transaction may contain multiple independently restorable roots. */
export function trashRestoreDraft(drafts: Map<string, LibraryTrashDraft>, root: string, entry: LibraryTrashEntry): LibraryTrashDraft {
    const key = JSON.stringify([root, entry.operationId, entry.nodeId ?? null]);
    let draft = drafts.get(key);
    if (!draft) {
        draft = { name: entry.name, parent: root };
        drafts.set(key, draft);
    }
    return draft;
}
export interface LibraryTrashHost {
    request<T>(path: string, body?: unknown): Promise<T>;
    created(document: OpenedDocument): Promise<void>;
    notify(message: UiText, error?: boolean): void;
    environmentChanged?(root: string): void;
}
export interface LibraryTrashController {
    open(root: string): Promise<void>;
    assertCloseReady(): void;
    freezeForClose(): boolean;
    thawAfterClose(): void;
}
export interface RestoreRequest {
    operationId: string;
    root: string;
    intent: {
        kind: "restoreNode";
        trashOperationId: string;
        nodeId?: string;
        parent: string;
        name: string;
    };
}
export interface PendingRestore {
    request: RestoreRequest;
    plan?: KnowledgePlan;
    confirmed: boolean;
    state: "preparing" | "prepared" | "committing" | "unknown" | "committed" | "failed";
    receipt?: KnowledgeReceipt;
    delivered: boolean;
    message: UiText;
}
const terminalErrorCodes = new Set(["conflict", "operationConflict", "preimageChanged", "invalidName", "invalidPath", "trashMissing", "trashSelectionRequired", "trashSelectionMismatch", "targetExists", "destinationExists", "unsupported", "permissionDenied", "rejected"]);
function errorMessage(error: unknown): UiText { return error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"); }
function errorCode(error: unknown): string { return error && typeof error === "object" && "code" in error && typeof error.code === "string" ? error.code : ""; }
function openedDocument(value: unknown, nodeId?: string): value is OpenedDocument {
    if (!value || typeof value !== "object")
        return false;
    const document = value as Partial<OpenedDocument>;
    return typeof document.sessionId === "string" && typeof document.path === "string" && typeof document.root === "string" && typeof document.source === "string" && typeof document.baseId === "string" && (nodeId === undefined || document.nodeId === nodeId);
}
/** Retains the original request across UI lifetimes; status reads never execute it. */
export class LibraryTrashRestoreFlow {
    private readonly closeGuard = new ProposalCloseGuard();
    assertCloseReady(): void { this.closeGuard.assertReady(this.busy || this.blocksNewRestore); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.busy || this.blocksNewRestore); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    bindCloseGuard(element: HTMLElement): void { this.closeGuard.bind(element); }
    runCloseTracked<T>(operation: () => Promise<T>): Promise<T> { return this.closeGuard.run(operation); }

    pending: PendingRestore | null = null;
    busy = false;
    constructor(private readonly host: LibraryTrashHost, private readonly newId: () => string = () => crypto.randomUUID()) { }
    get blocksNewRestore(): boolean { return Boolean(this.pending && !this.pending.delivered && this.pending.state !== "failed"); }
    async prepare(root: string, entry: LibraryTrashEntry, parent: string, name: string): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.busy || this.blocksNewRestore)
                throw new UiError(uiMessage("libraryTrash.resolveOriginalBeforeNew"));
            if (!parent.trim() || !name.trim())
                throw new UiError(uiMessage("libraryTrash.locationAndNameRequired"));
            const request: RestoreRequest = Object.freeze({ operationId: this.newId(), root, intent: Object.freeze({ kind: "restoreNode" as const, trashOperationId: entry.operationId, ...(entry.nodeId !== undefined ? { nodeId: entry.nodeId } : {}), parent, name }) });
            this.pending = { request, confirmed: false, state: "preparing", delivered: false, message: uiMessage("libraryTrash.preparingExactChanges") };
            this.busy = true;
            try {
                const plan = await this.host.request<KnowledgePlan>("/api/operations/prepare", request);
                if (plan.operationId !== request.operationId || typeof plan.planId !== "string" || !plan.planId)
                    throw new UiError(uiMessage("libraryTrash.prepareRequestMismatch"));
                if (plan.losses !== undefined && !Array.isArray(plan.losses)) throw new UiError(uiMessage("error.invalidResponse"));
                this.pending.plan = { ...plan, ...(plan.losses ? { losses: plan.losses.map(decodeKnowledgeLoss) } : {}) };
                this.pending.state = "prepared";
                this.pending.message = uiMessage("libraryTrash.checkLocationAndAllChanges");
            }
            catch (error) {
                this.pending.state = terminalErrorCodes.has(errorCode(error)) ? "failed" : "unknown";
                this.pending.message = joinUiText([errorMessage(error), this.pending.state === "unknown" ? uiMessage("libraryTrash.originalRequestRetainedSuffix") : ""]);
                throw error;
            }
            finally {
                this.busy = false;
            }
        });
    }
    cancelPrepared(): void {
        if (this.closeGuard.frozen) throw new UiError(uiMessage("shell.closeAuthorInputPending"));
        if (!this.busy && this.pending?.state === "prepared" && !this.pending.confirmed)
            this.pending = null;
    }
    async confirmAndCommit(): Promise<void> {
        return this.closeGuard.run(async () => {
            const pending = this.pending;
            if (this.busy || !pending?.plan || pending.state !== "prepared")
                throw new UiError(uiMessage("libraryTrash.noConfirmablePreparedRestore"));
            pending.confirmed = true;
            pending.state = "committing";
            pending.message = uiMessage("libraryTrash.executingConfirmedRestore");
            this.busy = true;
            try {
                const receipt = await this.host.request<KnowledgeReceipt>("/api/operations/commit", { operationId: pending.request.operationId, planId: pending.plan.planId });
                await this.receive(receipt);
            }
            catch (error) {
                this.recordCommitError(error);
                throw error;
            }
            finally {
                this.busy = false;
            }
        });
    }
    async reconcile(): Promise<void> {
        return this.closeGuard.run(async () => {
            const pending = this.pending;
            if (!pending || this.busy)
                return;
            this.busy = true;
            pending.message = uiMessage("libraryTrash.checkingOriginalResult");
            try {
                const result = decodeKnowledgeDelivery(await this.host.request<KnowledgeReceipt & Partial<KnowledgePlan>>("/api/operations/status", { operationId: pending.request.operationId }));
                if (result.operationId !== pending.request.operationId)
                    throw new UiError(uiMessage("libraryTrash.statusOtherOperation"));
                if (pending.state === "committed" && result.status !== "committed")
                    throw new UiError(uiMessage("libraryTrash.committedStatusCannotRegress"));
                if (result.status === "prepared" && typeof result.planId === "string") {
                    if (!result.planId || result.losses !== undefined && !Array.isArray(result.losses)) throw new UiError(uiMessage("error.invalidResponse"));
                    if (pending.plan && pending.plan.planId !== result.planId)
                        throw new UiError(uiMessage("libraryTrash.originalPrepareIdChanged"));
                    pending.plan = { ...result, planId: result.planId, operationId: pending.request.operationId, ...(result.losses ? { losses: result.losses.map(decodeKnowledgeLoss) } : {}) };
                    pending.state = "prepared";
                    pending.message = pending.confirmed ? uiMessage("libraryTrash.originalStillPrepared") : uiMessage("libraryTrash.originalPreparationFound");
                }
                else
                    await this.receive(result);
            }
            catch (error) {
                if (pending.state !== "committed")
                    pending.state = "unknown";
                pending.message = displayMessage("libraryTrash.failureOriginalRecordRetained", { p0: uiDisplayParameter(errorMessage(error)) });
                throw error;
            }
            finally {
                this.busy = false;
            }
        });
    }
    async deliverCommitted(): Promise<void> {
        return this.closeGuard.run(async () => {
            const pending = this.pending;
            if (!pending || pending.state !== "committed" || pending.delivered)
                return;
            const document = pending.receipt?.document;
            if (!pending.receipt || !committedDeliveryComplete(pending.receipt) || !openedDocument(document, pending.request.intent.nodeId)) {
                // Explicit delivery retry rechecks the original fact before reusing a
                // previously failed callback; a commit/status read never recurses.
                if (!this.busy) { await this.reconcile(); return; }
                pending.message = joinUiText([uiMessage("libraryTrash.restoredOpenInformationUnavailable"), ...(pending.receipt ? knowledgeDeliveryErrors(pending.receipt) : [])], "\n");
                return;
            }
            try {
                pending.receipt = await deliverCommitted(pending.receipt, async () => { await this.host.created(document); });
                if (!committedDeliveryComplete(pending.receipt))
                    throw new UiError(joinUiText([uiMessage("libraryTrash.restoredOpenInformationUnavailable"), ...knowledgeDeliveryErrors(pending.receipt)], "\n"));
                pending.delivered = true;
                pending.message = uiMessage("libraryTrash.restoredOpened");
                this.host.notify(pending.message);
            }
            catch (error) {
                pending.message = displayMessage("libraryTrash.restoredOpenIncomplete", { p0: uiDisplayParameter(errorMessage(error)) });
                throw error;
            }
        });
    }
    private async receive(value: KnowledgeReceipt): Promise<void> {
        const pending = this.pending;
        if (!pending)
            return;
        const receipt = decodeKnowledgeDelivery(value);
        if (receipt.operationId !== pending.request.operationId)
            throw new UiError(uiMessage("libraryTrash.resultRequestMismatch"));
        if (pending.plan && typeof receipt.planId === "string" && receipt.planId !== pending.plan.planId)
            throw new UiError(uiMessage("libraryTrash.resultPreparedMismatch"));
        if (!["committed", "resolved", "recoveryPending", "unknown"].includes(receipt.status) || pending.state === "committed" && receipt.status !== "committed")
            throw new UiError(uiMessage("libraryTrash.statusContractMismatch"));
        pending.receipt = receipt;
        if (receipt.status === "committed") {
            const changed = pending.state !== "committed";
            pending.state = "committed";
            pending.message = joinUiText([uiMessage("libraryTrash.restoreComplete"), ...knowledgeDeliveryErrors(receipt)], "\n");
            if (changed)
                this.host.environmentChanged?.(pending.request.root);
            await this.deliverCommitted();
        }
        else if (receipt.status === "resolved") {
            pending.state = "failed";
            pending.message = uiMessage("libraryTrash.originalEnded");
        }
        else {
            pending.state = "unknown";
            pending.message = uiMessage("libraryTrash.restoreUnconfirmedNoRepeat");
        }
    }
    private recordCommitError(error: unknown): void {
        const pending = this.pending;
        if (!pending || pending.state === "committed")
            return;
        pending.state = terminalErrorCodes.has(errorCode(error)) ? "failed" : "unknown";
        pending.message = joinUiText([errorMessage(error), pending.state === "unknown" ? uiMessage("libraryTrash.resultUnknownSuffix") : ""]);
    }
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] { const result = document.createElement(tag); result.className = className; setUiText(result, text); return result; }
function field(title: UiText, value: string): {
    label: HTMLLabelElement;
    input: HTMLInputElement;
} { const label = el("label", "knowledge-field"); label.append(el("span", "", title)); const input = el("input", "knowledge-input"); input.value = value; label.append(input); return { label, input }; }
/** Library entry point remains usable when there is no active note. */
export function createLibraryTrashController(host: LibraryTrashHost): LibraryTrashController {
    const flow = new LibraryTrashRestoreFlow(host);
    const drafts = new Map<string, LibraryTrashDraft>();
    const closeGuard = new ProposalCloseGuard();
    let draftEdited = false;
    const assertCloseReady = (): void => { closeGuard.assertReady(draftEdited); flow.assertCloseReady(); };
    let active: {
        dialog: HTMLDialogElement;
        root: string;
        promise: Promise<void>;
        close(): void;
    } | null = null;
    return {
        assertCloseReady,
        freezeForClose: () => {
            if (!closeGuard.freeze(draftEdited)) return false;
            if (!flow.freezeForClose()) { closeGuard.thaw(); return false; }
            return true;
        },
        thawAfterClose: () => { closeGuard.thaw(); flow.thawAfterClose(); },
        async open(root: string): Promise<void> {
            if (closeGuard.frozen) throw new UiError(uiMessage("shell.closeAuthorInputPending"));
            if (active) {
                active.dialog.focus();
                return active.promise;
            }
            const dialog = el("dialog", "knowledge-dialog library-trash-dialog");
            closeGuard.bind(dialog);
            flow.bindCloseGuard(dialog);
            setUiAttr(dialog, "aria-label", uiMessage("libraryTrash.title"));
            dialog.append(el("h2", "", uiMessage("libraryTrash.title")), el("p", "knowledge-help", uiMessage("libraryTrash.description")), el("p", "knowledge-address", root));
            const status = el("p", "knowledge-help");
            status.setAttribute("aria-live", "polite");
            const pendingHost = el("section", "knowledge-section"), list = el("section", "knowledge-section"), actions = el("div", "knowledge-actions");
            dialog.append(status, pendingHost, list, actions);
            let entries: LibraryTrashEntry[] = [], closed = false, loaded = false;
            let resolve!: () => void;
            const promise = new Promise<void>(r => { resolve = r; });
            const close = (): void => {
                if (closed)
                    return;
                try {
                    closeGuard.assertReady(false);
                    flow.assertCloseReady();
                }
                catch (error) { setUiText(status, errorMessage(error)); return; }
                drafts.clear();
                draftEdited = false;
                closed = true;
                dialog.close();
                disposeUiTree(dialog);
                dialog.remove();
                if (active?.dialog === dialog)
                    active = null;
                resolve();
            };
            active = { dialog, root, promise, close };
            const button = (label: UiText, run: () => void | Promise<void>, primary = false): HTMLButtonElement => {
                const value = el("button", `knowledge-button${primary ? " primary" : ""}`, label);
                value.type = "button";
                value.onclick = () => {
                    void Promise.resolve(run()).catch(error => { setUiText(status, errorMessage(error)); host.notify(errorMessage(error), true); }).finally(() => {
                        if (!closed)
                            render();
                    });
                };
                return value;
            };
            const lock = (): void => {
                dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input,button").forEach(control => {
                    if (control.dataset.close !== "true")
                        control.disabled = true;
                });
            };
            const load = async (): Promise<void> => {
                return closeGuard.run(async () => {
                    setUiText(status, uiMessage("libraryTrash.readingRecords"));
                    const result = await host.request<{
                        trash: LibraryTrashEntry[];
                    }>("/api/knowledge/trash", { root });
                    if (closed)
                        return;
                    entries = result.trash;
                    loaded = true;
                    setUiText(status, flow.pending?.delivered ? flow.pending.message : "");
                    render();
                });
            };
            const renderPreview = (pending: PendingRestore): void => {
                pendingHost.append(el("h3", "", pending.confirmed ? uiMessage("libraryTrash.confirmedOriginalRestore") : uiMessage("libraryTrash.restorePreview")), el("p", "knowledge-help", displayMessage("libraryTrash.restoreNameSummary", { p0: uiDisplayParameter(pending.request.intent.name) })), el("p", "knowledge-address", displayMessage("libraryTrash.targetParentSummary", { p0: uiDisplayParameter(pending.request.intent.parent) })));
                if (pending.request.root !== root)
                    pendingHost.append(el("p", "knowledge-warning", displayMessage("libraryTrash.pendingOtherLibrary", { p0: uiDisplayParameter(pending.request.root) })));
                const changes = pending.plan?.changes ?? [];
                if (changes.length) {
                    pendingHost.append(el("p", "knowledge-help", displayMessage("libraryTrash.fileChangeCount", { p0: changes.length })));
                    const paths = el("ul", "knowledge-change-list");
                    for (const change of changes) {
                        const item = el("li");
                        item.append(el("strong", "", ({ create: uiMessage("libraryTrash.changeRestoreTo"), copy: uiMessage("libraryTrash.changeRestoreCopyTo"), move: uiMessage("libraryTrash.changeMoveTo"), rename: uiMessage("libraryTrash.changeRename"), relocateFrom: uiMessage("libraryTrash.changeMoveOutRecycle"), relocateTo: uiMessage("libraryTrash.changeRestoreTo"), remove: uiMessage("libraryTrash.changeMoveOutRecycle"), delete: uiMessage("libraryTrash.changeMoveOutRecycle"), write: uiMessage("libraryTrash.changeRestoreContentTo") } as Record<string, UiText>)[change.action ?? ""] ?? uiMessage("libraryTrash.changeFallback")), el("span", "knowledge-address", change.path ?? ""));
                        if (change.bytes !== undefined)
                            item.append(el("small", "knowledge-help", displayMessage("knowledge.byteCount", { p0: uiDisplayParameter(change.bytes.toLocaleString()) })));
                        paths.append(item);
                    }
                    pendingHost.append(paths);
                }
                for (const loss of pending.plan?.losses ?? [])
                    pendingHost.append(el("p", "knowledge-warning", knowledgeLossMessage(loss)));
                pendingHost.append(el("p", pending.state === "unknown" || pending.state === "failed" ? "knowledge-warning" : "knowledge-help", pending.message));
            };
            const render = (): void => {
                if (closed)
                    return;
                pendingHost.replaceChildren();
                list.replaceChildren();
                const pending = flow.pending;
                if (pending && !pending.delivered) {
                    renderPreview(pending);
                    const controls = el("div", "knowledge-actions");
                    pendingHost.append(controls);
                    if (pending.state === "prepared") {
                        if (!pending.confirmed)
                            controls.append(button(uiMessage("libraryTrash.cancelPreparation"), () => { flow.cancelPrepared(); render(); }));
                        controls.append(button(pending.confirmed ? uiMessage("libraryTrash.executeConfirmedOriginal") : uiMessage("libraryTrash.confirmRestore"), async () => {
                            lock();
                            await flow.confirmAndCommit();
                            if (!closed && flow.pending?.state === "committed")
                                await load();
                        }, true));
                    }
                    else if (pending.state === "committed") {
                        const canDeliver = pending.receipt !== undefined && committedDeliveryComplete(pending.receipt) && openedDocument(pending.receipt.document, pending.request.intent.nodeId);
                        controls.append(button(canDeliver ? uiMessage("libraryTrash.openRestoredNote") : uiMessage("knowledge.checkOriginalOperation"), async () => {
                            lock();
                            if (flow.pending?.receipt && committedDeliveryComplete(flow.pending.receipt) && openedDocument(flow.pending.receipt.document, flow.pending.request.intent.nodeId))
                                await flow.deliverCommitted();
                            else
                                await flow.reconcile();
                        }));
                    }
                    else if (pending.state === "unknown")
                        controls.append(button(uiMessage("knowledge.checkOriginalOperation"), async () => {
                            lock();
                            await flow.reconcile();
                            if (!closed && flow.pending?.state === "committed")
                                await load();
                        }));
                }
                if (!entries.length)
                    list.append(el("p", "knowledge-empty", loaded ? uiMessage("libraryTrash.noReadableRecords") : uiMessage("libraryTrash.recordsReadIncomplete")));
                for (const entry of entries) {
                    const draft = trashRestoreDraft(drafts, root, entry);
                    const card = el("article", "knowledge-card");
                    card.append(el("strong", "", entry.name));
                    if (entry.path)
                        card.append(el("p", "knowledge-address", entry.path));
                    const name = field(uiMessage("knowledge.restoreName"), draft.name), parent = field(uiMessage("knowledge.targetParentFullPath"), draft.parent);
                    name.input.oninput = () => { draftEdited = true; draft!.name = name.input.value; };
                    parent.input.oninput = () => { draftEdited = true; draft!.parent = parent.input.value; };
                    const prepare = button(uiMessage("knowledge.prepareRestore"), async () => { lock(); await flow.prepare(root, entry, parent.input.value, name.input.value); });
                    prepare.disabled = flow.busy || flow.blocksNewRestore;
                    name.input.disabled = parent.input.disabled = flow.busy;
                    card.append(name.label, parent.label, prepare);
                    list.append(card);
                }
                if (flow.busy)
                    lock();
            };
            const refresh = button(uiMessage("libraryTrash.refreshRecords"), load), done = button(uiMessage("common.close"), close);
            done.dataset.close = "true";
            actions.append(refresh, done);
            dialog.addEventListener("cancel", event => { event.preventDefault(); close(); });
            document.body.append(dialog);
            dialog.showModal();
            setUiText(status, uiMessage("libraryTrash.readingRecords"));
            try {
                if (flow.pending && !flow.pending.delivered && flow.pending.state !== "failed")
                    await flow.reconcile();
                await load();
            }
            catch (error) {
                if (!closed) {
                    setUiText(status, errorMessage(error));
                    host.notify(errorMessage(error), true);
                    render();
                }
            }
            return promise;
        },
    };
}
