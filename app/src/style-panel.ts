import { ProposalCloseGuard } from "./proposal-close-guard";
import { disposeUiTree, replaceUiChildren, joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Library style UI. Native module bytes and Core transactions own persistence. */
import { ApiError } from "./api-error";
import { descriptionUiText, type ErrorDescription } from "./ui-diagnostics";
import { decodeAppliedLibraryStyle, decodeLibraryStylePlan, decodeLibraryStyleState, decodeStyleOperationReceipt, type LibraryStylePlan, type LibraryStyleState, type NativeStyleControl, type NativeStyleValue } from "./style-wire";
export type { LibraryStylePlan, LibraryStyleState, NativeStyleControl, NativeStyleValue } from "./style-wire";
export interface LibraryStylePrepare {
    operationId: string;
    baseId: string;
    source?: string;
    changes?: Record<string, NativeStyleValue>;
    enabled?: boolean;
}
export interface LibraryStyleHost {
    read(): Promise<unknown>;
    prepare(request: LibraryStylePrepare): Promise<unknown>;
    apply(request: {
        operationId: string;
        planId: string;
    }): Promise<unknown>;
    committed?(): void;
    openSource?(path: string): void | Promise<void>;
    onApplied?(state: LibraryStyleState): void | Promise<void>;
    operationId?(): string;
}
function resultUnknown(error: unknown): boolean {
    const code = error instanceof Error && "code" in error && typeof error.code === "string" ? error.code : undefined;
    return code === undefined || ["unknown", "resultUnknown", "pending", "style_state_unavailable"].includes(code);
}
/** Keeps an uncertain request immutable and retries it with its original ID. */
export class NativeStyleController {
    private readonly closeGuard = new ProposalCloseGuard();
    assertCloseReady(): void { this.closeGuard.assertReady(this.dirty || this.locked || this.committedPendingRead); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.dirty || this.locked || this.committedPendingRead); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    bindCloseGuard(element: HTMLElement): void { this.closeGuard.bind(element); }
    runCloseTracked<T>(operation: () => Promise<T>): Promise<T> { return this.closeGuard.run(operation); }

    state: LibraryStyleState | null = null;
    source = "";
    enabled = true;
    changes: Record<string, NativeStyleValue> = {};
    plan: LibraryStylePlan | null = null;
    busy = false;
    uncertain: "prepare" | "apply" | null = null;
    committedPendingRead = false;
    private request: LibraryStylePrepare | null = null;
    private notifiedCommit: string | null = null;
    constructor(readonly host: LibraryStyleHost) { }
    get advanced(): boolean { return this.state !== null && this.source !== this.state.source; }
    get dirty(): boolean {
        return this.state !== null && (this.advanced || Object.keys(this.changes).length > 0 || this.enabled !== this.state.enabled);
    }
    get locked(): boolean { return this.busy || this.uncertain !== null; }
    async load(): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.dirty || this.locked)
                throw new UiError(uiMessage("stylePanel.unresolvedDraftCannotReread"));
            this.install(await this.host.read());
        });
    }
    setSource(source: string): void { this.assertEditable(); this.source = source; this.plan = null; this.request = null; }
    setEnabled(enabled: boolean): void { this.assertEditable(); this.enabled = enabled; this.plan = null; this.request = null; }
    resetDraft(): void { this.assertEditable(); this.install(this.state!); }
    setControl(key: string, value: NativeStyleValue): void {
        this.assertEditable();
        if (this.advanced)
            throw new UiError(uiMessage("stylePanel.sourceEditedControlsPaused"));
        const control = this.state?.controls.find(item => item.key === key);
        if (!control?.editable)
            throw new UiError(control?.reason ? descriptionUiText(control.reason) : uiMessage("stylePanel.nativeParameterNotUnique"));
        if (typeof value !== typeof control.value)
            throw new UiError(uiMessage("stylePanel.valueTypeMismatch"));
        if (value === control.value)
            delete this.changes[key];
        else
            this.changes[key] = value;
        this.plan = null;
        this.request = null;
    }
    async prepare(): Promise<LibraryStylePlan> {
        return this.closeGuard.run(async () => {
            if (!this.state || this.busy || this.uncertain === "apply")
                throw new UiError(uiMessage("stylePanel.cannotPrepare"));
            if (!this.request) {
                if (!this.dirty)
                    throw new UiError(uiMessage("stylePanel.noChanges"));
                this.request = {
                    operationId: this.host.operationId?.() ?? crypto.randomUUID(),
                    baseId: this.state.baseId,
                    ...(this.advanced ? { source: this.source } : Object.keys(this.changes).length ? { changes: { ...this.changes } } : {}),
                    ...(this.enabled !== this.state.enabled ? { enabled: this.enabled } : {}),
                };
            }
            this.busy = true;
            try {
                const plan = decodeLibraryStylePlan(await this.host.prepare({ ...this.request, ...(this.request.changes ? { changes: { ...this.request.changes } } : {}) }));
                if (plan.operationId !== this.request.operationId || !plan.planId || plan.beforeSource !== this.state.source || (this.request.source !== undefined && plan.afterSource !== this.request.source)) {
                    this.request = null;
                    this.plan = null;
                    this.uncertain = null;
                    throw Object.assign(new UiError(uiMessage("stylePanel.prepareBasisMismatch")), { code: "stylePlanMismatch" });
                }
                this.plan = plan;
                this.uncertain = null;
                return plan;
            }
            catch (error) {
                if (resultUnknown(error))
                    this.uncertain = "prepare";
                else {
                    this.request = null;
                    this.plan = null;
                    this.uncertain = null;
                }
                throw error;
            }
            finally {
                this.busy = false;
            }
        });
    }
    async apply(): Promise<LibraryStyleState> {
        return this.closeGuard.run(async () => {
            if (!this.plan || this.busy || this.uncertain === "prepare")
                throw new UiError(uiMessage("stylePanel.viewPreparedSourceFirst"));
            const plan = this.plan;
            let confirmed = false;
            this.busy = true;
            try {
                const state = decodeAppliedLibraryStyle(await this.host.apply({ operationId: plan.operationId, planId: plan.planId }));
                if (!this.originalCommit(state.operationReceipt, plan))
                    throw Object.assign(new UiError(uiMessage("stylePanel.saveReceiptPlanMismatch")), { code: "resultUnknown" });
                confirmed = true;
                const { operationReceipt, ...current } = state;
                this.install(current);
                this.notifyCommitted(plan);
                return this.state!;
            }
            catch (error) {
                if (confirmed)
                    throw error;
                if (this.committedPendingRead || resultUnknown(error)) {
                    this.uncertain = "apply";
                    if (error instanceof ApiError && error.code === "style_state_unavailable" && error.details?.kind === "committedReceipt" && this.originalCommit(error.details.receipt, plan)) {
                        this.committedPendingRead = true;
                        this.notifyCommitted(plan);
                    }
                }
                else {
                    this.request = null;
                    this.plan = null;
                    this.uncertain = null;
                }
                throw error;
            }
            finally {
                this.busy = false;
            }
        });
    }
    private originalCommit(value: unknown, plan: LibraryStylePlan): boolean {
        try { const receipt = decodeStyleOperationReceipt(value); return receipt.operationId === plan.operationId && receipt.planId === plan.planId && receipt.status === "committed"; }
        catch { return false; }
    }
    private notifyCommitted(plan: LibraryStylePlan): void {
        if (this.notifiedCommit === plan.operationId)
            return;
        this.notifiedCommit = plan.operationId;
        this.host.committed?.();
    }
    private assertEditable(): void {
        if (this.closeGuard.frozen) throw new UiError(uiMessage("shell.closeAuthorInputPending"));
        if (!this.state || this.locked)
            throw new UiError(uiMessage("stylePanel.operationPending"));
    }
    private install(incoming: unknown): void {
        const state = decodeLibraryStyleState(incoming);
        this.state = state;
        this.source = state.source;
        this.enabled = state.enabled;
        this.changes = {};
        this.plan = null;
        this.request = null;
        this.uncertain = null;
        this.committedPendingRead = false;
    }
}
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] {
    const result = document.createElement(tag);
    result.className = className;
    setUiText(result, text);
    return result;
}
export function nativeStyleControlReason(reason: ErrorDescription): UiText { return descriptionUiText(reason); }
/** Both local and hosted panels edit the same inspected native parameter. */
export function nativeStyleControlInput(control: NativeStyleControl, value: NativeStyleValue): HTMLInputElement | HTMLSelectElement {
    const localeOptions = nativeLanguageOptions(control.key, value);
    if (localeOptions.length && control.kind === "text") {
        const select = element("select", "text-input");
        select.dataset.controlKey = JSON.stringify(["library-style", control.key, "native-value"]);
        setUiAttr(select, "aria-label", control.label);
        for (const option of localeOptions) {
            const entry = element("option", "", option.label);
            entry.value = option.value;
            entry.selected = option.value === value;
            select.append(entry);
        }
        return select;
    }
    const input = element("input", "text-input");
    input.dataset.controlKey = JSON.stringify(["library-style", control.key, "native-value"]);
    setUiAttr(input, "aria-label", control.label);
    input.type = "text";
    input.value = value;
    return input;
}
/** Values remain native expressions; this never creates parallel locale metadata. */
export function nativeLanguageOptions(key: string, current: NativeStyleValue): {
    label: UiText;
    value: string;
}[] {
    const values: [
        string,
        UiText
    ][] = key === "text.lang" ? [
        ['"zh"', uiMessage("stylePanel.languageChinese")], ['"en"', "English · en"], ['"ja"', uiMessage("stylePanel.languageJapanese")], ['"ko"', "한국어 · ko"], ['"fr"', "Français · fr"], ['"de"', "Deutsch · de"], ['"es"', "Español · es"], ['"ar"', "العربية · ar"],
    ] : key === "text.region" ? [
        ["none", uiMessage("stylePanel.regionUnspecified")], ['"CN"', uiMessage("stylePanel.regionChinaMainland")], ['"HK"', uiMessage("stylePanel.regionHongKong")], ['"TW"', uiMessage("stylePanel.regionTaiwan")], ['"US"', uiMessage("stylePanel.regionUnitedStates")], ['"GB"', uiMessage("stylePanel.regionUnitedKingdom")], ['"JP"', uiMessage("stylePanel.regionJapan")], ['"KR"', uiMessage("stylePanel.regionKorea")], ['"FR"', uiMessage("stylePanel.regionFrance")], ['"DE"', uiMessage("stylePanel.regionGermany")],
    ] : [];
    if (!values.length)
        return [];
    if (!values.some(([value]) => value === current))
        values.unshift([current, displayMessage("stylePanel.retainNativeValue", { p0: uiDisplayParameter(current) })]);
    return values.map(([value, label]) => ({ value, label }));
}
/** Import style-panel.css and retain this returned controller per library.
 * Closing an uncertain operation retains its immutable request in the controller.
 */
export async function openLibraryStylePanel(host: LibraryStyleHost, controller = new NativeStyleController(host)): Promise<NativeStyleController> {
    if (!controller.state)
        await controller.load();
    const dialog = element("dialog", "modal library-style-panel");
    controller.bindCloseGuard(dialog);
    let closed = false;
    const header = element("div", "modal-heading");
    const heading = element("h2", "", uiMessage("stylePanel.title"));
    heading.id = `style-heading-${crypto.randomUUID()}`;
    const close = element("button", "button-secondary compact", uiMessage("common.close"));
    close.type = "button";
    header.append(heading, close);
    dialog.setAttribute("aria-labelledby", heading.id);
    const description = element("p", "modal-description", uiMessage("stylePanel.nativeStyleDescription"));
    const path = element("p", "style-path");
    const notices = element("div", "style-notices");
    const form = element("form", "modal-form");
    const enabledLabel = element("label", "style-enabled");
    const enabled = element("input");
    enabled.type = "checkbox";
    setUiAttr(enabled, "aria-label", uiMessage("stylePanel.automaticallyApplyStyle"));
    enabledLabel.append(enabled, uiTextNode(uiMessage("stylePanel.automaticallyApplyStyle")));
    const controls = element("div", "style-controls");
    const advanced = element("details", "style-native-source");
    advanced.append(element("summary", "", uiMessage("stylePanel.advancedNativeSource")));
    advanced.append(element("p", "field-help", uiMessage("stylePanel.sourceEditingDescription")));
    const sourceLabel = element("label", "field-label", uiMessage("stylePanel.moduleSource"));
    const source = element("textarea", "text-input style-source");
    source.spellcheck = false;
    setUiAttr(source, "aria-label", uiMessage("stylePanel.moduleSource"));
    source.rows = 16;
    sourceLabel.append(source);
    advanced.append(sourceLabel);
    if (host.openSource) {
        const sourceEntry = element("button", "text-button", uiMessage("stylePanel.openStyleFileSource"));
        sourceEntry.type = "button";
        sourceEntry.onclick = () => { void controller.runCloseTracked(async () => { await host.openSource!(controller.state!.path); }).catch(showError); };
        advanced.append(sourceEntry);
    }
    const preview = element("div", "style-plan");
    preview.hidden = true;
    const status = element("p", "style-status");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    const actions = element("div", "modal-actions");
    const reset = element("button", "button-secondary", uiMessage("stylePanel.restoreReadSettings"));
    reset.type = "button";
    const prepare = element("button", "button-secondary", uiMessage("stylePanel.viewChanges"));
    prepare.type = "button";
    const apply = element("button", "button-primary", uiMessage("stylePanel.applyLibraryStyle"));
    apply.type = "button";
    actions.append(reset, prepare, apply);
    form.append(enabledLabel, controls, notices, advanced, preview, status, actions);
    dialog.append(header, description, path, form);
    const inputs: {
        input: HTMLInputElement | HTMLSelectElement;
        control: NativeStyleControl;
    }[] = [];
    function showError(error: unknown): void {
        if (closed) return;
        status.classList.add("style-error");
        setUiText(status, joinUiText([errorUiText(error), ...(controller.committedPendingRead ? [uiMessage("stylePanel.committedRereadSuffix")] : controller.uncertain ? [uiMessage("stylePanel.unconfirmedRetrySameSuffix")] : [])]));
        refresh();
    }
    function clearPreview(): void { replaceUiChildren(preview); preview.hidden = true; }
    function edited(): void { status.classList.remove("style-error"); setUiText(status, uiMessage("stylePanel.draftUnapplied")); clearPreview(); refresh(); }
    function refresh(): void {
        if (closed) return;
        enabled.disabled = controller.locked;
        source.disabled = controller.locked;
        for (const item of inputs)
            item.input.disabled = !item.control.editable || controller.advanced || controller.locked;
        prepare.disabled = controller.busy || controller.uncertain === "apply" || (!controller.dirty && !controller.uncertain);
        apply.disabled = controller.busy || !controller.plan || controller.uncertain === "prepare";
        setUiText(prepare, controller.uncertain === "prepare" ? uiMessage("stylePanel.confirmSamePreparation") : uiMessage("stylePanel.viewChanges"));
        setUiText(apply, controller.committedPendingRead ? uiMessage("stylePanel.rereadCommittedSettings") : controller.uncertain === "apply" ? uiMessage("stylePanel.confirmSameSave") : uiMessage("stylePanel.applyLibraryStyle"));
        reset.disabled = controller.locked || !controller.dirty;
        close.disabled = controller.busy;
    }
    function drawState(): void {
        if (closed) return;
        const state = controller.state!;
        setUiText(path, state.path);
        enabled.checked = controller.enabled;
        source.value = controller.source;
        disposeUiTree(controls);
        disposeUiTree(notices);
        controls.replaceChildren();
        notices.replaceChildren();
        inputs.length = 0;
        for (const warning of state.warnings ?? [])
            notices.append(element("p", "field-help style-warning", warning));
        for (const control of state.controls) {
            const currentValue = controller.changes[control.key] ?? control.value;
            const label = element("label", "field-label", control.label);
            const input = nativeStyleControlInput(control, currentValue);
            input.onchange = () => {
                try {
                    const value = input.value;
                    controller.setControl(control.key, value);
                    edited();
                }
                catch (error) {
                    showError(error);
                }
            };
            label.append(input);
            if (control.reason)
                label.append(element("span", "field-help", nativeStyleControlReason(control.reason)));
            controls.append(label);
            inputs.push({ input, control });
        }
        if (!state.controls.length)
            notices.append(element("p", "field-help", uiMessage("stylePanel.noLosslessSimplifiedControls")));
        refresh();
    }
    enabled.onchange = () => {
        try {
            controller.setEnabled(enabled.checked);
            edited();
        }
        catch (error) {
            showError(error);
        }
    };
    source.oninput = () => {
        try {
            controller.setSource(source.value);
            edited();
        }
        catch (error) {
            showError(error);
        }
    };
    reset.onclick = () => {
        try {
            controller.resetDraft();
            drawState();
            clearPreview();
            status.classList.remove("style-error");
            setUiText(status, uiMessage("stylePanel.readStyleRestoredDiskUnchanged"));
        }
        catch (error) {
            showError(error);
        }
    };
    prepare.onclick = () => {
        void (async () => {
            const request = controller.prepare();
            refresh();
            try {
                const plan = await request;
                disposeUiTree(preview);
                preview.replaceChildren(element("h3", "", uiMessage("stylePanel.nativeChangesToSave")));
                const summary = element("ul");
                for (const line of plan.summary)
                    summary.append(element("li", "", line));
                preview.append(summary);
                const columns = element("div", "style-source-comparison");
                for (const [caption, value] of [[uiMessage("stylePanel.beforeOperation"), plan.beforeSource], [uiMessage("stylePanel.afterOperation"), plan.afterSource]] as const) {
                    const label = element("label", "field-label", caption);
                    const area = element("textarea", "text-input style-source");
                    area.readOnly = true;
                    area.value = value;
                    area.rows = 10;
                    label.append(area);
                    columns.append(label);
                }
                preview.append(columns);
                preview.hidden = false;
                status.classList.remove("style-error");
                setUiText(status, uiMessage("stylePanel.preparedCheckApply"));
                refresh();
            }
            catch (error) {
                showError(error);
            }
        })();
    };
    apply.onclick = () => {
        void controller.runCloseTracked(async () => {
            const request = controller.apply();
            refresh();
            try {
                const state = await request;
                drawState();
                clearPreview();
                status.classList.remove("style-error");
                setUiText(status, uiMessage("stylePanel.styleSavedRecompiling"));
                try {
                    await host.onApplied?.(state);
                }
                catch (error) {
                    showError(error);
                    setUiText(status, displayMessage("stylePanel.styleSavedRefreshFailed", { p0: uiDisplayParameter(errorUiText(error)) }));
                }
            }
            catch (error) {
                showError(error);
            }
        }).catch(showError);
    };
    form.onsubmit = event => { event.preventDefault(); prepare.click(); };
    document.body.append(dialog);
    drawState();
    dialog.showModal();
    await new Promise<void>(resolve => {
        function dismiss(): void {
            if (closed) return;
            try { controller.assertCloseReady(); }
            catch (error) { showError(error); return; }
            closed = true;
            dialog.close();
            disposeUiTree(dialog);
            dialog.remove();
            resolve();
        }
        close.onclick = dismiss;
        dialog.addEventListener("cancel", event => { event.preventDefault(); dismiss(); });
    });
    return controller;
}
