import { disposeUiTree, replaceUiChildren, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { decodeErrorDescription, descriptionUiText, exactKeys, type ErrorDescription } from "./ui-diagnostics";
export type HeadingNumberingScheme = "none" | "decimal" | "chinese";
export interface HeadingNumberingIntent {
    scheme: HeadingNumberingScheme;
    start: number;
    maxLevel: number;
    excludeLabels: string[];
}
export interface HeadingNumberingInspection {
    sourceHash: string;
    settings: HeadingNumberingIntent | null;
    editable: boolean;
    reason: ErrorDescription | null;
    frozenEndnotesRequired: boolean;
}
export interface NumberingAvailability extends HeadingNumberingInspection {
    availabilityMessage?: UiText;
}
export function decodeHeadingNumberingInspection(value: unknown): HeadingNumberingInspection {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new UiError(uiMessage("error.invalidResponse"));
    const item = value as Record<string, unknown>, reason = item.reason === null ? null : decodeErrorDescription(item.reason);
    const settings = item.settings as HeadingNumberingIntent | null;
    if (!exactKeys(item, ["sourceHash", "settings", "editable", "reason", "frozenEndnotesRequired"]) || typeof item.sourceHash !== "string" || typeof item.editable !== "boolean" || typeof item.frozenEndnotesRequired !== "boolean" || item.editable && item.frozenEndnotesRequired || reason === undefined || settings !== null && (!settings || typeof settings !== "object" || !exactKeys(settings as unknown as Record<string, unknown>, ["scheme", "start", "maxLevel", "excludeLabels"]) || !["none", "decimal", "chinese"].includes(settings.scheme) || !Number.isSafeInteger(settings.start) || settings.start < 1 || settings.start > 1000000 || !Number.isSafeInteger(settings.maxLevel) || settings.maxLevel < 1 || settings.maxLevel > 4294967295 || !Array.isArray(settings.excludeLabels) || settings.excludeLabels.length > 200 || settings.excludeLabels.some(label => typeof label !== "string")))
        throw new UiError(uiMessage("error.invalidResponse"));
    return { sourceHash: item.sourceHash, editable: item.editable, frozenEndnotesRequired: item.frozenEndnotesRequired, reason, settings };
}
export interface HeadingNumberingPreview {
    beforeSource: string;
    afterSource: string;
    headings?: {
        text: string;
        before: string | null;
        after: string | null;
    }[];
}
export interface NumberingHost {
    authorInputs?(): import("./author-input").AuthorInputSession | undefined;
    closeGuard?(guard: () => boolean): () => void;
    source(): string;
    inspection(): NumberingAvailability;
    preview(action: HeadingNumberingIntent): Promise<HeadingNumberingPreview>;
    apply(action: HeadingNumberingIntent): Promise<boolean>;
    personalDefaultKey: string;
    personalNoteKey: string;
    displayPersonal(enabled: boolean): void;
    readable?(): boolean;
    composing?(): boolean;
    binding?(): string;
    observeState?(element: HTMLElement, listener: () => void): () => void;
}
export class NumberingController {
    action: HeadingNumberingIntent;
    preview: HeadingNumberingPreview | null = null;
    personal: "default" | "show" | "hide" = "default";
    busy = false;
    valid = true;
    private generation = 0;
    private readonly binding: string;
    constructor(readonly host: NumberingHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
        this.action = structuredClone(host.inspection().settings ?? { scheme: "decimal", start: 1, maxLevel: 9, excludeLabels: [] });
        this.binding = host.binding?.() ?? "";
        const value = storage.getItem(host.personalNoteKey);
        if (value === "show" || value === "hide")
            this.personal = value;
    }
    get personalVisible(): boolean { return this.personal === "default" ? this.storage.getItem(this.host.personalDefaultKey) === "show" : this.personal === "show"; }
    get personalDefaultVisible(): boolean { return this.storage.getItem(this.host.personalDefaultKey) === "show"; }
    get available(): boolean { return this.host.readable?.() !== false && (this.host.binding?.() ?? "") === this.binding; }
    get canPropose(): boolean { const inspection = this.host.inspection(); return this.available && (inspection.editable || inspection.frozenEndnotesRequired); }
    invalidate(): void { this.preview = null; this.valid = false; this.generation++; }
    observeSurface(element: HTMLElement, changed: () => void): () => void {
        let source = this.host.source();
        return this.host.observeState?.(element, () => {
            const current = this.host.source();
            if (current !== source || !this.available) {
                source = current;
                this.invalidate();
            }
            changed();
        }) ?? (() => { });
    }
    setPersonal(mode: "default" | "show" | "hide"): void {
        if (mode === "default")
            this.storage.removeItem(this.host.personalNoteKey);
        else
            this.storage.setItem(this.host.personalNoteKey, mode);
        this.personal = mode;
        this.host.displayPersonal(this.personalVisible);
    }
    setPersonalDefault(enabled: boolean): void { this.storage.setItem(this.host.personalDefaultKey, enabled ? "show" : "hide"); this.host.displayPersonal(this.personalVisible); }
    change(action: HeadingNumberingIntent): void {
        if (this.busy || !this.available || this.host.composing?.())
            throw new UiError(uiMessage("numberingPanel.previewOrCompositionPending"));
        this.preview = null;
        this.generation++;
        this.valid = false;
        if (!this.canPropose)
            throw new UiError(uiMessage("numberingPanel.nativeRuleNotLosslesslyEditable"));
        if (!Number.isSafeInteger(action.start) || action.start < 1 || action.start > 1000000 || !Number.isSafeInteger(action.maxLevel) || action.maxLevel < 1 || action.maxLevel > 4294967295 || action.excludeLabels.length > 200)
            throw new UiError(uiMessage("numberingPanel.checkStartLevelExclusions"));
        this.action = structuredClone(action);
        this.valid = true;
    }
    async prepare(): Promise<void> {
        if (this.busy || !this.valid || !this.available || this.host.composing?.() || !this.canPropose)
            throw new UiError(uiMessage("numberingPanel.cannotPrepareRuleOrInput"));
        const source = this.host.source(), action = structuredClone(this.action), generation = ++this.generation;
        this.busy = true;
        try {
            const preview = await this.host.preview(action);
            if (generation !== this.generation || !this.available || source !== this.host.source() || preview.beforeSource !== source)
                throw new UiError(uiMessage("numberingPanel.bodyPermissionOrInputChanged"));
            this.preview = structuredClone(preview);
        }
        finally {
            this.busy = false;
        }
    }
    async apply(): Promise<boolean> {
        if (this.busy || !this.preview || !this.available || this.host.composing?.() || this.preview.beforeSource !== this.host.source() || !this.canPropose)
            throw new UiError(uiMessage("numberingPanel.viewCurrentPreviewFirst"));
        this.busy = true;
        try {
            const applied = await this.host.apply(structuredClone(this.action));
            if (applied)
                this.preview = null;
            return applied;
        }
        finally {
            this.busy = false;
        }
    }
    cancel(): void { this.generation++; this.preview = null; }
}
const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); setUiText(e, text); return e; };
export async function openNumberingPanel(host: NumberingHost): Promise<void> {
    const focus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const controller = new NumberingController(host), dialog = el("dialog");
    dialog.className = "modal library-style-panel numbering-dialog";
    setUiAttr(dialog, "aria-label", uiMessage("numberingPanel.dialogLabel"));
    const authorInputs = host.authorInputs?.(), stopCloseGuard = host.closeGuard?.(() => dialog.open);
    let composing = false;
    dialog.addEventListener("compositionstart", () => { composing = true; controller.invalidate(); draw(); });
    dialog.addEventListener("compositionend", () => { composing = false; changed(); });
    const title = el("h2", uiMessage("numberingPanel.title"));
    const effective = el("section");
    effective.className = "numbering-effective-settings";
    const personal = el("select");
    setUiAttr(personal, "aria-label", uiMessage("numberingPanel.personalVisibility"));
    for (const [value, text] of [["default", uiMessage("numberingPanel.followPersonalDefault")], ["show", uiMessage("numberingPanel.showPersonalNumbers")], ["hide", uiMessage("numberingPanel.hidePersonalNumbers")]] as const) {
        const o = el("option", text);
        o.value = value;
        personal.append(o);
    }
    personal.value = controller.personal;
    personal.onchange = () => {
        try {
            controller.setPersonal(personal.value as "default" | "show" | "hide");
            draw();
        }
        catch (e) {
            setUiText(message, errorUiText(e));
        }
    };
    const ownDefault = el("input");
    ownDefault.type = "checkbox";
    ownDefault.checked = localStorage.getItem(host.personalDefaultKey) === "show";
    ownDefault.onchange = () => {
        try {
            controller.setPersonalDefault(ownDefault.checked);
            draw();
        }
        catch (e) {
            setUiText(message, errorUiText(e));
        }
    };
    const defaultLabel = el("label", uiMessage("numberingPanel.showPersonalNumbersByDefault"));
    defaultLabel.append(ownDefault);
    const scheme = el("select");
    setUiAttr(scheme, "aria-label", uiMessage("numberingPanel.documentScheme"));
    for (const [value, text] of [["none", uiMessage("numberingPanel.noNumbering")], ["decimal", uiMessage("numberingPanel.decimalHierarchy")], ["chinese", uiMessage("numberingPanel.chineseHierarchy")]] as const) {
        const o = el("option", text);
        o.value = value;
        scheme.append(o);
    }
    scheme.value = controller.action.scheme;
    const start = el("input");
    start.type = "number";
    start.min = "1";
    start.max = "1000000";
    start.value = String(controller.action.start);
    setUiAttr(start, "aria-label", uiMessage("numberingPanel.documentStart"));
    const level = el("input");
    level.type = "number";
    level.min = "1";
    level.max = "4294967295";
    level.value = String(controller.action.maxLevel);
    setUiAttr(level, "aria-label", uiMessage("numberingPanel.documentMaxLevel"));
    const exclude = el("input");
    exclude.value = controller.action.excludeLabels.join(", ");
    setUiAttr(exclude, "placeholder", uiMessage("numberingPanel.excludedLabelPlaceholder"));
    setUiAttr(exclude, "aria-label", uiMessage("numberingPanel.documentExcludedLabels"));
    authorInputs?.bind(start, "numbering-start");
    authorInputs?.bind(level, "numbering-level");
    authorInputs?.bind(exclude, "numbering-exclude");
    const message = el("p");
    message.setAttribute("role", "status");
    const preview = el("div");
    const actions = el("div");
    actions.className = "modal-actions";
    const button = (text: UiText, run: () => void | Promise<void>) => {
        const b = el("button", text);
        b.type = "button";
        b.className = "button-secondary";
        b.onclick = () => {
            void (async () => {
                try {
                    if (composing || host.composing?.())
                        throw new UiError(uiMessage("numberingPanel.finishCompositionFirst"));
                    const promise = run();
                    draw();
                    await promise;
                }
                catch (e) {
                    setUiText(message, errorUiText(e));
                }
                draw();
            })();
        };
        actions.append(b);
        return b;
    };
    const prepare = button(uiMessage("numberingPanel.previewDocumentNumbering"), () => controller.prepare());
    const apply = button(uiMessage("numberingPanel.applyDocumentDraftChange"), async () => {
        if (await controller.apply())
            setUiText(message, uiMessage("numberingPanel.appliedToNativeDraft"));
    });
    const close = button(uiMessage("common.close"), () => { controller.cancel(); dialog.close(); });
    function changed(): void {
        try {
            controller.change({ scheme: scheme.value as HeadingNumberingScheme, start: Number(start.value), maxLevel: Number(level.value), excludeLabels: exclude.value.split(/[,，]+/).map(s => s.trim()).filter(Boolean) });
            setUiText(message, uiMessage("numberingPanel.inputUnapplied"));
        }
        catch (e) {
            setUiText(message, errorUiText(e));
        }
        draw();
    }
    for (const input of [scheme, start, level, exclude]) {
        input.onchange = changed;
        input.oninput = event => {
            controller.invalidate();
            if (!(event as InputEvent).isComposing && !composing)
                changed();
            else
                draw();
        };
    }
    function draw(): void {
        if (!controller.available)
            controller.invalidate();
        const inspection = controller.available ? host.inspection() : { editable: false, frozenEndnotesRequired: false, reason: null, availabilityMessage: uiMessage("numberingPanel.noteOrReadPermissionChanged") }, canPropose = controller.available && controller.canPropose;
        replaceUiChildren(effective);
        effective.append(el("h3", uiMessage("numberingPanel.effectiveSettings")));
        const summary = el("dl");
        summary.append(el("dt", uiMessage("numberingPanel.followPersonalDefault")), el("dd", controller.personalDefaultVisible ? uiMessage("numberingPanel.personalDefaultShown") : uiMessage("numberingPanel.personalDefaultHidden")),
            el("dt", uiMessage("numberingPanel.personalVisibility")), el("dd", controller.personal === "default" ? uiMessage("numberingPanel.followPersonalDefault") : controller.personal === "show" ? uiMessage("numberingPanel.showPersonalNumbers") : uiMessage("numberingPanel.hidePersonalNumbers")),
            el("dt", uiMessage("numberingPanel.effectiveSettings")), el("dd", controller.personalVisible ? uiMessage("numberingPanel.showPersonalNumbers") : uiMessage("numberingPanel.hidePersonalNumbers")));
        const currentSettings = "settings" in inspection ? inspection.settings : null;
        summary.append(el("dt", uiMessage("numberingPanel.documentNativeOverride")));
        if (!controller.available) summary.append(el("dd", uiMessage("numberingPanel.noteOrReadPermissionChanged")));
        else if (currentSettings) {
            const schemeLabel = currentSettings.scheme === "none" ? uiMessage("numberingPanel.noNumbering") : currentSettings.scheme === "chinese" ? uiMessage("numberingPanel.chineseHierarchy") : uiMessage("numberingPanel.decimalHierarchy");
            summary.append(el("dd", schemeLabel), el("dt", uiMessage("numberingPanel.documentStart")), el("dd", String(currentSettings.start)), el("dt", uiMessage("numberingPanel.documentMaxLevel")), el("dd", String(currentSettings.maxLevel)), el("dt", uiMessage("numberingPanel.documentExcludedLabels")), el("dd", currentSettings.excludeLabels.join(", ") || "—"));
        } else summary.append(el("dd", uiMessage("numberingPanel.followNativeDocument")));
        effective.append(summary);
        personal.disabled = !controller.available;
        ownDefault.disabled = !controller.available;
        for (const input of [scheme, start, level, exclude])
            input.disabled = controller.busy || !canPropose;
        prepare.disabled = controller.busy || composing || !controller.valid || !canPropose;
        apply.disabled = controller.busy || composing || !controller.preview || !canPropose;
        close.disabled = controller.busy;
        replaceUiChildren(preview);
        if (canPropose && inspection.frozenEndnotesRequired)
            preview.append(el("p", uiMessage("numberingPanel.endnoteModulePreviewHint")));
        else if (!inspection.editable)
            preview.append(el("p", displayMessage("numberingPanel.complexRuleRetained", { p0: inspection.availabilityMessage ?? (inspection.reason ? descriptionUiText(inspection.reason) : uiMessage("numberingPanel.unknownRule")) })));
        if (controller.preview) {
            preview.append(el("h3", uiMessage("numberingPanel.nativeChangePreview")));
            for (const h of controller.preview.headings ?? [])
                preview.append(el("p", () => `${uiTextValue(h.text)}：${uiTextValue(h.before ?? uiMessage("numberingPanel.noNumbering"))} → ${uiTextValue(h.after ?? uiMessage("numberingPanel.noNumbering"))}`));
            const details = el("details");
            details.append(el("summary", uiMessage("numberingPanel.exactSourceComparison")), el("pre", controller.preview.beforeSource), el("pre", controller.preview.afterSource));
            preview.append(details);
        }
    }
    dialog.append(title, effective, el("h3", uiMessage("numberingPanel.personalDisplay")), el("p", uiMessage("numberingPanel.personalDisplayDescription")), personal, defaultLabel, el("h3", uiMessage("numberingPanel.documentNativeOverride")), el("p", uiMessage("numberingPanel.documentNumberingDescription")), scheme, start, level, exclude, preview, message, actions);
    document.body.append(dialog);
    const stopObserving = controller.observeSurface(dialog, draw);
    draw();
    dialog.showModal();
    await new Promise<void>(resolve => {
        dialog.addEventListener("close", () => {
            stopCloseGuard?.();
            authorInputs?.retire();
            stopObserving();
            controller.cancel();
            disposeUiTree(dialog);
            dialog.remove();
            if (focus?.isConnected)
                focus.focus();
            resolve();
        }, { once: true });
        dialog.addEventListener("cancel", e => {
            if (controller.busy || composing || host.composing?.())
                e.preventDefault();
        });
    });
}
