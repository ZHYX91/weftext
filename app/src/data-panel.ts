import { reminderForm, type ReminderRawInput } from "./reminder-form";
import { nativeControlAddress } from "./native-control-identity";
import { newReminder } from "./reminder-values";
import { ReminderDeviceControl } from "./reminder-device-control";
import { ProposalCloseGuard } from "./proposal-close-guard";
import type { OpenedDocument } from "./types";
import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody } from "./api-error";
export * from "./native-value-form";
import { uiDisplayParameter, message as uiMessage, displayMessage, setUiText, setUiAttr, uiTextValue, joinUiText, errorUiText, UiError, type UiText } from "./ui-locale";
import { type Cell, type NoteProjection } from "./data-projection";
import { readNoteProjection, sameProjectionInput } from "./data-projection-read";
export { decodeProjection, decodeCell, type Cell, type FieldProjection, type NoteProjection } from "./data-projection";
import { descriptionUiText, coreDescriptionUiText } from "./ui-diagnostics";
import { submitOriginal, TerminatedAttempt, type OriginalFact } from "./original-operation";
import { action, ui, labelFor, emptyReferences, loadFormReferences, fieldNames, typeNames, cellLabels, commonFields, fieldsForTypes, typeConflicts, resolveShape, emptyNativeValue, nativeFormControls, valueEditor, decodeSourcePatch, formatNativeValue, diagnosticText, type NativeValue, type Shape, type DataSchema, type SchemaField, type SchemaRead, type DocumentBasis, type PanelHost, type SourcePatch } from "./native-value-form";
interface FieldDraft {
    value: NativeValue;
    changed: boolean;
    original?: string;
    binding?: string;
    rawInputs?: Record<string, string>;
    revision?: number;
}
interface SchemaProposal {
    source: string;
    beforeSource: string;
    beforeRevision: string | null;
    plan?: { planId: string; operationId: string };
    preview?: unknown;
}
export class DataPanel {
    readonly element = ui("aside", "data-panel");
    private schema: DataSchema | null = null;
    private projection: NoteProjection | null = null;
    private basis: DocumentBasis | null = null;
    private generation = 0;
    private drafts = new Map<string, FieldDraft>();
    private filter = "";
    private pinned = new Set<string>();
    private schemaRead: SchemaRead | null = null;
    private draftCache = new Map<string, Map<string, FieldDraft>>();
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly typeDrafts = new Map<string, string[]>();
    private readonly typeInstalling = new Set<string>();
    private readonly schemaDrafts = new Map<string, SchemaProposal>();
    private readonly schemaInstalling = new Set<string>();
    private references = emptyReferences();
    private referencesRoot = "";
    private composing = false;
    private pendingRender = false;
    private advancedOpen = false;
    private renderedIdentity: string | null = null;
    private draftIdentity: string | null = null;
    private reminderDisposers: (() => void)[] = [];
    private readonly reminderDevice: ReminderDeviceControl;
    constructor(private readonly host: PanelHost) {
        this.closeGuard.bind(this.element);
        this.reminderDevice = new ReminderDeviceControl(host);
        setUiAttr(this.element, "aria-label", uiMessage("dataPanel.regionLabel"));
        this.element.append(ui("p", "data-empty", uiMessage("dataPanel.openNoteHelp")));
        // Invalid raw text never reaches the typed changed callback. Record it
        // on its owning input event; close proof reads only the retained owners.
        for (const event of ["input", "change"]) this.element.addEventListener(event, () => {
            if (!this.closeGuard.frozen) this.rememberInputs();
        });
        this.element.addEventListener("compositionstart", () => { this.composing = true; });
        this.element.addEventListener("compositionend", () => {
            this.composing = false;
            requestAnimationFrame(() => {
                if (this.closeGuard.frozen) return;
                if (!this.composing && this.pendingRender) {
                    this.pendingRender = false;
                    this.render();
                }
            });
        });
    }
    private hasUnresolved(): boolean {
        return this.typeDrafts.size > 0 || this.schemaDrafts.size > 0 ||
            [this.drafts, ...this.draftCache.values()].some(drafts => [...drafts.values()].some(draft => draft.changed || !!Object.keys(draft.rawInputs ?? {}).length));
    }
    private discardAllButton(afterDiscard?: () => void): HTMLButtonElement {
        return action(uiMessage("dataPanel.discardAllFormChanges"), () => {
            try { this.closeGuard.assertReady(false); }
            catch (error) { this.host.notify(errorUiText(error), true); return; }
            this.drafts.clear(); this.draftCache.clear(); this.typeDrafts.clear(); this.schemaDrafts.clear();
            if (afterDiscard) afterDiscard();
            else if (this.basis) this.render();
            else {
                this.element.replaceChildren(ui("p", "data-empty", uiMessage("dataPanel.openNoteHelp")));
                if (this.reminderDevice.hasPending()) this.element.append(this.reminderDevice.element);
            }
        }, "data-button compact");
    }
    assertCloseReady(): void { this.closeGuard.assertReady(this.hasUnresolved()); this.reminderDevice.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        if (!this.closeGuard.freeze(this.hasUnresolved())) return false;
        try { if (this.reminderDevice.freezeForClose()) return true; }
        catch { this.closeGuard.thaw(); return false; }
        this.closeGuard.thaw(); return false;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); this.reminderDevice.thawAfterClose(); }
    async refresh(): Promise<void> {
        if (this.closeGuard.frozen) return;
        return this.closeGuard.run(() => this.refreshOwned());
    }
    private async refreshOwned(): Promise<void> {
        this.rememberInputs();
        const selected = this.host.getDocument();
        const basis = selected && { ...selected };
        const generation = ++this.generation;
        const current = () => generation === this.generation && !!basis && sameProjectionInput(this.host.getDocument(), basis);
        const identity = (doc: DocumentBasis) => JSON.stringify([doc.workspaceId, doc.nodeId ?? doc.path]);
        if (!basis) {
            if (this.basis)
                this.draftCache.set(identity(this.basis), this.drafts);
            this.basis = null;
            this.projection = null;
            this.draftIdentity = this.renderedIdentity = null;
            this.reminderDisposers.splice(0).forEach(dispose => dispose());
            this.reminderDevice.retire();
            this.element.replaceChildren(ui("p", "data-empty", uiMessage("dataPanel.openNoteHelp")));
            if (this.hasUnresolved()) this.element.append(this.discardAllButton());
            if (this.reminderDevice.hasPending()) this.element.append(this.reminderDevice.element);
            return;
        }
        if (!this.basis || identity(this.basis) !== identity(basis)) {
            if (this.basis)
                this.draftCache.set(identity(this.basis), this.drafts);
            this.drafts = this.draftCache.get(identity(basis)) ?? new Map();
        }
        this.draftIdentity = identity(basis);
        this.basis = { ...basis };
        this.element.setAttribute("aria-busy", "true");
        try {
            const schemaResult = await this.host.request<SchemaRead>("/api/data/schema", { root: basis.root });
            if (!current()) return;
            const projection = await readNoteProjection(this.host.request.bind(this.host), basis, schemaResult);
            if (!current())
                return;
            this.schema = schemaResult.schema;
            this.projection = { ...projection, readOnly: projection.readOnly || this.host.canEdit?.() === false };
            this.schemaRead = schemaResult;
            try {
                this.pinned = new Set(JSON.parse(localStorage.getItem(`weftext-fields:${basis.workspaceId ?? basis.root}`) ?? "[]") as string[]);
            }
            catch {
                this.pinned.clear();
            }
            if (this.referencesRoot !== basis.root) {
                this.referencesRoot = basis.root;
                this.references = emptyReferences();
            }
            this.render();
            if (basis.workspaceId)
                await loadFormReferences(this.host, basis.root, this.references, this.element, () => !this.closeGuard.frozen && generation === this.generation && this.referencesRoot === basis.root);
        }
        catch (error) {
            if (current()) {
                const discard = this.discardAllButton(() => discard.remove());
                this.element.replaceChildren(ui("h2", "", uiMessage("dataPanel.title")), ui("p", "data-warning", error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse")), action(uiMessage("dataPanel.retryRead"), () => this.refresh()), discard);
                this.reminderDevice.retire();
                if (this.reminderDevice.hasPending()) this.element.append(this.reminderDevice.element);
            }
        }
        finally {
            if (generation === this.generation)
                this.element.removeAttribute("aria-busy");
        }
    }
    private render(): void {
        if (this.closeGuard.frozen) return;
        if (this.composing) {
            this.pendingRender = true;
            return;
        }
        if (!this.schema || !this.projection || !this.basis)
            return;
        const schema = this.schema;
        const projection = this.projection;
        const active = this.element.contains(document.activeElement) ? document.activeElement as HTMLElement : null, fieldKey = active?.closest<HTMLElement>("[data-field-key]")?.dataset.fieldKey, controlKey = active?.dataset.controlKey, pinFocused = active?.matches("button[aria-pressed]");
        const selection = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement && active.type === "text" ? { from: active.selectionStart, to: active.selectionEnd } : null;
        const advanced = this.element.querySelector<HTMLDetailsElement>(".data-field-group.advanced");
        if (advanced)
            this.advancedOpen = advanced.open;
        const oldFocus = this.element.contains(document.activeElement) ? (document.activeElement as HTMLInputElement).id : "";
        this.rememberInputs();
        this.element.replaceChildren();
        const heading = ui("div", "data-panel-heading");
        heading.append(ui("h2", "", uiMessage("dataPanel.title")), action(uiMessage("common.refresh"), () => this.refresh(), "data-button compact"));
        heading.append(this.discardAllButton());
        this.element.append(heading);
        const status = ui("p", projection.qualification === "qualified" || projection.qualification === "certified" ? "data-help" : "data-warning", displayMessage("dataPanel.qualification", { p0: uiDisplayParameter(projection.qualification) }));
        this.element.append(status);
        this.renderTypes(schema, projection.types);
        this.renderSchemaSettings();
        const filter = ui("input", "data-input");
        filter.type = "search";
        filter.value = this.filter;
        setUiAttr(filter, "placeholder", uiMessage("dataPanel.filterPlaceholder"));
        filter.id = "property-filter";
        setUiAttr(filter, "aria-label", uiMessage("dataPanel.filterLabel"));
        filter.oninput = () => { this.filter = filter.value; this.renderFields(); };
        this.element.append(filter);
        this.element.append(ui("div", "data-fields"));
        this.renderFields();
        this.reminderDevice.retire();
        if (this.basis && fieldsForTypes(schema, projection.types).some(item => item.field.id === "tasks/reminder")) {
            this.element.append(this.reminderDevice.element);
            void this.closeGuard.run(() => this.reminderDevice.refresh(this.basis!));
        }
        else if (this.reminderDevice.hasPending()) this.element.append(this.reminderDevice.element);
        if (projection.unknownFields.length) {
            const unknown = ui("details", "data-unknown-fields");
            unknown.append(ui("summary", "", displayMessage("dataPanel.unknownFieldsDisclosure", { p0: projection.unknownFields.length })));
            for (const item of projection.unknownFields) {
                unknown.append(ui("strong", "", item.key), ui("pre", "", item.raw));
            }
            this.element.append(unknown);
        }
        if (projection.diagnostics.length) {
            const diagnostics = ui("details", "data-diagnostics");
            diagnostics.append(ui("summary", "", displayMessage("dataPanel.diagnosticCount", { p0: projection.diagnostics.length })));
            projection.diagnostics.forEach(item => diagnostics.append(ui("p", "data-warning", descriptionUiText(item))));
            this.element.append(diagnostics);
        }
        this.element.append(ui("p", "data-help", uiMessage("dataPanel.draftApplyHelp")));
        if (oldFocus)
            this.element.querySelector<HTMLElement>(`#${CSS.escape(oldFocus)}`)?.focus({ preventScroll: true });
        if (fieldKey) {
            const field = this.element.querySelector<HTMLElement>(`[data-field-key="${CSS.escape(fieldKey)}"]`), target = controlKey ? field?.querySelector<HTMLElement>(`[data-control-key="${CSS.escape(controlKey)}"]`) : pinFocused ? field?.querySelector<HTMLElement>("button[aria-pressed]") : null;
            target?.focus({ preventScroll: true });
            if (selection?.from != null && selection.to != null && (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement))
                target.setSelectionRange(selection.from, selection.to);
        }
    }
    private renderTypes(schema: DataSchema, tokens: string[]): void {
        const basis = this.basis;
        const identity = this.draftIdentity!;
        const details = ui("details", "data-type-picker");
        details.append(ui("summary", "", displayMessage("dataPanel.typesDisclosure", { p0: uiDisplayParameter(tokens.length ? joinUiText(tokens.map(token => typeNames[token] ?? token), "、") : uiMessage("dataPanel.ordinaryNote")) })));
        const selected = new Set(this.typeDrafts.get(identity) ?? tokens);
        const choices = ui("div", "data-type-options");
        const aliases = schema.types.map(type => { const alias = Object.entries(schema.aliases).find(([, id]) => id === type.id)?.[0] ?? type.id; return { alias, id: type.id }; });
        for (const { alias, id } of aliases) {
            const checkbox = ui("input");
            checkbox.type = "checkbox";
            checkbox.checked = selected.has(alias) || selected.has(id);
            const label = labelFor(typeNames[alias] ?? alias, checkbox);
            label.prepend(checkbox);
            choices.append(label);
            checkbox.onchange = () => {
                selected.delete(id);
                selected.delete(alias);
                if (checkbox.checked)
                    selected.add(alias);
                const next = [...selected];
                if (!this.typeInstalling.has(identity) && JSON.stringify([...next].sort()) === JSON.stringify([...tokens].sort())) this.typeDrafts.delete(identity);
                else this.typeDrafts.set(identity, next);
                setUiText(warnings, joinUiText(typeConflicts(schema, [...selected]), "；"));
            };
        }
        const known = new Set(aliases.flatMap(item => [item.alias, item.id]));
        const unknown = tokens.filter(token => !known.has(token));
        if (unknown.length)
            choices.append(ui("p", "data-warning", displayMessage("dataPanel.unregisteredTypes", { p0: uiDisplayParameter(unknown.join("、")) })));
        const warnings = ui("p", "data-warning");
        const apply = action(uiMessage("dataPanel.applyTypes"), () => this.applyTypes([...selected], basis, identity));
        const discard = action(uiMessage("dataPanel.discardFormChanges"), () => { this.typeDrafts.delete(identity); this.render(); });
        if (this.projection?.readOnly) {
            apply.disabled = true;
            choices.querySelectorAll<HTMLInputElement>("input").forEach(input => { input.disabled = true; });
            details.append(ui("p", "data-help", uiMessage("dataPanel.readOnlyHelp")));
        }
        details.append(choices, warnings, apply, discard);
        this.element.append(details);
    }
    private renderFields(): void {
        if (this.closeGuard.frozen) return;
        if (this.composing) {
            this.pendingRender = true;
            return;
        }
        const target = this.element.querySelector<HTMLElement>(".data-fields");
        if (!target || !this.schema || !this.projection)
            return;
        this.rememberInputs();
        const oldAdvanced = target.querySelector<HTMLDetailsElement>(".advanced");
        if (oldAdvanced)
            this.advancedOpen = oldAdvanced.open;
        this.reminderDisposers.splice(0).forEach(dispose => dispose());
        target.replaceChildren();
        const schema = this.schema;
        const declared = fieldsForTypes(schema, this.projection.types);
        const ids = new Set(declared.map(item => item.field.id));
        const basis = this.basis;
        for (const item of this.projection.fields)
            if (!ids.has(item.fieldId)) {
                const field = schema.fields.find(field => field.id === item.fieldId);
                if (field)
                    declared.push({ field, required: false });
            }
        const groups = { pinned: ui("div", "data-field-group"), common: ui("div", "data-field-group"), advanced: ui("details", "data-field-group advanced") };
        groups.advanced.append(ui("summary", "", uiMessage("dataPanel.moreFields")));
        groups.advanced.open = this.advancedOpen;
        groups.advanced.ontoggle = () => {
            if (!this.closeGuard.frozen && groups.advanced.isConnected)
                this.advancedOpen = groups.advanced.open;
        };
        const matching = declared.filter(({ field }) => `${uiTextValue(field.key)} ${uiTextValue(fieldNames[field.key] ?? "")}`.toLowerCase().includes(this.filter.toLowerCase()));
        for (const { field, required } of matching) {
            const projected = this.projection.fields.find(item => item.fieldId === field.id);
            const cell: Cell = projected?.cell ?? { state: "absent" };
            const wrapper = ui("section", "data-property");
            wrapper.dataset.fieldKey = field.key;
            wrapper.dataset.fieldId = field.id;
            const title = ui("div", "data-property-title");
            title.append(ui("h3", "", () => `${uiTextValue(fieldNames[field.key] ?? field.key)}${uiTextValue(required ? " *" : "")}`));
            const pin = action(this.pinned.has(field.id) ? uiMessage("dataPanel.unpinField") : uiMessage("dataPanel.pinField"), () => {
                if (this.pinned.has(field.id))
                    this.pinned.delete(field.id);
                else
                    this.pinned.add(field.id);
                localStorage.setItem(`weftext-fields:${this.basis!.workspaceId ?? this.basis!.root}`, JSON.stringify([...this.pinned]));
                this.renderFields();
            }, "data-button compact");
            pin.setAttribute("aria-pressed", String(this.pinned.has(field.id)));
            title.append(pin);
            wrapper.append(title, ui("p", `data-cell-state ${cell.state}`, cellLabels[cell.state]));
            if (this.projection.readOnly || cell.state === "unavailable" || cell.state === "invalid" || projected?.writable === false) {
                wrapper.append(ui("p", "data-help", cell.reason ? descriptionUiText(cell.reason) : uiMessage("dataPanel.fieldWriteUnavailable")));
                if (cell.value)
                    wrapper.append(ui("p", "data-readonly-value", formatNativeValue(cell.value)));
                if (projected?.raw)
                    wrapper.append(ui("pre", "data-readonly-source", projected.raw));
                cell.diagnostics?.forEach(item => wrapper.append(ui("p", "data-warning", descriptionUiText(item))));
            }
            else if (cell.state === "absent" && !this.drafts.has(field.key))
                wrapper.append(action(uiMessage("dataPanel.fillField"), () => { this.drafts.set(field.key, { value: field.id === "tasks/reminder" ? newReminder("absolute") : emptyNativeValue(field.shape, schema), changed: true, original: JSON.stringify(cell) }); this.renderFields(); }));
            else {
                const draft: FieldDraft = this.drafts.get(field.key) ?? { value: cell.value ?? { kind: "none" } as NativeValue, changed: false, original: JSON.stringify(cell) };
                const binding = JSON.stringify([field.id, field.shape, schema.definitions]);
                draft.binding ??= binding;
                const schemaChanged = draft.binding !== binding;
                if (!draft.changed) {
                    draft.value = cell.value ?? { kind: "none" };
                    draft.original = JSON.stringify(cell);
                    draft.binding = binding;
                }
                this.drafts.set(field.key, draft);
                const conflicted = draft.changed && (draft.original !== JSON.stringify(cell) || schemaChanged);
                if (conflicted)
                    wrapper.append(ui("p", "data-warning", uiMessage("dataPanel.fieldChanged")));
                const controls = ui("div", "data-property-actions");
                const apply = action(uiMessage("queryEdit.applyDraft"), async () => {
                    if (!wrapper.querySelector<HTMLInputElement>(":invalid"))
                        await this.applyField(field.key, draft.value, basis);
                    else
                        wrapper.querySelector<HTMLInputElement>(":invalid")?.reportValidity();
                }, "data-button primary");
                apply.disabled = !draft.changed || conflicted;
                if (schemaChanged && draft.changed)
                    wrapper.append(ui("p", "data-warning", uiMessage("dataPanel.fieldBindingChanged")), ui("pre", "data-readonly-source", JSON.stringify({ value: draft.value, rawInputs: draft.rawInputs }, null, 2)));
                else {
                    const changed = (next: NativeValue): void => { if (this.closeGuard.frozen) return; draft.value = next; draft.changed = true; draft.revision = (draft.revision ?? 0) + 1; apply.disabled = conflicted; };
                    if (field.id === "tasks/reminder") {
                        const form = reminderForm(draft.value, schema, changed, { fieldId: field.id, path: [] });
                        wrapper.append(form.element);
                        this.reminderDisposers.push(form.dispose);
                        const raw: ReminderRawInput[] = [], retained: string[] = [];
                        for (const control of form.element.querySelectorAll<HTMLInputElement | HTMLSelectElement>("input,select")) {
                            const address = nativeControlAddress(control);
                            if (!draft.rawInputs || !Object.hasOwn(draft.rawInputs, address.key)) continue;
                            const text = draft.rawInputs[address.key];
                            if (control instanceof HTMLInputElement) raw.push({path: address.identity.path, role: address.role, text});
                            else if (control.value !== text) retained.push(text);
                        }
                        retained.push(...form.restoreRaw(raw).rejected.map(item => item.input.text));
                        if (retained.length) wrapper.append(ui("pre", "data-readonly-source", retained.join("\n")));
                    } else {
                        wrapper.append(valueEditor(field.shape, draft.value, schema, changed, fieldNames[field.key] ?? field.key, { fieldId: field.id, path: [] }, 0, this.references));
                        for (const [key, control] of this.formInputs(wrapper)) if (draft.rawInputs && key in draft.rawInputs) {
                            control.value = draft.rawInputs[key];
                            control.dispatchEvent(new Event("input", {bubbles:true}));
                        }
                    }
                }
                controls.append(apply, action(uiMessage("dataPanel.discardFormChanges"), () => { this.drafts.delete(field.key); this.renderFields(); }));
                if (field.nullable && draft.value.kind !== "none")
                    controls.append(action(uiMessage("dataPanel.setExplicitNone"), () => { draft.value = { kind: "none" }; draft.changed = true; draft.revision = (draft.revision ?? 0) + 1; this.renderFields(); }));
                wrapper.append(controls);
            }
            const group = this.pinned.has(field.id) ? groups.pinned : commonFields.has(field.key) || required || this.filter ? groups.common : groups.advanced;
            group.append(wrapper);
        }
        if (!matching.length)
            target.append(ui("p", "data-empty", declared.length ? uiMessage("dataPanel.noFilterMatch") : uiMessage("dataPanel.noDeclaredFields")));
        if (groups.pinned.childElementCount) {
            groups.pinned.prepend(ui("h3", "data-group-label", uiMessage("dataPanel.pinnedFields")));
            target.append(groups.pinned);
        }
        target.append(groups.common);
        if (groups.advanced.childElementCount > 1)
            target.append(groups.advanced);
        this.renderedIdentity = this.draftIdentity;
    }
    private formInputs(wrapper: HTMLElement): [
        string,
        HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
    ][] {
        return nativeFormControls(wrapper);
    }
    private rememberInputs(): void {
        const drafts = this.renderedIdentity === this.draftIdentity ? this.drafts : this.renderedIdentity ? this.draftCache.get(this.renderedIdentity) : undefined;
        for (const wrapper of this.element.querySelectorAll<HTMLElement>(".data-property[data-field-key]")) {
            const draft = drafts?.get(wrapper.dataset.fieldKey!);
            if (!draft)
                continue;
            const raw: Record<string, string> = {};
            for (const [key, control] of this.formInputs(wrapper))
                if (!control.validity.valid) {
                    raw[key] = control.value;
                    draft.changed = true;
                }
            if (JSON.stringify(raw) !== JSON.stringify(draft.rawInputs ?? {})) draft.revision = (draft.revision ?? 0) + 1;
            if (Object.keys(raw).length)
                draft.rawInputs = raw;
            else if (wrapper.querySelector("input,select,textarea"))
                delete draft.rawInputs;
        }
    }
    private async applyField(key: string, value: NativeValue, basis: DocumentBasis | null): Promise<void> {
        if (this.closeGuard.frozen) return;
        const drafts = this.drafts, draft = drafts.get(key), revision = draft?.revision;
        await this.closeGuard.run(async () => {
            if (!await this.patch("/api/data/edit", { key, path: [], value }, basis)) return;
            if (drafts.get(key) === draft && draft?.revision === revision) drafts.delete(key);
            await this.refresh();
        });
    }
    private async applyTypes(types: string[], basis: DocumentBasis | null, identity: string): Promise<void> {
        if (this.closeGuard.frozen || this.typeInstalling.has(identity)) return;
        if (this.schema) {
            const conflicts = typeConflicts(this.schema, types);
            if (conflicts.length) {
                this.host.notify(joinUiText(conflicts, "；"), true);
                return;
            }
        }
        const draft = this.typeDrafts.get(identity);
        await this.closeGuard.run(async () => {
            this.typeInstalling.add(identity);
            try {
                if (!await this.patch("/api/data/types", { types }, basis)) return;
                if (this.typeDrafts.get(identity) === draft) this.typeDrafts.delete(identity);
                await this.refresh();
            } finally { this.typeInstalling.delete(identity); }
        });
    }
    private async patch(route: string, request: Record<string, unknown>, basis: DocumentBasis | null): Promise<boolean> {
        if (this.composing) {
            this.host.notify(uiMessage("dataPanel.compositionPending"), true);
            return false;
        }
        if (this.host.canEdit?.() === false) {
            this.host.notify(uiMessage("dataPanel.editUnavailable"), true);
            return false;
        }
        if (!basis)
            return false;
        if (this.host.getDocument()?.source !== basis.source || this.host.getDocument()?.sessionId !== basis.sessionId || this.host.getDocument()?.serial !== basis.serial) {
            this.host.notify(uiMessage("dataPanel.sourceBasisChanged"), true);
            return false;
        }
        this.element.setAttribute("aria-busy", "true");
        try {
            const result = await this.host.request<SourcePatch | {
                edit: SourcePatch;
            } | {
                patches: SourcePatch[];
                source?: string;
            }>(route, { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, ...request });
            await this.host.applyPatch(decodeSourcePatch(result, basis.source), basis);
            this.host.notify(uiMessage("dataPanel.applied"));
            return true;
        }
        catch (error) {
            this.host.notify(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"), true);
            return false;
        }
        finally {
            this.element.removeAttribute("aria-busy");
        }
    }
    private renderSchemaSettings(): void {
        const read = this.schemaRead;
        if (!read?.canInstall || !this.basis || this.projection?.readOnly)
            return;
        const basis = this.basis;
        const root = basis.root;
        const retained = this.schemaDrafts.get(root);
        const details = ui("details", "schema-settings");
        details.append(ui("summary", "", uiMessage("schemaSettings.disclosure")), ui("p", "data-help", displayMessage("schemaSettings.installScopeHelp", { p0: uiDisplayParameter(read.builtinDefault ? uiMessage("schemaSettings.builtinDefault") : uiMessage("schemaSettings.installed")) })));
        const source = ui("textarea", "data-input schema-source");
        source.rows = 10;
        source.value = retained?.source ?? read.source;
        source.oninput = () => {
            const proposal: SchemaProposal = { source: source.value, beforeSource: retained?.beforeSource ?? read.source, beforeRevision: retained ? retained.beforeRevision : read.revision };
            if (proposal.source === read.source && !this.schemaInstalling.has(root)) this.schemaDrafts.delete(root);
            else this.schemaDrafts.set(root, proposal);
            previewHost.replaceChildren();
        };
        setUiAttr(source, "aria-label", uiMessage("schemaSettings.sourceLabel"));
        details.append(source);
        const previewHost = ui("div", "schema-preview");
        details.append(previewHost);
        const showProposal = (proposal: SchemaProposal): void => {
            previewHost.replaceChildren(ui("pre", "", diagnosticText(proposal.preview)));
            const plan = proposal.plan;
            if (plan) previewHost.append(action(uiMessage("schemaSettings.install"), async () => {
                if (this.closeGuard.frozen || this.schemaInstalling.has(root) || this.schemaDrafts.get(root) !== proposal) return;
                if (this.host.getDocument()?.root !== root || this.host.canEdit?.() === false || this.composing) {
                    this.host.notify(uiMessage("schemaSettings.installContextChanged"), true);
                    return;
                }
                await this.closeGuard.run(async () => {
                    this.schemaInstalling.add(root);
                    try {
                        await this.host.request("/api/data/schema/install", plan);
                        this.host.environmentChanged?.(root);
                        if (this.schemaDrafts.get(root) === proposal) this.schemaDrafts.delete(root);
                        this.host.notify(uiMessage("schemaSettings.installedNotice"));
                        await this.refresh();
                    } catch (error) {
                        this.host.notify(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"), true);
                    } finally { this.schemaInstalling.delete(root); }
                });
            }, "data-button primary"));
            else previewHost.append(ui("p", "data-warning", uiMessage("schemaSettings.noExecutablePlan")));
        };
        if (retained?.preview !== undefined) showProposal(retained);
        const prepare = action(uiMessage("schemaSettings.prepare"), async () => {
            if (this.closeGuard.frozen) return;
            if (this.host.getDocument()?.root !== basis.root || this.host.canEdit?.() === false || this.composing) {
                this.host.notify(uiMessage("schemaSettings.contextChanged"), true);
                return;
            }
            const proposal: SchemaProposal = { source: source.value, beforeSource: retained?.beforeSource ?? read.source, beforeRevision: retained ? retained.beforeRevision : read.revision };
            this.schemaDrafts.set(root, proposal);
            prepare.disabled = true;
            const finish = this.closeGuard.begin();
            try {
                JSON.parse(proposal.source);
                const result = await this.host.request<{
                    planId?: string;
                    operationId?: string;
                    plan?: {
                        planId?: string;
                    };
                    preview?: unknown;
                    diagnostics?: unknown[];
                }>("/api/data/schema/prepare", { root, source: proposal.source, beforeSource: proposal.beforeSource, beforeRevision: proposal.beforeRevision, operationId: crypto.randomUUID() });
                if (this.schemaDrafts.get(root) !== proposal) return;
                const planId = result.planId ?? result.plan?.planId;
                proposal.preview = result.preview ?? result.diagnostics ?? result;
                if (planId && result.operationId) proposal.plan = { planId, operationId: result.operationId };
                showProposal(proposal);
            }
            catch (error) {
                previewHost.replaceChildren(ui("p", "data-warning", error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse")));
            }
            finally {
                prepare.disabled = false;
                finish();
            }
        });
        details.append(prepare, action(uiMessage("dataPanel.discardFormChanges"), () => { this.schemaDrafts.delete(root); this.render(); }));
        this.element.append(details);
    }
}
export interface CaptureContext {
    root: string;
    parent?: string;
}
export class QuickCapture {
    readonly element = ui("section", "quick-capture");
    private operationId: string | null = null;
    private pending: Record<string, unknown> | null = null;
    private pendingRevision: number | null = null;
    private schema: DataSchema | null = null;
    private references = emptyReferences();
    private reminderDisposers: (() => void)[] = [];
    private readonly closeGuard = new ProposalCloseGuard();
    private dirty = false;
    private inputRevision = 0;
    private submitting = false;
    private initializing = false;
    private markInput(): void { if (!this.closeGuard.frozen) { this.dirty = true; this.inputRevision++; } }
    assertCloseReady(): void { this.closeGuard.assertReady(this.dirty || this.pending !== null); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.dirty || this.pending !== null); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    dispose(): void { this.reminderDisposers.splice(0).forEach(dispose => dispose()); }
    constructor(private readonly host: PanelHost, private context: CaptureContext) {
        setUiAttr(this.element, "aria-label", uiMessage("quickCapture.title"));
        this.closeGuard.bind(this.element);
        for (const event of ["input", "change"]) this.element.addEventListener(event, () => this.markInput(), true);
    }
    async initialize(): Promise<void> {
        if (this.closeGuard.frozen || this.schema || this.initializing) return;
        const finish = this.closeGuard.begin();
        this.initializing = true;
        try {
            const result = await this.host.request<DataSchema | {
                schema: DataSchema;
            }>("/api/data/schema", { root: this.context.root });
            this.schema = "schema" in result ? result.schema : result;
            this.render();
            await loadFormReferences(this.host, this.context.root, this.references, this.element, () => !this.closeGuard.frozen);
        }
        catch (error) {
            if (this.schema) this.host.notify(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"), true);
            else this.element.replaceChildren(ui("p", "data-warning", (error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"))), action(uiMessage("common.retry"), () => this.initialize()));
        }
        finally { this.initializing = false; finish(); }
    }
    updateContext(): void {
        if (this.closeGuard.frozen) return;
        const template = this.element.querySelector<HTMLButtonElement>(".capture-template-entry");
        const help = this.element.querySelector<HTMLElement>(".capture-template-help");
        if (!template || !help)
            return;
        const document = this.host.getDocument();
        template.disabled = !this.host.openTemplates || !document?.nodeId || document.root !== this.context.root;
        setUiText(help, template.disabled ? uiMessage("quickCapture.openLibraryNoteForTemplate") : uiMessage("quickCapture.templateWizardHelp"));
    }
    private render(): void {
        if (this.closeGuard.frozen) return;
        this.dispose();
        if (!this.schema)
            return;
        const schema = this.schema;
        const form = ui("form", "capture-form");
        form.append(ui("h2", "", uiMessage("quickCapture.title")));
        const template = action(uiMessage("quickCapture.openTemplateWizard"), async () => {
            if (this.closeGuard.frozen) return;
            const finish = this.closeGuard.begin();
            try {
                await this.host.openTemplates?.();
            }
            catch (error) {
                this.host.notify(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"), true);
            }
            finally { finish(); }
        }, "data-button capture-template-entry");
        form.append(template, ui("p", "data-help capture-template-help"));
        const name = ui("input", "data-input");
        name.required = true;
        setUiAttr(name, "placeholder", uiMessage("periodNavigation.namePlaceholder"));
        const parent = ui("input", "data-input");
        parent.value = this.context.parent ?? this.context.root;
        parent.required = true;
        const title = ui("input", "data-input");
        setUiAttr(title, "placeholder", uiMessage("quickCapture.optionalPlaceholder"));
        const body = ui("textarea", "data-input capture-body");
        body.rows = 7;
        setUiAttr(body, "placeholder", uiMessage("periodNavigation.nativeBody"));
        form.append(labelFor(uiMessage("periodNavigation.parent"), parent), parent, labelFor(uiMessage("quickCapture.requiredName"), name), name, labelFor(uiMessage("periodNavigation.optionalTitle"), title), title);
        const type = ui("select", "data-input");
        const ordinary = ui("option", "", uiMessage("dataPanel.ordinaryNote"));
        ordinary.value = "";
        type.append(ordinary);
        schema.types.forEach(item => { const alias = Object.entries(schema.aliases).find(([, id]) => id === item.id)?.[0] ?? item.id; const option = ui("option", "", typeNames[alias] ?? alias); option.value = alias; type.append(option); });
        form.append(labelFor(uiMessage("queryPlan.noteTypes"), type), type);
        const fields = ui("div", "capture-fields");
        let values = new Map<string, NativeValue>();
        form.append(fields);
        type.onchange = () => {
            this.dispose();
            fields.replaceChildren();
            values = new Map();
            for (const { field, required } of fieldsForTypes(schema, type.value ? [type.value] : [])) {
                const container = required || commonFields.has(field.key) ? ui("section", "capture-property") : ui("details", "capture-property");
                const heading = container instanceof HTMLDetailsElement ? ui("summary", "", fieldNames[field.key] ?? field.key) : ui("h3", "", () => `${uiTextValue(fieldNames[field.key] ?? field.key)}${uiTextValue(required ? " *" : "")}`);
                container.append(heading);
                const addEditor = (): void => {
                    this.markInput();
                    const value = field.id === "tasks/reminder" ? newReminder("absolute") : emptyNativeValue(field.shape, schema);
                    values.set(field.key, value);
                    const changed = (next: NativeValue): void => { if (this.closeGuard.frozen) return; this.markInput(); values.set(field.key, next); };
                    if (field.id === "tasks/reminder") {
                        const form = reminderForm(value, schema, changed, {fieldId:field.id,path:[]});
                        container.append(form.element);
                        this.reminderDisposers.push(form.dispose);
                    } else container.append(valueEditor(field.shape, value, schema, changed, fieldNames[field.key] ?? field.key, {fieldId:field.id,path:[]}, 0, this.references));
                };
                if (required)
                    addEditor();
                else {
                    const add = action(uiMessage("quickCapture.fillField"), () => { add.remove(); addEditor(); });
                    container.append(add);
                }
                fields.append(container);
            }
        };
        form.append(labelFor(uiMessage("quickCapture.bodyHeading"), body), body);
        const feedback = ui("p", "data-help", uiMessage("quickCapture.clearAfterOpenHelp"));
        feedback.setAttribute("aria-live", "polite");
        const submit = ui("button", "data-button primary", uiMessage("quickCapture.createOpen"));
        submit.type = "submit";
        const discard = action(uiMessage("dataPanel.discardFormChanges"), () => {
            if (this.pending || this.submitting || this.closeGuard.frozen) return;
            this.dirty = false; this.inputRevision++; this.render();
        });
        form.append(feedback, submit, discard);
        this.element.replaceChildren(form);
        this.updateContext();
        form.onsubmit = event => {
            event.preventDefault();
            if (this.submitting || this.closeGuard.frozen) return;
            if (!form.reportValidity())
                return;
            void (async () => {
                const finish = this.closeGuard.begin();
                this.submitting = true;
                const retry = this.pending !== null;
                submit.disabled = true;
                discard.disabled = true;
                let receiptReceived = false;
                try {
                    this.operationId ??= crypto.randomUUID();
                    if (!this.pending) {
                        this.pending = structuredClone({ root: this.context.root, parent: parent.value, name: name.value, title: title.value || undefined, types: type.value ? [type.value] : [], fields: Object.fromEntries(values), body: body.value, operationId: this.operationId });
                        this.pendingRevision = this.inputRevision;
                    }
                    const submitted = this.pending;
                    const submittedRevision = this.pendingRevision;
                    const result = await submitOriginal<OriginalFact & {
                        document?: OpenedDocument;
                        documentError?: ApiErrorBody;
                    }>(this.host.request, "/api/data/capture", this.pending, "capture", retry);
                    if (result.status !== "committed")
                        throw new UiError(displayMessage("quickCapture.creationUnconfirmed", { p0: uiDisplayParameter(result.status) }));
                    receiptReceived = true;
                    const document = result.document;
                    const documentError = result.documentError === undefined ? undefined : decodeApiErrorBody(result.documentError);
                    if (result.documentError !== undefined && !documentError) throw new UiError(uiMessage("error.invalidResponse"));
                    const deliveryError = result.deliveryError === undefined ? undefined : decodeApiErrorBody(result.deliveryError);
                    if (result.deliveryError !== undefined && !deliveryError) throw new UiError(uiMessage("error.invalidResponse"));
                    const deliveryErrors = [documentError, deliveryError].filter((error): error is ApiErrorBody => !!error);
                    if (deliveryErrors.length || !document)
                        throw new UiError(displayMessage("quickCapture.submittedOpenPending", { p0: uiDisplayParameter(deliveryErrors.length ? joinUiText(deliveryErrors.map(apiErrorUiText), "\n") : uiMessage("common.confirmOriginalOperation")) }));
                    if (!this.host.created)
                        throw new UiError(uiMessage("capture.createdDocumentDeliveryRequired"));
                    await this.host.created(document, result.operationId);
                    const unchanged = this.inputRevision === submittedRevision && name.value === submitted.name && parent.value === submitted.parent && (title.value || undefined) === submitted.title && body.value === submitted.body && JSON.stringify(type.value ? [type.value] : []) === JSON.stringify(submitted.types) && JSON.stringify(Object.fromEntries(values)) === JSON.stringify(submitted.fields);
                    this.operationId = null;
                    this.pending = null;
                    this.pendingRevision = null;
                    if (unchanged) {
                        this.dirty = false;
                        name.value = "";
                        title.value = "";
                        body.value = "";
                        values.clear();
                        type.value = "";
                        fields.replaceChildren();
                    }
                    feedback.className = "data-help";
                    setUiText(feedback, unchanged ? uiMessage("quickCapture.createdOpened") : uiMessage("quickCapture.createdWithNewInput"));
                    this.updateContext();
                    await loadFormReferences(this.host, this.context.root, this.references, this.element, () => !this.closeGuard.frozen);
                }
                catch (error) {
                    if (!receiptReceived && error instanceof TerminatedAttempt) {
                        this.operationId = null;
                        this.pending = null;
                        this.pendingRevision = null;
                    }
                    feedback.className = "data-warning";
                    setUiText(feedback, () => `${uiTextValue(error instanceof Error ? errorUiText(error) : uiMessage("error.invalidResponse"))}${uiTextValue(this.pending ? uiMessage("quickCapture.originalCreateRetainedSuffix") : uiMessage("quickCapture.originalInputRetainedSuffix"))}`);
                }
                finally {
                    submit.disabled = false;
                    discard.disabled = this.pending !== null;
                    this.submitting = false;
                    finish();
                }
            })();
        };
    }
}
