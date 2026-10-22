import { disposeUiTree, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { decodeProofreadingOptions, type ProofreadRule } from "./proofreading-panel";
import { defaultProcessingSettings, parseProcessingLanguages, processingLanguageTag, type ProcessingRuleScope, type ProcessingSettings } from "./processing-settings";
import { apiErrorUiText } from "./api-error";
import { exactKeys } from "./ui-diagnostics";
import { decodeProcessingOperation, decodeProcessingRead, type ProcessingOperation, type ProcessingRead } from "./style-wire";
export type { ProcessingOperation, ProcessingRead } from "./style-wire";
export interface ProcessingDocument {
    schemaVersion: 1;
    defaults: ProcessingSettings;
    rules: ProofreadRule[];
    scopedAllowlist: {
        text: string;
        scope: ProcessingRuleScope;
    }[];
}
export interface ProcessingRequest {
    operationId: string;
    baseId: string;
    beforeSource: string | null;
    processing: ProcessingDocument | null;
}
export interface ProcessingSettingsHost {
    persistenceKey: string;
    read(): Promise<unknown>;
    prepare(request: ProcessingRequest): Promise<unknown>;
    status(request: ProcessingRequest): Promise<unknown>;
    settle(request: ProcessingRequest): Promise<unknown>;
    apply(request: {
        request: ProcessingRequest;
        planId: string;
    }): Promise<unknown>;
    committed?(): void;
    onApplied?(): void | Promise<void>;
}
export const defaultProcessingDocument = (): ProcessingDocument => ({ schemaVersion: 1, defaults: defaultProcessingSettings(), rules: [], scopedAllowlist: [] });
export function decodeProcessingDocument(value: unknown): ProcessingDocument {
    const v = value as ProcessingDocument;
    if (!v || v.schemaVersion !== 1 || Object.keys(v).some(k => !["schemaVersion", "defaults", "rules", "scopedAllowlist"].includes(k)))
        throw new UiError(uiMessage("processingSettings.documentUnsupported"));
    const validated = decodeProofreadingOptions({ characters: true, typos: true, pairs: true, sensitive: true, rules: v.rules ?? [], allowlist: [], scopedAllowlist: v.scopedAllowlist ?? [], processing: v.defaults });
    return { schemaVersion: 1, defaults: validated.processing!, rules: validated.rules, scopedAllowlist: validated.scopedAllowlist! };
}
interface Retained {
    state: ProcessingRead;
    source: string;
    pending?: {
        request: ProcessingRequest;
        operation?: ProcessingOperation;
    };
}
/** Saves the frozen request before network IO. Reopening queries its original ID. */
export class ProcessingSettingsController {
    state: ProcessingRead | null = null;
    source = "";
    pending: Retained["pending"];
    busy = false;
    recoveryError: string | null = null;
    constructor(readonly host: ProcessingSettingsHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
        const raw = storage.getItem(host.persistenceKey);
        if (!raw)
            return;
        try {
            const saved = JSON.parse(raw) as Retained;
            if (!saved || typeof saved !== "object" || Array.isArray(saved) || !exactKeys(saved as unknown as Record<string, unknown>, ["state", "source"], ["pending"]) || !saved.state?.baseId || typeof saved.source !== "string" || !(saved.state.beforeSource === null || typeof saved.state.beforeSource === "string"))
                throw new UiError(uiMessage("processingSettings.recoveryShapeInvalid"));
            if (Object.hasOwn(saved, "pending")) {
                const pending = saved.pending, request = pending?.request;
                if (!pending || typeof pending !== "object" || Array.isArray(pending) || !exactKeys(pending as unknown as Record<string, unknown>, ["request"], ["operation"]) || !request || typeof request !== "object" || Array.isArray(request) || !exactKeys(request as unknown as Record<string, unknown>, ["operationId", "baseId", "beforeSource", "processing"]) || typeof request.operationId !== "string" || !request.operationId || request.baseId !== saved.state.baseId || request.beforeSource !== saved.state.beforeSource)
                    throw new UiError(uiMessage("processingSettings.recoveryShapeInvalid"));
                if (request.processing !== null)
                    decodeProcessingDocument(request.processing);
            }
            this.state = decodeProcessingRead(saved.state, decodeProcessingDocument);
            this.source = saved.source;
            this.pending = saved.pending;
            if (this.pending && Object.hasOwn(this.pending, "operation")) {
                const operation = decodeProcessingOperation(this.pending.operation);
                if (operation.operationId !== this.pending.request.operationId || operation.preview && operation.preview.beforeSource !== this.pending.request.beforeSource) throw new UiError(uiMessage("processingSettings.receiptMismatch"));
                this.pending.operation = operation;
            }
        }
        catch {
            this.state = null;
            this.source = "";
            this.pending = undefined;
            this.recoveryError = raw;
        }
    }
    get locked(): boolean { return this.busy || !!this.pending || this.recoveryError !== null; }
    get dirty(): boolean { return !!this.state && this.source !== JSON.stringify(this.state.processing ?? defaultProcessingDocument(), null, 2); }
    async load(): Promise<void> {
        if (this.locked || this.dirty)
            throw new UiError(uiMessage("processingSettings.pendingOrDraftRequired"));
        this.install(await this.host.read());
    }
    edit(source: string): void {
        if (this.locked)
            throw new UiError(uiMessage("processingSettings.operationPending"));
        this.source = source;
        this.retain();
    }
    resetDraft(): void {
        if (this.locked || !this.state)
            throw new UiError(uiMessage("processingSettings.confirmOriginalRequired"));
        this.source = JSON.stringify(this.state.processing ?? defaultProcessingDocument(), null, 2);
        this.retain();
    }
    async prepare(reset = false): Promise<void> {
        if (!this.state || this.locked)
            throw new UiError(uiMessage("processingSettings.readOriginalRequired"));
        const processing = reset ? null : decodeProcessingDocument(JSON.parse(this.source));
        this.pending = { request: { operationId: crypto.randomUUID(), baseId: this.state.baseId, beforeSource: this.state.beforeSource, processing } };
        try {
            this.retain();
        }
        catch (error) {
            this.pending = undefined;
            throw error;
        }
        this.busy = true;
        try {
            this.accept(await this.host.prepare(structuredClone(this.pending.request)));
        }
        finally {
            this.busy = false;
        }
    }
    async status(): Promise<void> {
        if (!this.pending || this.busy)
            throw new UiError(uiMessage("processingSettings.noPendingOperation"));
        this.busy = true;
        try {
            this.accept(await this.host.status(structuredClone(this.pending.request)));
            await this.finishTerminal();
        }
        finally {
            this.busy = false;
        }
    }
    async retryPrepare(): Promise<void> {
        if (!this.pending || this.busy)
            throw new UiError(uiMessage("processingSettings.noPrepareToRetry"));
        this.busy = true;
        try {
            this.accept(await this.host.prepare(structuredClone(this.pending.request)));
            await this.finishTerminal();
        }
        finally {
            this.busy = false;
        }
    }
    async settle(): Promise<void> {
        if (!this.pending || this.busy)
            throw new UiError(uiMessage("processingSettings.noOperationToSettle"));
        this.busy = true;
        try {
            this.accept(await this.host.settle(structuredClone(this.pending.request)));
            await this.finishTerminal();
        }
        finally {
            this.busy = false;
        }
    }
    async apply(): Promise<void> {
        if (!this.pending?.operation?.planId || this.pending.operation.status !== "prepared" || this.busy)
            throw new UiError(uiMessage("processingSettings.previewOriginalRequired"));
        this.busy = true;
        try {
            this.accept(await this.host.apply({ request: structuredClone(this.pending.request), planId: this.pending.operation.planId }));
            if ((this.pending?.operation?.status as string | undefined) === "committed") {
                const fresh = await this.host.read();
                this.install(fresh);
                await this.host.onApplied?.();
            }
        }
        finally {
            this.busy = false;
        }
    }
    private accept(value: unknown): void {
        const operation = decodeProcessingOperation(value);
        if (!this.pending || operation.operationId !== this.pending.request.operationId || !["prepared", "committed", "recoveryPending", "notRegistered", "notCommitted", "cancelled", "resolved"].includes(operation.status) || ["prepared", "committed", "recoveryPending"].includes(operation.status) && !operation.planId || this.pending.operation?.planId && operation.planId !== this.pending.operation.planId || this.pending.operation?.status === "committed" && operation.status !== "committed")
            throw new UiError(uiMessage("processingSettings.receiptMismatch"));
        if (operation.preview && operation.preview.beforeSource !== this.pending.request.beforeSource)
            throw new UiError(uiMessage("processingSettings.receiptMismatch"));
        const committed = operation.status === "committed" && this.pending.operation?.status !== "committed";
        this.pending.operation = operation;
        try {
            this.retain();
        }
        finally {
            if (committed)
                this.host.committed?.();
        }
    }
    private async finishTerminal(): Promise<void> {
        const status = this.pending?.operation?.status;
        if (status === "committed") {
            this.install(await this.host.read());
            await this.host.onApplied?.();
        }
        else if (status === "cancelled" || status === "resolved") {
            const draft = this.source;
            this.install(await this.host.read());
            this.source = draft;
            this.retain();
        }
    }
    private install(value: unknown): void { const state = decodeProcessingRead(value, decodeProcessingDocument); this.state = state; this.pending = undefined; this.source = JSON.stringify(state.processing ?? defaultProcessingDocument(), null, 2); this.retain(); }
    private retain(): void { this.storage.setItem(this.host.persistenceKey, JSON.stringify({ state: this.state, source: this.source, pending: this.pending })); }
}
const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); setUiText(e, text); return e; };
export async function openProcessingSettingsPanel(host: ProcessingSettingsHost): Promise<void> {
    const controller = new ProcessingSettingsController(host);
    if (!controller.pending && !controller.dirty && !controller.recoveryError)
        await controller.load();
    const dialog = el("dialog");
    dialog.className = "modal library-style-panel";
    const title = el("h2", uiMessage("processingSettings.title"));
    setUiAttr(dialog, "aria-label", uiMessage("processingSettings.title"));
    const source = el("textarea");
    source.className = "text-input style-source";
    source.rows = 14;
    setUiAttr(source, "aria-label", uiMessage("processingSettings.sourceLabel"));
    const languages = el("input");
    languages.className = "text-input";
    setUiAttr(languages, "aria-label", uiMessage("processingSettings.languagesLabel"));
    const message = el("p");
    message.setAttribute("role", "status");
    const preview = el("pre");
    const warning = el("p"), operationSummary = el("p");
    const actions = el("div");
    actions.className = "modal-actions";
    const buttons: HTMLButtonElement[] = [];
    const action = (text: UiText, run: () => Promise<void> | void) => {
        const b = el("button", text);
        b.type = "button";
        b.className = "button-secondary";
        b.onclick = () => {
            void (async () => {
                try {
                    const promise = run();
                    draw();
                    await promise;
                    setUiText(message, controller.pending ? displayMessage("processingSettings.originalSummary", { p0: uiDisplayParameter(controller.pending.request.operationId), p1: uiDisplayParameter(controller.pending.operation?.status ?? uiMessage("processingSettings.resultPending")) }) : uiMessage("processingSettings.loadedHelp"));
                }
                catch (error) {
                    setUiText(message, () => `${uiTextValue(error instanceof Error ? errorUiText(error) : String(error))}${uiTextValue(controller.pending ? uiMessage("processingSettings.originalRequestRetainedSuffix") : "")}`);
                }
                draw();
            })();
        };
        buttons.push(b);
        actions.append(b);
        return b;
    };
    const prepare = action(uiMessage("processingSettings.previewChanges"), () => controller.prepare());
    const query = action(uiMessage("processingSettings.inspectOriginal"), () => controller.status());
    const retry = action(uiMessage("processingSettings.retryPrepare"), () => controller.retryPrepare());
    const settle = action(uiMessage("processingSettings.settleRetainingInput"), () => controller.settle());
    const apply = action(uiMessage("processingSettings.applyPreview"), () => controller.apply());
    const reset = action(uiMessage("processingSettings.discardInput"), () => controller.resetDraft());
    const clear = action(uiMessage("processingSettings.previewRemoveDefaults"), () => controller.prepare(true));
    const close = action(uiMessage("common.close"), () => { dialog.close(); });
    function draw(): void {
        source.value = controller.source;
        source.disabled = controller.locked;
        let parsed: ProcessingDocument | undefined;
        try {
            parsed = decodeProcessingDocument(JSON.parse(controller.source));
        }
        catch { /* Preserve malformed source draft. */ }
        languages.value = parsed?.defaults.languages.map(processingLanguageTag).join(", ") ?? "";
        languages.disabled = controller.locked || !parsed;
        prepare.disabled = controller.locked;
        query.disabled = controller.busy || !controller.pending;
        retry.disabled = controller.busy || !controller.pending;
        settle.disabled = controller.busy || !controller.pending;
        apply.disabled = controller.busy || controller.pending?.operation?.status !== "prepared";
        reset.disabled = controller.locked;
        clear.disabled = controller.locked;
        close.disabled = controller.busy;
        setUiText(warning, controller.state?.warning ? apiErrorUiText(controller.state.warning) : "");
        const operation = controller.pending?.operation, previewFacts = operation?.preview ? { beforeSource: operation.preview.beforeSource, afterSource: operation.preview.afterSource } : operation?.preview;
        setUiText(operationSummary, operation?.preview?.summary ?? "");
        setUiText(preview, controller.pending ? JSON.stringify({ operationId: controller.pending.request.operationId, beforeProcessing: controller.state?.processing ?? null, afterProcessing: controller.pending.request.processing, result: operation ? { ...operation, ...(operation.preview !== undefined ? { preview: previewFacts } : {}) } : undefined }, null, 2) : "");
        if (controller.recoveryError) {
            setUiText(preview, controller.recoveryError);
            setUiText(message, uiMessage("processingSettings.recoveryUnreadable"));
        }
    }
    source.oninput = () => {
        try {
            controller.edit(source.value);
        }
        catch (e) {
            setUiText(message, errorUiText(e));
        }
    };
    languages.onchange = () => {
        try {
            const value = decodeProcessingDocument(JSON.parse(source.value));
            value.defaults.languages = parseProcessingLanguages(languages.value);
            controller.edit(JSON.stringify(value, null, 2));
            draw();
        }
        catch (e) {
            setUiText(message, errorUiText(e));
        }
    };
    dialog.append(title, el("p", uiMessage("processingSettings.languageIndependenceHelp")), warning, languages, source, message, operationSummary, preview, actions);
    document.body.append(dialog);
    draw();
    dialog.showModal();
    await new Promise<void>(resolve => {
        dialog.addEventListener("close", () => { disposeUiTree(dialog); dialog.remove(); resolve(); }, { once: true });
        dialog.addEventListener("cancel", e => {
            if (controller.busy)
                e.preventDefault();
        });
    });
}
