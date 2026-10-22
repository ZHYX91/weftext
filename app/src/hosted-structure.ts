import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, nativeFormControls, emptyNativeValue, fieldsForTypes, formatNativeValue, typeConflicts, ui, valueEditor } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { canCommitStructure, clearStructureCapture, decodeStructureCatalog, decodeStructureDraft, freezeStructureRequest, initialStructureDraft, structureEnded, structureInput, structureReceipt, structureSelectors, type StructureCatalog, type StructureDraft, type StructureKind, type StructureReceipt, type StructureRequest } from "./hosted-structure-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedStructureHost {
    protect(request: StructureRequest, phase: "prepare" | "commit"): Promise<void>;
    changed(receipt: StructureReceipt, request: StructureRequest): void | Promise<void>;
    open(result: NonNullable<StructureReceipt["result"]>, request: StructureRequest): Promise<void>;
}
const empty = (): StructureCatalog => ({ sources: [], slots: [], templates: [], scope: "configuredAuthorizedStructure", coverage: "complete" });
const labels: [
    StructureKind,
    UiText
][] = [["create", uiMessage("hostedStructure.createKind")], ["rename", uiMessage("tagNavigation.rename")], ["move", uiMessage("hostedStructure.moveKind")], ["trash", uiMessage("hostedStructure.trashKind")], ["restore", uiMessage("hostedStructure.restoreKind")], ["copy", uiMessage("hostedStructure.copyKind")], ["template", uiMessage("hostedStructure.templateKind")]];
export class HostedStructure {
    private epoch = 0;
    private catalogVersion = 0;
    private key = "";
    private draft: StructureDraft = initialStructureDraft();
    private catalog = empty();
    private busy = false;
    private unreadable?: string;
    private kind = ui("select");
    private source = ui("select");
    private destination = ui("select");
    private template = ui("select");
    private trashId = ui("input");
    private title = ui("input");
    private body = ui("textarea");
    private bodyMode = ui("select");
    private parameters = ui("textarea");
    private inputs = ui("select");
    private form = ui("section");
    private capture = ui("section");
    private captureKey = "";
    private fieldsForm?: HTMLFieldSetElement;
    private preview = ui("section");
    private message = ui("p", "hint");
    private coverage = ui("p", "hint");
    private prepareButton = action(uiMessage("hostedStructure.prepareOriginal"), () => this.run(() => this.prepare()));
    constructor(private host: HTMLElement, private request: Request, private bridge: HostedStructureHost) { this.clear(false); }
    get hasPending() { return !!this.draft.pending && !structureEnded(this.draft.pending.receipt?.status); }
    clear(removeCache = true) {
        this.epoch++;
        this.catalogVersion++;
        this.key = "";
        this.draft = initialStructureDraft();
        this.catalog = empty();
        this.busy = false;
        this.unreadable = undefined;
        this.host.replaceChildren();
        this.host.hidden = true;
        this.form.replaceChildren();
        this.capture.replaceChildren();
        this.captureKey = "";
        this.fieldsForm = undefined;
        this.preview.replaceChildren();
        setUiText(this.message, "");
        setUiText(this.coverage, "");
        for (const control of [this.kind, this.source, this.destination, this.template, this.trashId, this.title, this.body, this.bodyMode, this.parameters, this.inputs]) {
            control.value = "";
            control.disabled = false;
        }
        this.inputs.replaceChildren();
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-structure:"))
                    sessionStorage.removeItem(key);
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        this.key = hostedCachePrefix("structure", deployment, actor, generation) + "state";
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw)
                this.draft = decodeStructureDraft(raw);
        }
        catch {
            this.unreadable = raw ?? "";
        }
        this.host.hidden = false;
        setUiAttr(this.coverage, "aria-label", uiMessage("hostedStructure.coverageLabel"));
        const field = (label: UiText, control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement) => { setUiAttr(control, "aria-label", label); const wrapper = ui("label"); wrapper.append(ui("span", "", label), control); return wrapper; };
        this.kind.replaceChildren(...labels.map(([kind, label]) => { const option = ui("option", "", label); option.value = kind; return option; }));
        this.bodyMode.replaceChildren(...([["text", uiMessage("hostedStructure.plainTextMode")], ["markup", uiMessage("hostedStructure.typstMarkupMode")]] as [
            string,
            UiText
        ][]).map(([value, text]) => { const option = ui("option", "", text); option.value = value; return option; }));
        this.body.rows = 5;
        this.parameters.rows = 4;
        this.body.spellcheck = this.parameters.spellcheck = false;
        this.inputs.multiple = true;
        this.form.append(field(uiMessage("hostedStructure.kindLabel"), this.kind), field(uiMessage("hostedStructure.sourceLabel"), this.source), field(uiMessage("hostedStructure.destinationLabel"), this.destination), field(uiMessage("hostedStructure.templateLabel"), this.template), field(uiMessage("hostedStructure.trashOperationLabel"), this.trashId), field(uiMessage("hostedStructure.titleLabel"), this.title), field(uiMessage("hostedStructure.bodyLabel"), this.body), field(uiMessage("hostedStructure.bodyModeLabel"), this.bodyMode), this.capture, field(uiMessage("hostedStructure.parametersLabel"), this.parameters), field(uiMessage("hostedStructure.inputsLabel"), this.inputs), this.prepareButton);
        this.kind.onchange = () => { this.draft.kind = this.kind.value as StructureKind; this.inputChanged(); };
        this.source.onchange = () => { this.draft.source = this.source.value; this.inputChanged(); };
        this.destination.onchange = () => { this.draft.destination = this.destination.value; this.inputChanged(); };
        this.template.onchange = () => { this.draft.template = this.template.value; this.inputChanged(); };
        this.trashId.oninput = () => { this.draft.trashOperationId = this.trashId.value; this.inputChanged(); };
        this.title.oninput = () => { this.draft.title = this.title.value; this.inputChanged(); };
        this.body.oninput = () => { this.draft.body = this.body.value; this.inputChanged(); };
        this.bodyMode.onchange = () => { this.draft.bodyMode = this.bodyMode.value as "text" | "markup"; this.inputChanged(); };
        this.parameters.oninput = () => { this.draft.parametersText = this.parameters.value; this.inputChanged(); };
        this.inputs.onchange = () => { this.draft.inputs = [...this.inputs.selectedOptions].map(option => option.value); this.inputChanged(); };
        this.host.replaceChildren(ui("h2", "", uiMessage("hostedStructure.title")), ui("p", "hint", uiMessage("hostedStructure.authorizationHelp")), action(uiMessage("hostedStructure.refreshCatalog"), () => this.refresh().catch(error => this.show(error))), this.coverage, this.form, this.message, this.preview);
        this.render();
        await this.refresh();
    }
    async refresh() {
        const epoch = this.epoch, version = ++this.catalogVersion, value = decodeStructureCatalog(await this.request<StructureCatalog>("structure"));
        if (epoch !== this.epoch || version !== this.catalogVersion)
            return;
        this.catalog = value;
        this.options(this.source, value.sources.map(item => [item.resource, () => `${uiTextValue(item.title)}${uiTextValue(item.status === "unavailable" ? uiMessage("hostedStructure.restoreReceiptRequiredSuffix") : "")}`]), this.draft.source);
        this.options(this.destination, value.slots.map(item => [item.resource, () => `${uiTextValue(item.title)} · ${uiTextValue(item.parentTitle ?? item.parent)} / ${uiTextValue(item.name)}${uiTextValue(item.status === "unavailable" ? uiMessage("hostedStructure.unavailableSuffix") : "")}`]), this.draft.destination);
        this.options(this.template, value.templates.map(item => [item.id, () => `${uiTextValue(item.title)}${uiTextValue(item.status === "unavailable" ? uiMessage("hostedStructure.unavailableSuffix") : "")}`]), this.draft.template);
        setUiText(this.coverage, value.coverage === "partial" ? uiMessage("hostedStructure.partialCatalog") : uiMessage("hostedStructure.completeCatalog"));
        this.render();
    }
    private options(select: HTMLSelectElement, values: [
        string,
        UiText
    ][], current: string) {
        const first = ui("option", "", uiMessage("hostedStructure.selectPlaceholder"));
        first.value = "";
        select.replaceChildren(first, ...values.map(([value, label]) => { const option = ui("option", "", label); option.value = value; return option; }));
        if (current && !values.some(([value]) => value === current)) {
            const old = ui("option", "", displayMessage("hostedStructure.retainedSelection", { p0: uiDisplayParameter(current) }));
            old.value = current;
            select.append(old);
        }
        select.value = current;
    }
    setDocuments(documents: {
        resource: string;
        title: string;
    }[]) {
        this.inputs.replaceChildren(...documents.map(item => { const option = ui("option", "", item.title); option.value = item.resource; option.selected = this.draft.inputs.includes(item.resource); return option; }));
        for (const id of this.draft.inputs.filter(id => !documents.some(item => item.resource === id))) {
            const old = ui("option", "", displayMessage("hostedStructure.retainedInput", { p0: uiDisplayParameter(id) }));
            old.value = id;
            old.selected = true;
            this.inputs.append(old);
        }
    }
    private persist(value = this.draft) {
        if (!this.key || this.unreadable !== undefined)
            throw new UiError(uiMessage("hostedStructure.recoveryReadOnly"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(value));
        }
        catch {
            throw new UiError(uiMessage("hostedStructure.recoveryPersistenceFailed"));
        }
    }
    private keep() {
        try {
            this.persist();
        }
        catch (error) {
            this.show(error);
        }
    }
    private show(error: unknown) { setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("hostedStructure.resultUnconfirmed")); }
    private inputChanged() { this.captureRawInputs(); this.draft.revision++; this.keep(); this.render(); }
    private fieldControls() { return this.fieldsForm ? nativeFormControls(this.fieldsForm).map(([key, control]) => ({ key, control })) : []; }
    private captureRawInputs() {
        for (const { key, control } of this.fieldControls()) {
            if (control.dataset.unmatchedRaw)
                continue;
            if (!control.validity.valid)
                this.draft.rawInputs[key] = control.value;
            else
                delete this.draft.rawInputs[key];
        }
    }
    private renderCapture() {
        const d = this.draft, slot = this.catalog.slots.find(item => item.resource === d.destination), key = JSON.stringify([d.kind, d.destination, slot?.capture, slot?.captureStatus, d.types, Object.keys(d.fields)]);
        this.capture.hidden = d.kind !== "create";
        if (this.captureKey === key)
            return;
        this.captureRawInputs();
        this.captureKey = key;
        this.fieldsForm = undefined;
        this.capture.replaceChildren();
        if (d.kind !== "create")
            return;
        const schema = slot?.capture;
        this.capture.append(ui("h3", "", uiMessage("hostedStructure.captureTitle")));
        if (d.types.length || Object.keys(d.fields).length || Object.keys(d.rawInputs).length)
            this.capture.append(action(uiMessage("hostedStructure.clearCapture"), () => { this.fieldsForm = undefined; this.captureKey = ""; this.draft = clearStructureCapture(this.draft); this.keep(); this.render(); }));
        if (!schema || slot.captureStatus === "unavailable") {
            this.capture.append(ui("p", "hint", uiMessage("hostedStructure.captureUnavailable")));
            for (const [id, value] of Object.entries(d.fields))
                this.capture.append(ui("p", "", () => `${uiTextValue(id)}：${uiTextValue(formatNativeValue(value))}`));
        }
        else {
            const types = ui("select");
            types.multiple = true;
            setUiAttr(types, "aria-label", uiMessage("hostedStructure.captureTypesLabel"));
            for (const id of slot.types ?? []) {
                const option = ui("option", "", id);
                option.value = id;
                option.selected = d.types.includes(id);
                types.append(option);
            }
            for (const id of d.types.filter(id => !(slot.types ?? []).includes(id))) {
                const option = ui("option", "", displayMessage("hostedStructure.retainedType", { p0: uiDisplayParameter(id) }));
                option.value = id;
                option.selected = true;
                types.append(option);
            }
            types.onchange = () => { d.types = [...types.selectedOptions].map(option => option.value); this.inputChanged(); };
            this.capture.append(types);
            for (const conflict of typeConflicts(schema, d.types))
                this.capture.append(ui("p", "hint", conflict));
            const form = ui("fieldset");
            this.fieldsForm = form;
            this.capture.append(form);
            for (const { field, required } of fieldsForTypes(schema, d.types)) {
                const row = ui("section");
                row.dataset.fieldShape = JSON.stringify(field.shape);
                row.append(ui("p", "", () => `${uiTextValue(field.id)}${uiTextValue(required ? uiMessage("hostedStructure.requiredSuffix") : "")}`));
                const value = d.fields[field.id];
                if (value) {
                    row.append(valueEditor(field.shape, value, schema, next => { d.fields[field.id] = next; this.inputChanged(); }, displayMessage("hostedStructure.captureFieldLabel", { p0: uiDisplayParameter(field.id) }), { fieldId: field.id, path: [] }));
                    if (field.nullable)
                        row.append(action(displayMessage("hostedStructure.setFieldNone", { p0: uiDisplayParameter(field.id) }), () => { d.fields[field.id] = { kind: "none" }; this.captureKey = ""; this.inputChanged(); }));
                    row.append(action(displayMessage("hostedStructure.removeField", { p0: uiDisplayParameter(field.id) }), () => { delete d.fields[field.id]; this.inputChanged(); }));
                }
                else
                    row.append(action(displayMessage("hostedStructure.fillField", { p0: uiDisplayParameter(field.id) }), () => { d.fields[field.id] = emptyNativeValue(field.shape, schema); this.inputChanged(); }));
                form.append(row);
            }
            for (const [id, value] of Object.entries(d.fields).filter(([id]) => !fieldsForTypes(schema, d.types).some(item => item.field.id === id)))
                this.capture.append(ui("p", "hint", displayMessage("hostedStructure.unmatchedField", { p0: uiDisplayParameter(id), p1: uiDisplayParameter(formatNativeValue(value)) })), action(displayMessage("hostedStructure.discardField", { p0: uiDisplayParameter(id) }), () => { delete d.fields[id]; this.inputChanged(); }));
            for (const { key, control } of this.fieldControls())
                if (d.rawInputs[key] !== undefined) {
                    const raw = d.rawInputs[key];
                    control.value = raw;
                    if (control.value !== raw) {
                        control.dataset.unmatchedRaw = "true";
                        setUiValidity(control, uiMessage("hostedStructure.rawInputUnrepresentable"));
                    }
                    else
                        control.dispatchEvent(new Event("input", { bubbles: false }));
                }
            const edited = (event: Event) => {
                if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || event.target instanceof HTMLTextAreaElement) {
                    if (event.target.dataset.unmatchedRaw) {
                        delete event.target.dataset.unmatchedRaw;
                        setUiValidity(event.target, "");
                    }
                }
                this.captureRawInputs();
                this.keep();
            };
            form.addEventListener("input", edited);
            form.addEventListener("change", edited);
            form.addEventListener("click", event => {
                if (!(event.target instanceof Element) || !event.target.closest("button"))
                    return;
                const invalid = this.fieldControls().find(item => !item.control.validity.valid);
                if (invalid) {
                    event.preventDefault();
                    event.stopPropagation();
                    invalid.control.reportValidity();
                    setUiText(this.message, uiMessage("hostedStructure.inputIncomplete"));
                }
            }, { capture: true });
        }
        const matched = new Set(this.fieldControls().filter(item => !item.control.dataset.unmatchedRaw).map(item => item.key)), unmapped = Object.entries(d.rawInputs).filter(([key]) => !matched.has(key));
        if (unmapped.length)
            this.capture.append(ui("p", "hint", uiMessage("hostedStructure.unmatchedRawInputHelp")), ...unmapped.map(([, text]) => ui("pre", "", text)), action(uiMessage("hostedStructure.discardUnmatchedRaw"), () => {
                for (const [key] of unmapped)
                    delete d.rawInputs[key];
                this.inputChanged();
            }));
    }
    private async run(work: () => Promise<void>) {
        if (this.busy)
            return;
        const epoch = this.epoch;
        this.busy = true;
        this.render();
        try {
            await work();
        }
        catch (error) {
            if (epoch === this.epoch)
                this.show(error);
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async prepare() {
        this.captureRawInputs();
        if (this.fieldControls().some(({ control }) => !control.reportValidity())) {
            this.keep();
            throw new UiError(uiMessage("hostedStructure.invalidInputBeforePrepare"));
        }
        const slot = this.catalog.slots.find(item => item.resource === this.draft.destination);
        if (this.draft.kind === "create" && slot?.capture) {
            if (typeConflicts(slot.capture, this.draft.types).length || fieldsForTypes(slot.capture, this.draft.types).some(item => item.required && !this.draft.fields[item.field.id]))
                throw new UiError(uiMessage("hostedStructure.requiredFieldsOrConflicts"));
        }
        const epoch = this.epoch, request = freezeStructureRequest(this.draft, this.catalog, crypto.randomUUID()), next = { ...this.draft, pending: { request, input: structureInput(this.draft), revision: this.draft.revision, selector: structureSelectors(this.draft, this.catalog), acceptLosses: false } };
        this.persist(next);
        this.draft = next;
        this.render();
        await this.bridge.protect(request, "prepare");
        if (epoch !== this.epoch)
            return;
        const value = await this.request<StructureReceipt>("structure/prepare", request);
        if (epoch === this.epoch)
            await this.receive(value);
    }
    private async receive(value: StructureReceipt) {
        const pending = this.draft.pending;
        if (!pending)
            return;
        pending.receipt = structureReceipt(pending.request, value, pending.receipt);
        this.keep();
        setUiText(this.message, value.status === "committed" ? uiMessage("hostedStructure.originalCommitted") : value.status === "prepared" ? uiMessage("hostedStructure.originalPrepared") : structureEnded(value.status) ? uiMessage("hostedStructure.originalEnded") : uiMessage("hostedStructure.originalPending"));
        if (value.status === "committed") {
            const epoch = this.epoch;
            await this.bridge.changed(value, pending.request);
            if (epoch === this.epoch)
                await this.refresh();
        }
    }
    private render() {
        this.renderCapture();
        const d = this.draft;
        for (const [control, value] of [[this.kind, d.kind], [this.source, d.source], [this.destination, d.destination], [this.template, d.template], [this.trashId, d.trashOperationId], [this.title, d.title], [this.body, d.body], [this.bodyMode, d.bodyMode], [this.parameters, d.parametersText]] as const)
            if (control.value !== value)
                control.value = value;
        for (const control of [this.source, this.destination, this.template, this.trashId, this.title, this.body, this.bodyMode, this.parameters])
            control.parentElement!.hidden = control === this.source ? d.kind === "create" || d.kind === "template" && d.template === "native-note" : control === this.destination ? d.kind === "trash" : control === this.template || control === this.parameters ? d.kind !== "template" : control === this.trashId ? d.kind !== "restore" : d.kind !== "create";
        this.prepareButton.disabled = this.busy || !!d.pending || this.unreadable !== undefined;
        this.preview.replaceChildren();
        if (this.unreadable !== undefined) {
            this.form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input,select,textarea").forEach(control => control.disabled = true);
            this.preview.append(ui("p", "hint", uiMessage("hostedStructure.unreadableRecovery")), ui("pre", "", this.unreadable), action(uiMessage("hostedStructure.discardRecovery"), () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initialStructureDraft(); this.form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input,select,textarea").forEach(control => control.disabled = false); this.render(); }));
            return;
        }
        const pending = d.pending;
        if (!pending)
            return;
        const receipt = pending.receipt, card = ui("section");
        card.append(ui("h3", "", uiMessage("hostedStructure.originalTitle")), ui("p", "", `${pending.request.kind} · ${pending.request.resource} · ${pending.request.operationId}`), ui("p", "", displayMessage("hostedStructure.operationStatus", { p0: uiDisplayParameter(receipt?.status ?? "unknown") })));
        const summary = receipt?.preview?.summary;
        if (summary)
            card.append(ui("pre", "", Array.isArray(summary) ? summary.join("\n") : summary));
        for (const loss of receipt?.preview?.losses ?? [])
            card.append(ui("p", "hint", () => `${uiTextValue(loss.code ?? uiMessage("hostedStructure.lossFallbackTitle"))}：${uiTextValue(loss.message)}`));
        const accepted = ui("input");
        accepted.type = "checkbox";
        accepted.checked = pending.commit?.acceptLosses ?? pending.acceptLosses;
        accepted.disabled = !!pending.commit;
        setUiAttr(accepted, "aria-label", uiMessage("hostedStructure.acceptLossesLabel"));
        accepted.onchange = () => { pending.acceptLosses = accepted.checked; this.keep(); };
        const lossChoice = ui("label");
        lossChoice.append(accepted, ui("span", "", uiMessage("hostedStructure.acceptLossesLabel")));
        card.append(lossChoice);
        if (pending.revision !== d.revision || pending.input !== structureInput(d))
            card.append(ui("p", "hint", uiMessage("hostedStructure.inputIntentChanged")));
        const recover = action(uiMessage("hostedStructure.inspectOriginal"), () => this.run(async () => {
            const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/operation", { resource: pending.request.resource, operationId: pending.request.operationId });
            if (epoch === this.epoch)
                await this.receive(value);
        }));
        recover.disabled = this.busy;
        const retry = action(uiMessage("hostedStructure.retryOriginalPrepare"), () => this.run(async () => {
            this.persist();
            const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/prepare", pending.request);
            if (epoch === this.epoch)
                await this.receive(value);
        }));
        retry.disabled = this.busy || structureEnded(receipt?.status);
        const commit = action(uiMessage("hostedStructure.commitOriginal"), () => this.run(async () => {
            if (!canCommitStructure(this.draft, this.catalog))
                throw new UiError(uiMessage("hostedStructure.commitEligibilityChanged"));
            if (receipt?.preview?.losses?.length && !pending.acceptLosses)
                throw new UiError(uiMessage("hostedStructure.acceptLossesRequired"));
            const epoch = this.epoch;
            pending.commit ??= { resource: pending.request.resource, operationId: pending.request.operationId, planId: pending.receipt!.planId!, acceptLosses: pending.acceptLosses };
            this.persist();
            await this.bridge.protect(pending.request, "commit");
            if (epoch !== this.epoch)
                return;
            if (!canCommitStructure(this.draft, this.catalog))
                throw new UiError(uiMessage("hostedStructure.protectBasisChanged"));
            const value = await this.request<StructureReceipt>("structure/commit", pending.commit);
            if (epoch === this.epoch)
                await this.receive(value);
        }));
        commit.disabled = this.busy || !canCommitStructure(d, this.catalog);
        const settle = action(uiMessage("hostedStructure.settleOriginal"), () => this.run(async () => {
            const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/settle", { resource: pending.request.resource, operationId: pending.request.operationId });
            if (epoch === this.epoch)
                await this.receive(value);
        }));
        settle.disabled = this.busy || structureEnded(receipt?.status);
        card.append(commit, recover, retry, settle);
        if (receipt?.status === "committed") {
            card.append(ui("pre", "", (receipt.nodeRefs ?? []).map(ref => `weftext-node://${ref.workspaceId}/${ref.nodeId}`).join("\n")));
            if (receipt.result?.canOpen) {
                const result = receipt.result;
                card.append(action(uiMessage("hostedStructure.openAuthorizedResult"), () => this.run(() => this.bridge.open(result, pending.request))));
            }
            else
                card.append(ui("p", "hint", uiMessage("hostedStructure.committedOpenUnavailable")));
        }
        if (structureEnded(receipt?.status))
            card.append(action(uiMessage("hostedStructure.clearEndedReceipt"), () => { const next = { ...d, pending: undefined }; this.persist(next); this.draft = next; this.render(); }));
        this.preview.append(card);
    }
}
