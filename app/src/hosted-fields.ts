import { schemaFieldLabel } from "./builtin-property-labels";
import { uiDisplayParameter } from "./ui-locale";
import { message, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, nativeFormControls, emptyNativeValue, formatNativeValue, ui, valueEditor, type Cell, type DataSchema, type NativeValue, type Shape, type FormReferences } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { TagTree } from "./tag-tree";
import { registryIntent, registryReceipt, type RegistryCatalog, type RegistryReceipt, type RegistryRequest } from "./hosted-navigation-registry-state";
import { tagPicker } from "./tag-picker";
type Grant = {
    resource: string;
    fieldId: string;
    write: boolean;
};
type Profile = {
    resource: string;
    basis: string | null;
    field: {
        fieldId: string;
        key: string;
        shape: Shape;
        nullable: boolean;
        required: boolean;
        cell: Cell;
        writable: boolean;
    };
};
type Proposal = {
    resource: string;
    fieldId: string;
    basis: string;
    value: NativeValue;
    operationId: string;
};
type TagCreation = {
    request: RegistryRequest;
    originalValue: NativeValue;
    receipt?: RegistryReceipt;
    phase?: "prepare" | "commit" | "settle";
};
type Draft = {
    value: NativeValue;
    shape?: Shape;
    rawInputs?: Record<string, string>;
    pending?: Proposal;
    newTag?: {
        displayName: string;
        parentRef: string | null;
    };
    tagCreation?: TagCreation;
};
type Receipt = {
    status: string;
    cell?: Cell;
};
type Request = <T>(path: string, data?: unknown) => Promise<T>;
const schema: DataSchema = { definitions: {}, fields: [], types: [], aliases: {} };
export class HostedFields {
    private epoch = 0;
    private inputScopes: AuthorInputScopes;
    private prefix = "";
    private selected?: Grant;
    private profile?: Profile;
    private draft?: Draft;
    private compare = false;
    private busy = false;
    private grants: Grant[] = [];
    private form?: HTMLFieldSetElement;
    private formDraft?: Draft;
    private unreadableDraft?: string;
    private references: FormReferences = { nodes: [], state: "unavailable", messages: [] };
    private tagCatalog?: RegistryCatalog;
    private verifiedTagOperation?: string;
    private readonly picker = ui("select");
    private readonly editor = ui("section");
    private readonly message = ui("p", "hint");
    private readonly results = ui("section");
    constructor(private host: HTMLElement, private request: Request, private onQuery?: (id: string, metadata: {
        columns: string[];
        rowCount: number;
        coverage: string;
    }) => void, private changed?: () => void, authorInputs?: AuthorInputFactory) { this.inputScopes = new AuthorInputScopes(authorInputs); this.clear(false); }
    canEdit(resource: string, fieldId: string): boolean { return this.grants.some(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write); }
    async openGranted(resource: string, fieldId: string): Promise<void> {
        const grant = this.grants.find(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write);
        if (!grant)
            throw new UiError(message("hostedFields.fieldWriteUnauthorized"));
        await this.run(() => this.open(grant));
        this.host.scrollIntoView({ block: "start" });
        this.editor.querySelector<HTMLElement>("input,select,textarea")?.focus({ preventScroll: true });
    }
    clear(removeCache = true) {
        this.inputScopes.clear();
        this.epoch++;
        this.profile = undefined;
        this.selected = undefined;
        this.draft = undefined;
        this.references = { nodes: [], state: "unavailable", messages: [] };
        this.tagCatalog = undefined;
        this.verifiedTagOperation = undefined;
        this.grants = [];
        this.busy = false;
        this.form = undefined;
        this.formDraft = undefined;
        this.unreadableDraft = undefined;
        this.picker.replaceChildren();
        this.editor.replaceChildren();
        this.results.replaceChildren();
        this.host.replaceChildren();
        this.host.hidden = true;
        this.prefix = "";
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-field:"))
                    sessionStorage.removeItem(key);
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch;
        const { fields } = await this.request<{
            fields: Grant[];
        }>("fields");
        if (epoch !== this.epoch)
            return;
        this.prefix = hostedCachePrefix("field", deployment, actor, generation);
        this.grants = fields;
        this.host.hidden = !fields.length;
        this.host.replaceChildren(ui("h2", "", message("hostedFields.title")), ui("p", "hint", message("hostedFields.saveScopeHelp")));
        setUiAttr(this.picker, "aria-label", message("hostedFields.pickerLabel"));
        this.picker.replaceChildren(...fields.map((f, i) => { const option = ui("option", "", `${f.resource} · ${f.fieldId}`); option.value = String(i); return option; }));
        this.host.append(this.picker, action(message("hostedFields.readField"), () => this.run(() => this.open(fields[Number(this.picker.value)]))), this.message, this.editor);
        const queryField = ui("select");
        setUiAttr(queryField, "aria-label", message("hostedFields.queryFieldLabel"));
        for (const id of new Set(fields.map(f => f.fieldId))) {
            const option = ui("option", "", id);
            option.value = id;
            queryField.append(option);
        }
        const predicate = ui("input");
        setUiAttr(predicate, "aria-label", message("hostedFields.predicateLabel"));
        setUiAttr(predicate, "placeholder", message("hostedFields.predicatePlaceholder"));
        this.host.append(ui("h3", "", message("hostedFields.queryTitle")), queryField, predicate, action(message("hostedFields.runQuery"), () => this.run(async () => {
            const epoch = this.epoch;
            const response = await this.request<{
                queryId: string;
                columns: string[];
                result: {
                    rows: {
                        entityKey?: string;
                        cells: Record<string, Cell>;
                    }[];
                    coverage: string;
                    limited: boolean;
                };
            }>("query", { fields: [queryField.value], plan: { where: predicate.value || null } });
            if (epoch !== this.epoch)
                return;
            const table = ui("table");
            setUiAttr(table, "aria-label", message("hostedFields.resultTableLabel"));
            for (const row of response.result.rows) {
                const tr = ui("tr");
                tr.append(ui("th", "", row.entityKey ?? message("hostedFields.groupFallback")));
                for (const cell of Object.values(row.cells))
                    tr.append(ui("td", "", this.cellText(cell)));
                table.append(tr);
            }
            this.results.replaceChildren(ui("p", "", displayMessage("hostedFields.resultSummary", { p0: response.result.rows.length, p1: uiDisplayParameter(response.result.coverage === "partial" || response.result.limited ? message("hostedFields.partialResultSuffix") : "") })), table);
            this.onQuery?.(response.queryId, { columns: response.columns, rowCount: response.result.rows.length, coverage: response.result.coverage });
        })), this.results);
    }
    private key(g = this.selected!) { return this.prefix + encodeURIComponent(g.resource) + ":" + encodeURIComponent(g.fieldId); }
    private controls(form: HTMLFieldSetElement) {
        return nativeFormControls(form).map(([key, control]) => ({ key, control }));
    }
    private captureInputs() {
        if (!this.form || this.formDraft !== this.draft || !this.draft)
            return;
        const raw = { ...this.draft.rawInputs };
        for (const { key, control } of this.controls(this.form)) {
            if (!control.validity.valid)
                raw[key] = control.value;
            else
                delete raw[key];
        }
        if (Object.keys(raw).length)
            this.draft.rawInputs = raw;
        else
            delete this.draft.rawInputs;
    }
    private restoreInputs(form: HTMLFieldSetElement, draft: Draft) {
        for (const { key, control } of this.controls(form))
            if (draft.rawInputs && key in draft.rawInputs) {
                control.value = draft.rawInputs[key];
                control.dispatchEvent(new Event("input", { bubbles: true }));
            }
    }
    private unmappedInputs(form: HTMLFieldSetElement, draft: Draft) {
        const keys = new Set(this.controls(form).map(({ key }) => key));
        return Object.entries(draft.rawInputs ?? {}).filter(([key]) => !keys.has(key));
    }
    private retain() {
        if (this.selected && this.draft) {
            this.captureInputs();
            sessionStorage.setItem(this.key(), JSON.stringify(this.draft));
        }
    }
    private cellText(cell: Cell) { return cell.value ? formatNativeValue(cell.value) : ({ absent: message("queryValue.absent"), null: message("hostedFields.nullCell"), invalid: message("hostedQuery.invalidCell"), unavailable: message("queryValue.unavailable"), value: "" })[cell.state]; }
    private async run(work: () => Promise<void>) {
        if (this.busy)
            return;
        this.busy = true;
        const epoch = this.epoch;
        this.render();
        try {
            await work();
        }
        catch (e) {
            if (epoch === this.epoch)
                setUiText(this.message, e instanceof Error ? errorUiText(e) : message("hostedFields.operationIncomplete"));
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async open(grant: Grant) {
        if (!grant)
            return;
        this.retain();
        this.inputScopes.clear();
        const epoch = this.epoch;
        const stored = sessionStorage.getItem(this.key(grant));
        this.selected = grant;
        this.profile = undefined;
        this.draft = undefined;
        this.unreadableDraft = undefined;
        this.tagCatalog = undefined;
        this.verifiedTagOperation = undefined;
        try {
            this.draft = stored ? JSON.parse(stored) as Draft : undefined;
            const job = this.draft?.tagCreation;
            if (job && (!job.request || typeof job.request.operationId !== "string" || !job.originalValue || job.request.intent?.kind !== "createTag" || job.request.intent.assignTo?.resource !== grant.resource || job.request.intent.assignTo?.fieldId !== grant.fieldId))
                throw new UiError(message("hostedFields.tagOperationMismatch"));
            if (job?.receipt)
                registryReceipt(job.request, job.receipt);
        }
        catch {
            this.unreadableDraft = stored!;
            throw new UiError(message("hostedFields.unreadableRecovery"));
        }
        this.compare = !!stored;
        setUiText(this.message, this.draft?.pending ? message("hostedFields.pendingOriginalRead") : message("hostedFields.readingField"));
        this.render();
        const profile = await this.request<Profile>("fields/read", { resource: grant.resource, fieldId: grant.fieldId });
        if (epoch !== this.epoch)
            return;
        this.profile = profile;
        this.references = { nodes: [], state: "unavailable", messages: [] };
        // Registry visibility is separate from field write permission; lack of tags.read
        // never removes an existing typed reference or prevents an exact field write.
        try {
            const catalog = await this.request<RegistryCatalog>("navigation/registry");
            if (epoch !== this.epoch)
                return;
            this.references.tagTree = new TagTree(catalog.tags.items, catalog.tags.read && catalog.tags.coverage === "complete");
            this.tagCatalog = catalog;
            this.references.state = "ready";
        }
        catch (error) {
            if (epoch !== this.epoch)
                return;
            this.references.messages.push(String(error));
        }
        this.draft ??= { value: profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? emptyNativeValue(profile.field.shape, schema) };
        this.draft.shape = profile.field.shape;
        setUiText(this.message, this.draft.pending ? message("hostedFields.pendingOriginalRequired") : stored ? message("hostedFields.recoveredCompareRequired") : message("hostedFields.fieldReadUnsaved"));
    }
    private render() {
        this.captureInputs();
        this.form = undefined;
        this.formDraft = undefined;
        this.editor.replaceChildren();
        const profile = this.profile, draft = this.draft;
        if (this.unreadableDraft && this.selected) {
            this.editor.append(ui("h3", "", `${this.selected.resource} · ${this.selected.fieldId}`), ui("pre", "", this.unreadableDraft), action(message("hostedFields.discardUnreadableRecovery"), () => {
                sessionStorage.removeItem(this.key());
                this.unreadableDraft = undefined;
                return this.run(() => this.open(this.selected!));
            }));
            return;
        }
        if (!draft || !this.selected)
            return;
        const selected = this.selected, epoch = this.epoch, inputs = this.inputScopes.get(draft, { kind: "field", resource: selected.resource, fieldId: selected.fieldId }, () => this.epoch === epoch && this.draft === draft && this.selected === selected);
        // The authorized profile supplies one expanded shape; captions never require reading the whole schema.
        const presentedField = profile && { id: selected.fieldId, key: profile.field.key, shape: profile.field.shape, nullable: profile.field.nullable };
        const editorSchema: DataSchema = { ...schema, fields: presentedField ? [presentedField] : [] };
        this.editor.append(ui("h3", "", () => `${selected.resource} · ${uiTextValue(presentedField ? schemaFieldLabel(editorSchema, presentedField) : selected.fieldId)}`));
        this.editor.append(ui("p", "", profile ? displayMessage("hostedFields.currentCell", { p0: uiDisplayParameter(this.cellText(profile.field.cell)) }) : message("hostedFields.currentReadUnavailable")));
        const form = ui("fieldset");
        form.disabled = this.busy || !profile?.field.writable || !!draft.pending || !!draft.tagCreation;
        const shape = profile?.field.shape ?? draft.shape;
        if (shape)
            form.append(valueEditor(shape, draft.value, editorSchema, value => {
                draft.value = value;
                try {
                    this.retain();
                    setUiText(this.message, message("hostedFields.inputRetainedUnsaved"));
                }
                catch {
                    setUiText(this.message, message("hostedFields.recoveryPersistenceFailed"));
                }
            }, message("hostedFields.inputLabel"), { fieldId: this.selected.fieldId, path: [] }, 0, this.references, inputs ? { session: inputs, slot: "value" } : undefined));
        else
            form.append(ui("pre", "", formatNativeValue(draft.value)));
        if (profile?.field.nullable)
            form.append(action(message("hostedFields.setNull"), () => { draft.value = { kind: "none" }; this.retain(); this.render(); }));
        this.editor.append(form);
        this.restoreInputs(form, draft);
        this.form = form;
        this.formDraft = draft;
        const unmapped = this.unmappedInputs(form, draft);
        if (unmapped.length)
            this.editor.append(ui("p", "hint", message("hostedFields.unmatchedInputHelp")), ...unmapped.map(([, text]) => ui("pre", "", text)), action(message("hostedFields.discardUnmatchedInput"), () => {
                for (const [key] of unmapped)
                    delete draft.rawInputs![key];
                this.retain();
                this.render();
            }));
        form.addEventListener("input", () => {
            try {
                this.retain();
                setUiText(this.message, draft.rawInputs ? message("hostedFields.inputIncomplete") : message("hostedFields.inputRetainedUnsaved"));
            }
            catch {
                setUiText(this.message, message("hostedFields.recoveryPersistenceFailed"));
            }
        });
        form.addEventListener("click", event => {
            if (!(event.target instanceof Element) || !event.target.closest("button"))
                return;
            const invalid = this.controls(form).find(({ control }) => !control.validity.valid)?.control;
            if (invalid) {
                event.preventDefault();
                event.stopPropagation();
                invalid.reportValidity();
                setUiText(this.message, message("hostedFields.completeInputBeforeStructure"));
            }
        }, { capture: true });
        if (!profile?.field.writable)
            this.editor.append(ui("p", "hint", message("hostedFields.readOnlyHelp")));
        const save = action(message("hostedFields.saveField"), () => {
            if (this.unmappedInputs(form, draft).length)
                return;
            if (this.controls(form).some(({ control }) => !control.reportValidity())) {
                setUiText(this.message, message("hostedFields.invalidInputRetained"));
                return;
            }
            return this.run(async () => {
                if (!profile?.basis || draft.pending || this.compare)
                    return;
                const epoch = this.epoch;
                draft.pending = { resource: profile.resource, fieldId: profile.field.fieldId, basis: profile.basis, value: structuredClone(draft.value), operationId: crypto.randomUUID() };
                this.retain(); // Must succeed before any author request.
                const result = await this.request<Receipt>("fields/write", draft.pending);
                if (epoch !== this.epoch)
                    return;
                this.applyReceipt(result);
            });
        });
        save.disabled = this.busy || !profile?.basis || this.compare || !!draft.pending || !!draft.tagCreation || !!unmapped.length;
        const reread = action(message("hostedFields.rereadField"), () => this.run(() => this.open(this.selected!)));
        reread.disabled = this.busy || !!draft.pending || !!draft.tagCreation;
        const rebase = action(message("hostedFields.adoptCurrentBasis"), () => { this.compare = false; setUiText(this.message, message("hostedFields.basisConfirmed")); this.render(); });
        rebase.disabled = this.busy || !!draft.pending || !this.compare || !profile?.basis;
        const recover = action(message("hostedFields.inspectOriginal"), () => this.run(async () => {
            if (!draft.pending)
                return;
            const epoch = this.epoch;
            const req = { resource: draft.pending.resource, fieldId: draft.pending.fieldId, operationId: draft.pending.operationId };
            let result = await this.request<Receipt>("fields/operation", req);
            if (epoch !== this.epoch)
                return;
            if (["notRegistered", "notCommitted"].includes(result.status))
                result = await this.request<Receipt>("fields/settle", req);
            if (epoch !== this.epoch)
                return;
            this.applyReceipt(result);
        }));
        recover.disabled = this.busy || !draft.pending;
        this.editor.append(save, reread, rebase, recover);
        if (profile?.field.key === "tags" || draft.tagCreation)
            this.renderTagCreation();
    }
    private renderTagCreation(): void {
        const draft = this.draft!, profile = this.profile, section = ui("section");
        section.append(ui("h3", "", message("hostedFields.createAssignTitle")));
        draft.newTag ??= { displayName: "", parentRef: null };
        const form = ui("fieldset");
        form.disabled = this.busy || !!draft.pending || !!draft.tagCreation || !profile?.field.writable || this.compare;
        const name = ui("input", "data-input");
        setUiAttr(name, "aria-label", message("hostedFields.newTagNameLabel"));
        name.value = draft.newTag.displayName;
        const selected = this.selected!, epoch = this.epoch;
        this.inputScopes.get(draft, { kind: "field", resource: selected.resource, fieldId: selected.fieldId }, () => this.epoch === epoch && this.draft === draft && this.selected === selected)?.bind(name, "new-tag-name");
        const duplicateNames = ui("p", "hint");
        const names = () => { const count = this.tagCatalog?.tags.items.filter(tag => tag.displayName === name.value).length ?? 0; setUiText(duplicateNames, count ? displayMessage("hostedFields.duplicateTagNames", { p0: count }) : ""); };
        name.oninput = () => { draft.newTag!.displayName = name.value; this.retain(); names(); };
        names();
        form.append(name, duplicateNames);
        if (this.references.tagTree)
            form.append(tagPicker(this.references.tagTree, { label: message("hostedFields.newTagParentLabel"), value: draft.newTag.parentRef ?? "", root: true, changed: parentRef => { draft.newTag!.parentRef = parentRef; this.retain(); } }));
        else
            form.append(ui("p", "hint", message("hostedFields.tagTreeUnavailable")));
        const prepare = action(message("hostedFields.previewCreateAssign"), () => this.run(async () => {
            if (!profile?.basis || !profile.field.writable || this.compare || draft.pending || draft.tagCreation || !this.tagCatalog || !draft.newTag)
                return;
            const saved = profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? emptyNativeValue(profile.field.shape, schema);
            if (draft.rawInputs || JSON.stringify(saved) !== JSON.stringify(draft.value))
                throw new UiError(message("hostedFields.unsavedInputBeforeTagCreation"));
            const intent = registryIntent({ kind: "createTag", reference: "", slot: "", ...draft.newTag }, this.tagCatalog);
            if (intent.kind !== "createTag")
                return;
            intent.assignTo = { resource: profile.resource, fieldId: profile.field.fieldId, basis: profile.basis };
            const job: TagCreation = { request: { operationId: crypto.randomUUID(), intent }, originalValue: structuredClone(draft.value), phase: "prepare" };
            draft.tagCreation = job;
            this.retain();
            const epoch = this.epoch;
            const receipt = await this.request<RegistryReceipt>("navigation/registry/prepare", structuredClone(job.request));
            if (epoch !== this.epoch || this.draft !== draft)
                return;
            await this.receiveTagCreation(receipt);
        }));
        prepare.disabled = !this.references.tagTree || !this.tagCatalog?.tags.create;
        form.append(prepare);
        section.append(form);
        const job = draft.tagCreation;
        if (job) {
            const receipt = this.verifiedTagOperation === job.request.operationId ? job.receipt : undefined;
            section.append(ui("p", "hint", displayMessage("hostedFields.originalTagStatus", { p0: uiDisplayParameter(job.request.operationId), p1: uiDisplayParameter(receipt?.status ?? message("hostedFields.authorizationCheckPending")) })));
            if (receipt)
                section.append(ui("pre", "", JSON.stringify(receipt.preview, null, 2)));
            const commit = action(message("hostedFields.commitOriginalCreateAssign"), () => this.run(async () => {
                if (!receipt?.commitAllowed || !receipt.planId || job.phase)
                    return;
                job.phase = "commit";
                this.retain();
                const epoch = this.epoch;
                const next = await this.request<RegistryReceipt>("navigation/registry/commit", { operationId: job.request.operationId, planId: receipt.planId });
                if (epoch !== this.epoch || this.draft !== draft)
                    return;
                await this.receiveTagCreation(next);
            }));
            commit.disabled = this.busy || !!job.phase || !receipt?.commitAllowed;
            const lookup = action(message("hostedFields.inspectOriginalCreateAssign"), () => this.run(async () => {
                const epoch = this.epoch;
                const next = await this.request<RegistryReceipt>("navigation/registry/operation", { operationId: job.request.operationId });
                if (epoch !== this.epoch || this.draft !== draft)
                    return;
                await this.receiveTagCreation(next);
            }));
            lookup.disabled = this.busy;
            const cancel = action(message("hostedFields.cancelOriginalCreateAssign"), () => this.run(async () => {
                job.phase = "settle";
                this.retain();
                const epoch = this.epoch;
                const next = await this.request<RegistryReceipt>("navigation/registry/settle", { operationId: job.request.operationId });
                if (epoch !== this.epoch || this.draft !== draft)
                    return;
                await this.receiveTagCreation(next);
            }));
            cancel.disabled = this.busy || !!receipt && ["committed", "cancelled", "resolved"].includes(receipt.status);
            const fresh = action(message("hostedFields.finishTagRecord"), () => { delete draft.tagCreation; this.verifiedTagOperation = undefined; this.retain(); this.render(); });
            fresh.disabled = this.busy || !receipt || !["committed", "cancelled", "resolved"].includes(receipt.status);
            section.append(commit, lookup, cancel, fresh);
        }
        section.append(ui("p", "hint", message("hostedFields.directAssignmentHelp")));
        this.editor.append(section);
    }
    private async receiveTagCreation(incoming: RegistryReceipt): Promise<void> {
        const draft = this.draft!, job = draft.tagCreation!;
        job.receipt = registryReceipt(job.request, incoming, job.receipt);
        job.phase = undefined;
        this.verifiedTagOperation = job.request.operationId;
        this.retain();
        if (incoming.status === "committed" && job.request.intent.kind === "createTag" && job.request.intent.assignTo) {
            this.changed?.();
            const epoch = this.epoch, selected = job.request.intent.assignTo, profile = await this.request<Profile>("fields/read", { resource: selected.resource, fieldId: selected.fieldId });
            if (epoch !== this.epoch || this.draft !== draft)
                return;
            this.profile = profile;
            this.compare = true;
            if (JSON.stringify(draft.value) === JSON.stringify(job.originalValue))
                draft.value = profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? draft.value;
            this.references.tagTree = undefined;
            this.tagCatalog = undefined;
            this.retain();
            setUiText(this.message, message("hostedFields.createAssignCommitted"));
        }
        else
            setUiText(this.message, incoming.status === "prepared" ? message("hostedFields.createAssignPrepared") : message("hostedFields.tagFactsChecked"));
    }
    private applyReceipt(receipt: Receipt) {
        if (!this.draft)
            return;
        if (["committed", "cancelled", "resolved"].includes(receipt.status)) {
            if (receipt.status === "committed")
                this.changed?.();
            delete this.draft.pending;
            this.compare = true;
            if (this.profile)
                this.profile.basis = null;
            if (receipt.status === "committed" && receipt.cell?.value)
                this.draft.value = receipt.cell.value;
            if (receipt.status === "committed" && receipt.cell?.state === "null")
                this.draft.value = { kind: "none" };
            this.retain();
            setUiText(this.message, receipt.status === "committed" ? message("hostedFields.originalSaved") : receipt.status === "resolved" ? message("hostedFields.originalEndedUncommitted") : message("hostedFields.originalCancelled"));
        }
        else
            setUiText(this.message, message("hostedFields.originalUnconfirmed"));
    }
}
