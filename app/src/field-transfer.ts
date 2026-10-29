import { action, fieldNames, formatNativeValue, labelFor, ui, type PanelHost } from "./native-value-form";
import { replaceUiChildren, setUiText, displayMessage, message, errorUiText, uiDisplayParameter, UiError } from "./ui-locale";
import { decodeFieldTransferPatch, fieldTransferSelection, fieldTransferSourceHash, transferContextKey, transferLists,
    type FieldTransferContext, type FieldTransferInputs } from "./field-transfer-state";
export type { FieldTransferContext } from "./field-transfer-state";

export interface FieldTransferHost {
    getContext(): FieldTransferContext | null;
    isCurrent(context: FieldTransferContext): boolean;
    /** The DataPanel must reject all unapplied/raw/type/schema proposals and IME. */
    canTransfer(): boolean;
    request: PanelHost["request"];
    applyPatch: PanelHost["applyPatch"];
    notify: PanelHost["notify"];
    /** The real DataPanel owner guards request, patch delivery and final refresh. */
    run(operation: () => Promise<void>): Promise<void>;
}

/** Selection-only UI. Field values always come from the current owned projection. */
export class FieldTransfer {
    readonly element = ui("section", "data-field-transfer");
    private context: FieldTransferContext | null = null;
    private contextKey: string | null = null;
    private version = 0;
    private generation = 0;
    private busy = false;
    private composing = false;
    private pendingRefresh = false;
    private inputs: FieldTransferInputs = { fromField: "", toField: "", first: "1", last: "1", position: "1" };
    private controls: (HTMLInputElement | HTMLSelectElement | HTMLButtonElement)[] = [];
    private apply: HTMLButtonElement | null = null;
    private readonly availability = ui("p", "data-warning");
    private readonly preview = ui("div", "data-transfer-preview");
    private readonly error = ui("p", "data-warning");
    constructor(private readonly host: FieldTransferHost) {
        this.error.setAttribute("role", "alert"); this.error.hidden = true;
        this.availability.setAttribute("role", "status");
        this.element.addEventListener("compositionstart", () => { this.composing = true; this.version++; this.updateAvailability(); });
        this.element.addEventListener("compositionend", () => { this.composing = false; this.version++; if (this.pendingRefresh) this.refresh(); else this.updateAvailability(); });
    }
    retire(): void {
        this.generation++; this.version++; this.context = null; this.contextKey = null;
        // Retiring selection controls releases only their own IME lifetime;
        // author inputs and the DataPanel's close guard remain with their owners.
        this.composing = false; this.pendingRefresh = false;
        this.controls.forEach(control => control.disabled = true);
        setUiText(this.availability, message("fieldTransfer.sourceChanged"));
    }
    refresh(): void {
        const context = this.host.getContext(), key = context && transferContextKey(context);
        if (this.composing && key !== this.contextKey) this.retire();
        if (this.busy || this.composing) { this.pendingRefresh = true; this.updateAvailability(); return; }
        this.pendingRefresh = false;
        if (context && key === this.contextKey) { this.context = context; this.updateAvailability(); return; }
        this.generation++; this.version++; this.context = context; this.contextKey = key; this.controls = []; this.apply = null;
        replaceUiChildren(this.element, ui("h3", "", message("fieldTransfer.title")), ui("p", "data-help", message("fieldTransfer.help")), this.availability);
        const lists = context ? transferLists(context) : [], sources = lists.filter(field => field.cell.value.value.length > 0);
        if (!context || lists.length < 2 || !sources.length) { setUiText(this.availability, message("fieldTransfer.empty")); return; }
        this.inputs = { fromField: sources[0].fieldId, toField: lists.find(field => field.fieldId !== sources[0].fieldId)!.fieldId, first: "1", last: "1", position: "1" };
        const from = ui("select", "data-input"), to = ui("select", "data-input");
        const renderedGeneration = this.generation;
        const liveControl = (control: HTMLElement) => {
            const actual = this.host.getContext();
            return control.isConnected && this.element.isConnected && this.generation === renderedGeneration
                && this.contextKey === key && !!this.context && transferContextKey(this.context) === key && this.host.isCurrent(this.context)
                && !!actual && transferContextKey(actual) === key;
        };
        for (const field of lists) for (const [select, allowed] of [[from, field.cell.value.value.length > 0], [to, true]] as const) {
            if (!allowed) continue;
            const option = ui("option", "", displayMessage("fieldTransfer.fieldChoice", { p0: uiDisplayParameter(fieldNames[field.key] ?? field.key), p1: field.cell.value.value.length }));
            option.value = field.fieldId; select.append(option);
        }
        from.value = this.inputs.fromField; to.value = this.inputs.toField;
        from.dataset.transferControl = "fromField"; to.dataset.transferControl = "toField";
        const change = () => { this.version++; this.error.hidden = true; this.showPreview(); this.updateAvailability(); };
        from.onchange = () => { if (!liveControl(from)) return; this.inputs.fromField = from.value; this.inputs.first = this.inputs.last = "1"; first.value = last.value = "1"; change(); };
        to.onchange = () => { if (!liveControl(to)) return; this.inputs.toField = to.value; change(); };
        const number = (key: "first" | "last" | "position", label: Parameters<typeof labelFor>[0]) => {
            const input = ui("input", "data-input"); input.type = "text"; input.inputMode = "numeric"; input.value = this.inputs[key]; input.dataset.transferControl = key;
            input.oninput = () => { if (!liveControl(input)) return; this.inputs[key] = input.value; change(); };
            this.controls.push(input); this.element.append(labelFor(label, input), input); return input;
        };
        this.element.append(labelFor(message("fieldTransfer.fromField"), from), from, labelFor(message("fieldTransfer.toField"), to), to);
        const first = number("first", message("fieldTransfer.first")), last = number("last", message("fieldTransfer.last"));
        number("position", message("fieldTransfer.position"));
        this.apply = action(message("fieldTransfer.apply"), () => this.submit()); this.apply.dataset.transferControl = "apply";
        this.controls.push(from, to, this.apply); this.element.append(this.preview, this.error, this.apply);
        this.showPreview(); this.updateAvailability();
    }
    private updateAvailability(): void {
        const context = this.context, actual = this.host.getContext(), current = context && actual && this.contextKey === transferContextKey(context)
            && this.contextKey === transferContextKey(actual) && this.host.isCurrent(context);
        const allowed = !!current && this.host.canTransfer() && !this.composing;
        this.controls.forEach(control => control.disabled = this.busy || !allowed);
        this.element.setAttribute("aria-busy", String(this.busy));
        setUiText(this.availability, allowed ? "" : context ? current ? message("fieldTransfer.unapplied") : message("fieldTransfer.sourceChanged") : message("fieldTransfer.empty"));
    }
    private showPreview(): void {
        const context = this.context;
        if (!context) return;
        try {
            const selection = fieldTransferSelection(context, this.inputs), source = transferLists(context).find(field => field.fieldId === selection.fromField)!;
            replaceUiChildren(this.preview, ui("p", "data-help", displayMessage("fieldTransfer.preview", { p0: selection.start + 1, p1: selection.end + 1, p2: selection.to + 1 })));
            for (let ordinal = selection.start; ordinal <= selection.end; ordinal++) this.preview.append(ui("p", "data-help", displayMessage("fieldTransfer.item", { p0: ordinal + 1, p1: uiDisplayParameter(formatNativeValue(source.cell.value.value[ordinal])) })));
        } catch (error) { replaceUiChildren(this.preview, ui("p", "data-help", errorUiText(error))); }
    }
    private async submit(): Promise<void> {
        if (this.busy || this.composing || !this.context || !this.element.isConnected) return;
        const context = this.context, key = this.contextKey, version = this.version, generation = this.generation;
        const current = () => {
            const actual = this.host.getContext();
            return this.element.isConnected && !this.composing && version === this.version && generation === this.generation
                && !!key && this.contextKey === key && transferContextKey(context) === key && this.host.isCurrent(context) && this.host.canTransfer()
                && !!actual && transferContextKey(actual) === key;
        };
        try {
            if (!current()) throw new UiError(message("fieldTransfer.sourceChanged"));
            const selection = fieldTransferSelection(context, this.inputs), basis = { ...context.basis };
            this.busy = true; this.error.hidden = true; this.updateAvailability();
            await this.host.run(async () => {
                const sourceHash = await fieldTransferSourceHash(basis.source);
                if (!current()) throw new UiError(message("fieldTransfer.sourceChanged"));
                const wire = await this.host.request<unknown>("/api/data/transfer", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, sourceHash,
                    schemaRevision: context.schemaRead.revision, semanticDigest: context.schemaRead.semanticDigest, ...selection });
                if (!current()) throw new UiError(message("fieldTransfer.sourceChanged"));
                const patch = decodeFieldTransferPatch(wire, context, selection, sourceHash);
                if (!current()) throw new UiError(message("fieldTransfer.sourceChanged"));
                // Delivery is one native Draft transaction against the old input.
                await this.host.applyPatch(patch, basis);
                this.host.notify(message("fieldTransfer.applied"));
            });
        } catch (error) {
            setUiText(this.error, errorUiText(error)); this.error.hidden = false; this.host.notify(errorUiText(error), true);
        } finally { this.busy = false; this.updateAvailability(); if (this.pendingRefresh) this.refresh(); }
    }
}
