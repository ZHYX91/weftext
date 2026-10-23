import { joinUiText } from "./ui-locale";
import { decodeApiErrorBody, apiErrorUiText, type ApiErrorBody } from "./api-error";
import { decodeCoreDescription, coreDescriptionUiText, type CoreDescription } from "./ui-diagnostics";
import type { Inventory, OpenedDocument } from "./types";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { nativeLanguageOptions } from "./style-panel";
export interface InitialLibraryStyle {
    source: string;
    enabled: boolean;
}
export interface InitialRootNote {
    title: string;
    queries: string[];
}
export interface LibraryCreateOptions {
    initialStyle?: InitialLibraryStyle | null;
    initialRoot?: InitialRootNote;
}
export interface LibraryCreationRequest {
    path: string;
    options: LibraryCreateOptions;
    operationId: string;
}
export interface LibraryCreationReceipt {
    operationId: string;
    root: string;
    workspaceId: string;
    status: string;
    rootNotePath?: string;
    library?: Inventory;
    rootDocument?: OpenedDocument;
    deliveryError?: ApiErrorBody;
}
function typedCreationReceipt(value: LibraryCreationReceipt): LibraryCreationReceipt {
  if (!value || typeof value !== "object" || Object.hasOwn(value,"message")) throw new UiError(uiMessage("libraryCreation.originalReceiptInvalid"));
  if (!Object.hasOwn(value,"deliveryError")) return value;
  const deliveryError = decodeApiErrorBody(value.deliveryError);
  if (!deliveryError) throw new UiError(uiMessage("libraryCreation.deliveryErrorInvalid"));
  return {...value,deliveryError};
}
interface CreationSlot {
    key: string;
    value: string;
    editable: boolean;
    range: {
        utf16From: number;
        utf16To: number;
        raw: string;
    } | null;
}
export interface LibraryCreationDefaults {
    source: string;
    sourceHash: string;
    controls: CreationSlot[];
    rootPresets?: {
        id: string;
        title: string;
        plan: unknown;
    }[];
}
export interface LibraryCreationRecovery {
    operationId: string;
    status: string;
    description: CoreDescription;
    allowedActions?: string[];
    [key: string]: unknown;
}
export interface LibraryCreationHost {
    defaults(): Promise<LibraryCreationDefaults>;
    create(request: LibraryCreationRequest): Promise<LibraryCreationReceipt>;
    delivery(operationId: string): Promise<LibraryCreationReceipt>;
    deliver(receipt: LibraryCreationReceipt): Promise<void>;
    inspect(operationId: string): Promise<LibraryCreationRecovery>;
    resolve(operationId: string, action: string): Promise<LibraryCreationRecovery>;
    cancel?(request: LibraryCreationRequest): Promise<LibraryCreationRecovery>;
}
interface CreationDraft {
    path: string;
    language: string;
    region: string;
    defaults: LibraryCreationDefaults | null;
    rootMode?: "none" | "blank" | "queries";
    rootTitle?: string;
    queries?: string[];
}
interface RetainedCreation {
    version: 1;
    draft: CreationDraft;
    request: LibraryCreationRequest | null;
    receipt: LibraryCreationReceipt | null;
}
const key = "weftext-library-creation-v1";
function defaultsValid(value: LibraryCreationDefaults): boolean {
    return !!value && typeof value.source === "string" && typeof value.sourceHash === "string" && Array.isArray(value.controls) && ["text.lang", "text.region"].every(key => {
        const matches = value.controls.filter(c => c.key === key), slot = matches[0];
        return matches.length === 1 && slot.editable && slot.range && Number.isSafeInteger(slot.range.utf16From) && Number.isSafeInteger(slot.range.utf16To) && slot.range.utf16From >= 0 && slot.range.utf16From < slot.range.utf16To && slot.range.utf16To <= value.source.length && value.source.slice(slot.range.utf16From, slot.range.utf16To) === slot.range.raw && slot.value === slot.range.raw;
    });
}
export function initialLibraryStyle(defaults: LibraryCreationDefaults, language: string, region: string): InitialLibraryStyle {
    if (!defaultsValid(defaults))
        throw new UiError(uiMessage("libraryCreation.nativeLocaleParameterUnproven"));
    if (!/^"[A-Za-z]{2,8}"$/.test(language) || !(region === "none" || /^"(?:[A-Za-z]{2}|[0-9]{3})"$/.test(region)))
        throw new UiError(uiMessage("libraryCreation.nativeLocaleCodeInvalid"));
    const changes = [["text.lang", language], ["text.region", region]].map(([key, value]) => ({ ...defaults.controls.find(c => c.key === key)!.range!, value })).sort((a, b) => b.utf16From - a.utf16From);
    if (changes[1].utf16To > changes[0].utf16From)
        throw new UiError(uiMessage("libraryCreation.nativeParameterOverlap"));
    let source = defaults.source;
    for (const change of changes)
        source = source.slice(0, change.utf16From) + change.value + source.slice(change.utf16To);
    return { source, enabled: true };
}
function validRequest(value: LibraryCreationRequest): boolean {
    if (value?.options?.initialRoot && (typeof value.options.initialRoot.title !== "string" || !Array.isArray(value.options.initialRoot.queries) || value.options.initialRoot.queries.some(id => typeof id !== "string")))
        return false;
    return !!value && typeof value.path === "string" && !!value.path && /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(value.operationId) && !!value.options && !!value.options.initialStyle && typeof value.options.initialStyle.source === "string" && typeof value.options.initialStyle.enabled === "boolean";
}
/** The exact native bytes and original ID are retained before the first write. */
export class LibraryCreationController {
    draft: CreationDraft = { path: "", language: '"zh"', region: '"CN"', defaults: null };
    request: LibraryCreationRequest | null = null;
    receipt: LibraryCreationReceipt | null = null;
    proposal: LibraryCreationRequest | null = null;
    recovery: LibraryCreationRecovery | null = null;
    recoveryError: string | null = null;
    busy = false;
    constructor(readonly host: LibraryCreationHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
        const raw = storage.getItem(key);
        if (!raw)
            return;
        try {
            const value = JSON.parse(raw) as RetainedCreation;
            if (value.version !== 1 || !value.draft || typeof value.draft.path !== "string" || typeof value.draft.language !== "string" || typeof value.draft.region !== "string" || value.draft.defaults !== null && !defaultsValid(value.draft.defaults) || value.request !== null && !validRequest(value.request) || value.receipt && (!value.request || value.receipt.operationId !== value.request.operationId || typeof value.receipt.root !== "string" || typeof value.receipt.workspaceId !== "string"))
                throw new UiError("invalid creation record");
            this.draft = value.draft;
            this.request = value.request;
            this.receipt = value.receipt ? typedCreationReceipt(value.receipt) : null;
        }
        catch {
            this.recoveryError = raw;
        }
    }
    get locked(): boolean { return this.busy || this.request !== null || this.recoveryError !== null; }
    async load(): Promise<void> {
        if (this.request || this.recoveryError)
            return;
        if (this.draft.defaults) {
            if (!this.draft.defaults.rootPresets)
                this.draft.defaults.rootPresets = (await this.host.defaults()).rootPresets;
            this.retain();
            return;
        }
        const defaults = await this.host.defaults();
        if (!defaultsValid(defaults))
            throw new UiError(uiMessage("libraryCreation.initialStyleUnparseable"));
        this.draft.defaults = structuredClone(defaults);
        this.draft.language = defaults.controls.find(c => c.key === "text.lang")!.value;
        this.draft.region = defaults.controls.find(c => c.key === "text.region")!.value;
        this.retain();
    }
    change(update: Partial<Pick<CreationDraft, "path" | "language" | "region" | "rootMode" | "rootTitle" | "queries">>): void {
        if (this.locked)
            throw new UiError(uiMessage("libraryCreation.originalResultUnconfirmed"));
        this.draft = { ...this.draft, ...update };
        this.proposal = null;
        this.retain();
    }
    preview(): void {
        if (this.locked || !this.draft.defaults)
            throw new UiError(uiMessage("libraryCreation.readInitialStyleOrConfirmOriginal"));
        const path = this.draft.path.trim();
        if (!/^(?:[A-Za-z]:[\\/]|\\\\[^\\]+\\[^\\]+[\\/]|\/)/.test(path))
            throw new UiError(uiMessage("libraryCreation.newAbsolutePathRequired"));
        this.proposal = { path, options: { initialStyle: initialLibraryStyle(this.draft.defaults, this.draft.language, this.draft.region) }, operationId: crypto.randomUUID() };
        if (this.draft.rootMode && this.draft.rootMode !== "none") {
            const queries = this.draft.rootMode === "queries" ? this.draft.queries ?? [] : [];
            if (queries.some(id => !this.draft.defaults!.rootPresets?.some(p => p.id === id)) || new Set(queries).size !== queries.length)
                throw new UiError(uiMessage("libraryCreation.presetQueryUnavailable"));
            this.proposal.options.initialRoot = { title: this.draft.rootTitle ?? (this.draft.rootMode === "blank" ? "" : "工作台"), queries: [...queries] };
        }
    }
    async create(): Promise<LibraryCreationReceipt> {
        if (this.busy || this.recoveryError || !this.request && !this.proposal)
            throw new UiError(uiMessage("libraryCreation.previewCreationFirst"));
        if (!this.request) {
            this.request = structuredClone(this.proposal!);
            try {
                this.retain();
            }
            catch (error) {
                this.request = null;
                throw error;
            }
        }
        this.busy = true;
        try {
            const receipt = typedCreationReceipt(this.receipt ? await this.host.delivery(this.request.operationId) : await this.host.create(structuredClone(this.request)));
            if (receipt.operationId !== this.request.operationId || receipt.status !== "committed" || !receipt.workspaceId || !receipt.root || this.receipt && (receipt.workspaceId !== this.receipt.workspaceId || receipt.root !== this.receipt.root))
                throw new UiError(uiMessage("libraryCreation.receiptOperationMismatch"));
            this.receipt = typedCreationReceipt(structuredClone(receipt));
            this.retain();
            return receipt;
        }
        finally {
            this.busy = false;
        }
    }
    async inspect(): Promise<void> {
        if (!this.request || this.busy)
            throw new UiError(uiMessage("libraryCreation.noOriginalToCheck"));
        this.busy = true;
        try {
            this.installRecovery(await this.host.inspect(this.request.operationId));
        }
        finally {
            this.busy = false;
        }
    }
    async resolve(action: string): Promise<void> {
        if (!this.request || this.busy || !this.recovery?.allowedActions?.includes(action))
            throw new UiError(uiMessage("libraryCreation.checkAllowedRecoveryAction"));
        this.busy = true;
        try {
            this.installRecovery(await this.host.resolve(this.request.operationId, action));
        }
        finally {
            this.busy = false;
        }
    }
    async cancel(): Promise<void> {
        if (!this.request || this.busy || !this.host.cancel)
            throw new UiError(uiMessage("libraryCreation.originalCannotEnd"));
        this.busy = true;
        try {
            this.installRecovery(await this.host.cancel(structuredClone(this.request)));
        }
        finally {
            this.busy = false;
        }
    }
  async deliver(): Promise<LibraryCreationReceipt> {
    const receipt = await this.create();
    this.busy = true;
    try {
      if (receipt.deliveryError) throw new UiError(joinUiText([uiMessage("libraryCreation.committedDeliveryPendingPrefix"), apiErrorUiText(receipt.deliveryError!)]));
      await this.host.deliver(receipt);
      this.storage.removeItem(key);
      return receipt;
    } finally { this.busy = false; }
  }
    private installRecovery(result: LibraryCreationRecovery): void {
        if (!this.request || result.operationId !== this.request.operationId)
            throw new UiError(uiMessage("libraryCreation.recoveryOperationMismatch"));
        if (this.receipt && result.status !== "committed") throw new UiError(uiMessage("libraryCreation.committedStatusMismatch"));
        const description = decodeCoreDescription(result.description);
        if (!description || Object.hasOwn(result,"message")) throw new UiError(uiMessage("libraryCreation.recoveryProtocolInvalid"));
        this.recovery = {...structuredClone(result),description};
        if (result.status === "resolved" || result.status === "cancelled") {
            this.request = null;
            this.proposal = null;
            this.receipt = null;
        }
        this.retain();
    }
    private retain(): void {
        const old = this.storage.getItem(key);
        if (old) {
            const saved = JSON.parse(old) as RetainedCreation;
            const ended = this.recovery && ["resolved", "cancelled"].includes(this.recovery.status) && this.recovery.operationId === saved.request?.operationId;
            if (saved.request && saved.request.operationId !== this.request?.operationId && !ended)
                throw new UiError(displayMessage("libraryCreation.otherWindowRetainsOriginal", { p0: uiDisplayParameter(saved.request.operationId) }));
        }
        this.storage.setItem(key, JSON.stringify({ version: 1, draft: this.draft, request: this.request, receipt: this.receipt }));
    }
}
const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); setUiText(e, text); return e; };
let activeCreation: Promise<LibraryCreationReceipt | null> | null = null;
export function openLibraryCreation(host: LibraryCreationHost): Promise<LibraryCreationReceipt | null> {
    if (!activeCreation)
        activeCreation = showLibraryCreation(host).finally(() => { activeCreation = null; });
    return activeCreation;
}
async function showLibraryCreation(host: LibraryCreationHost): Promise<LibraryCreationReceipt | null> {
    const controller = new LibraryCreationController(host);
    await controller.load();
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = el("dialog");
    dialog.className = "modal library-creation-dialog";
    setUiAttr(dialog, "aria-label", uiMessage("libraryCreation.dialogLabel"));
    const path = el("input");
    path.className = "text-input";
    setUiAttr(path, "aria-label", uiMessage("libraryCreation.newLibraryPath"));
    setUiAttr(path, "placeholder", uiMessage("shell.libraryPathPlaceholder"));
    const language = el("select"), region = el("select");
    language.className = region.className = "text-input";
    setUiAttr(language, "aria-label", uiMessage("libraryCreation.initialDocumentLanguage"));
    setUiAttr(region, "aria-label", uiMessage("libraryCreation.initialDocumentRegion"));
    const message = el("p");
    message.setAttribute("role", "status");
    const preview = el("div"), actions = el("div");
    actions.className = "modal-actions";
    actions.style.flexWrap = "wrap";
    const rootMode = el("select");
    setUiAttr(rootMode, "aria-label", uiMessage("libraryCreation.initialRootContent"));
    for (const [value, label] of [["none", uiMessage("libraryCreation.rootModeNone")], ["blank", uiMessage("libraryCreation.rootModeBlank")], ["queries", uiMessage("libraryCreation.rootModeQueries")]] as const) {
        const option = el("option", label);
        option.value = value;
        rootMode.append(option);
    }
    const rootTitle = el("input");
    setUiAttr(rootTitle, "aria-label", uiMessage("libraryCreation.rootTitle"));
    const queryChoices = el("fieldset");
    queryChoices.append(el("legend", uiMessage("libraryCreation.selectInitialQueries")));
    for (const preset of controller.draft.defaults?.rootPresets ?? []) {
        const label = el("label", preset.title), input = el("input");
        input.type = "checkbox";
        input.value = preset.id;
        setUiAttr(input, "aria-label", displayMessage("libraryCreation.initialQueryAria", { p0: uiDisplayParameter(preset.title) }));
        input.onchange = () => { controller.change({ queries: [...queryChoices.querySelectorAll<HTMLInputElement>("input:checked")].map(input => input.value) }); draw(); };
        label.prepend(input);
        queryChoices.append(label);
    }
    let composing = false, finish: (value: LibraryCreationReceipt | null) => void = () => { };
    const button = (label: UiText, run: () => Promise<void> | void) => {
        const b = el("button", label);
        b.type = "button";
        b.className = "button-secondary";
        b.onclick = () => {
            void (async () => {
                try {
                    if (composing)
                        return;
                    const promise = run();
                    draw();
                    await promise;
                }
                catch (error) {
                    setUiText(message, () => `${uiTextValue(errorUiText(error))}${uiTextValue(controller.request ? displayMessage("libraryCreation.originalChoicesRetainedSuffix", { p0: uiDisplayParameter(controller.request.operationId) }) : "")}`);
                }
                draw();
            })();
        };
        actions.append(b);
        return b;
    };
    const prepare = button(uiMessage("libraryCreation.previewCreation"), () => { controller.preview(); setUiText(message, uiMessage("libraryCreation.checkPathAndLayoutDefaults")); });
    const create = button(uiMessage("shell.createLibrary"), async () => { const receipt = await controller.deliver(); finish(receipt); });
    const inspect = button(uiMessage("libraryCreation.viewOriginalRecovery"), () => controller.inspect());
    const cancel = button(uiMessage("libraryCreation.endUnstartedOriginal"), () => controller.cancel());
    const close = button(uiMessage("common.close"), () => finish(null));
    function choice(input: HTMLSelectElement, key: string, current: string): void {
        input.replaceChildren();
        for (const option of nativeLanguageOptions(key, current)) {
            const o = el("option", option.label);
            o.value = option.value;
            o.selected = option.value === current;
            input.append(o);
        }
    }
    function draw(): void {
        path.value = controller.draft.path;
        choice(language, "text.lang", controller.draft.language);
        choice(region, "text.region", controller.draft.region);
        for (const input of [path, language, region])
            input.disabled = controller.locked;
        rootMode.value = controller.draft.rootMode ?? "none";
        rootMode.disabled = controller.locked;
        rootTitle.value = controller.draft.rootTitle ?? (rootMode.value === "queries" ? "工作台" : "");
        rootTitle.hidden = rootMode.value === "none";
        rootTitle.disabled = controller.locked;
        queryChoices.hidden = rootMode.value !== "queries";
        queryChoices.disabled = controller.locked;
        queryChoices.querySelectorAll<HTMLInputElement>("input").forEach(input => { input.checked = controller.draft.queries?.includes(input.value) ?? false; });
        prepare.disabled = controller.locked;
        create.disabled = controller.busy || !controller.request && !controller.proposal;
        setUiText(create, controller.request ? uiMessage("libraryCreation.confirmSameCreation") : uiMessage("shell.createLibrary"));
        inspect.disabled = controller.busy || !controller.request;
        cancel.disabled = controller.busy || !controller.request || !host.cancel;
        close.disabled = controller.busy;
        preview.replaceChildren();
        const request = controller.request ?? controller.proposal;
        if (request?.options.initialRoot) {
            const root = request.options.initialRoot, name = request.path.replace(/[\\/]+$/, "").split(/[\\/]/).at(-1)!;
            preview.append(el("p", displayMessage("libraryCreation.newRootPreview", { p0: uiDisplayParameter(request.path), p1: uiDisplayParameter(name), p2: uiDisplayParameter(root.title || uiMessage("libraryCreation.untitled")) })), el("p", uiMessage("libraryCreation.rootIdentityDescription")));
            if (root.queries.length) {
                preview.append(el("p", uiMessage("libraryCreation.initialSchemaDescription")));
                for (const id of root.queries) {
                    const preset = controller.draft.defaults?.rootPresets?.find(p => p.id === id);
                    preview.append(el("h3", preset?.title ?? id), el("pre", JSON.stringify(preset?.plan, null, 2)));
                }
            }
        }
        if (request) {
            const details = el("details");
            details.append(el("summary", uiMessage("libraryCreation.advancedNativeInitialStyle")), el("pre", request.options.initialStyle!.source));
            preview.append(el("p", displayMessage("hostedReview.targetSummary", { p0: uiDisplayParameter(request.path) })), el("p", displayMessage("libraryCreation.originalOperation", { p0: uiDisplayParameter(request.operationId) })), el("p", uiMessage("libraryCreation.nativeLocaleDefaultsDescription")), details);
        }
        if (controller.recoveryError)
            preview.append(el("p", uiMessage("libraryCreation.recoveryRecordUnparseable")), el("pre", controller.recoveryError));
        if (controller.recovery) {
            preview.append(el("p", displayMessage("libraryCreation.originalOperationStatus", { p0: uiDisplayParameter(controller.recovery.status), p1: uiDisplayParameter(coreDescriptionUiText(controller.recovery.description)) })));
            for (const action of controller.recovery.allowedActions ?? []) {
                const b = el("button", ({ keepCurrent: uiMessage("libraryCreation.keepStateEndOriginal"), adoptAfterImages: uiMessage("libraryCreation.verifyOriginalWrittenContents"), resumeOriginalPlan: uiMessage("libraryCreation.continueOnlyMissingParts") } as Record<string, UiText>)[action] ?? action);
                b.type = "button";
                b.disabled = controller.busy;
                b.onclick = () => { void controller.resolve(action).then(draw).catch(error => { setUiText(message, String(error)); draw(); }); };
                preview.append(b);
            }
        }
    }
    path.oninput = () => {
        try {
            controller.change({ path: path.value });
            preview.replaceChildren();
            create.disabled = true;
        }
        catch (e) {
            setUiText(message, String(e));
        }
    };
    language.onchange = () => {
        try {
            controller.change({ language: language.value });
            draw();
        }
        catch (e) {
            setUiText(message, String(e));
        }
    };
    region.onchange = () => {
        try {
            controller.change({ region: region.value });
            draw();
        }
        catch (e) {
            setUiText(message, String(e));
        }
    };
    rootMode.onchange = () => { controller.change({ rootMode: rootMode.value as CreationDraft["rootMode"] }); draw(); };
    rootTitle.oninput = () => { controller.change({ rootTitle: rootTitle.value }); preview.replaceChildren(); create.disabled = true; };
    dialog.addEventListener("compositionstart", () => { composing = true; });
    dialog.addEventListener("compositionend", () => { composing = false; });
    dialog.addEventListener("keydown", event => event.stopPropagation());
    dialog.append(el("h2", uiMessage("shell.createLibrary")), el("p", uiMessage("libraryCreation.createLibraryDescription")), path, el("label", uiMessage("libraryCreation.documentLanguage")), language, el("label", uiMessage("libraryCreation.documentRegion")), region, el("label", uiMessage("libraryCreation.initialRootContent")), rootMode, rootTitle, queryChoices, preview, message, actions);
    document.body.append(dialog);
    draw();
    dialog.showModal();
    path.focus();
    return new Promise(resolve => {
        finish = value => {
            if (controller.busy || composing)
                return;
            dialog.close();
            dialog.remove();
            if (previousFocus?.isConnected)
                previousFocus.focus();
            resolve(value);
        };
        dialog.addEventListener("cancel", event => { event.preventDefault(); finish(null); });
    });
}
