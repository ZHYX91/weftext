import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { AuthorInputFactory, AuthorInputSession } from "./author-input";
import { action, ui } from "./data-panel";
import { parseQueryPlan, type QueryPlan } from "./query-panel";
import { hostedCachePrefix } from "./hosted-state";
import type { SourcePatch } from "./editor";
import type { EmbeddedQueryBasis } from "./embedded-query";
import { retainDefinitionReceipt, sameDefinitionBasis, validateDefinitionPatches, validatePreparedDefinition, type DefinitionAuthorContext, type DefinitionPrepareRequest, type DefinitionPrepared, type DefinitionProposal, type DefinitionReceipt, type DefinitionRecord } from "./hosted-query-definition-state";
type Request = <T>(path: string, body: unknown) => Promise<T>;
export interface HostedDefinitionHost {
    authorInputs?: AuthorInputFactory;
    context(): DefinitionAuthorContext | null;
    prepareBasis(): Promise<EmbeddedQueryBasis>;
    apply(patches: SourcePatch[], basis: EmbeddedQueryBasis): Promise<void>;
    reservations(resource: string, receipts: DefinitionReceipt[], includeReserved?: boolean): void;
}
const digest = async (source: string): Promise<string> => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(value => value.toString(16).padStart(2, "0")).join("");
const statusText: Record<DefinitionReceipt["status"], UiText> = { reserved: uiMessage("definitionProposal.reservedStatus"), admitted: uiMessage("definitionProposal.admittedStatus"), cancelled: uiMessage("definitionProposal.cancelledStatus"), recoveryPending: uiMessage("definitionProposal.recoveryPendingStatus"), notRegistered: uiMessage("definitionProposal.notRegisteredStatus") };
/** The proposal cache retains recovery identity; the original source remains the only author buffer. */
export class HostedQueryDefinitions {
    readonly element = ui("section");
    private epoch = 0;
    private key = "";
    private label = "weftext-query-new";
    private records: DefinitionRecord[] = [];
    private selected = "";
    private busy = false;
    private ownerSession = "";
    private inputOwner = "";
    private inputSession?: AuthorInputSession;
    private ownerGeneration = 0;
    private operationGeneration = new Map<string, number>();
    private message = ui("p", "hint");
    private controls = ui("section");
    private preview = ui("section");
    private unreadable?: string;
    constructor(private request: Request, private author: HostedDefinitionHost, private plan: () => string, private eligible: (resource: string) => boolean) { this.element.append(ui("h3", "", uiMessage("definitionProposal.title")), this.message, this.controls, this.preview); }
    clear() { this.inputSession?.retire(); this.inputSession = undefined; this.inputOwner = ""; this.epoch++; this.ownerGeneration++; this.operationGeneration.clear(); this.key = ""; this.records = []; this.selected = ""; this.ownerSession = ""; this.busy = false; this.unreadable = undefined; this.label = "weftext-query-new"; this.controls.replaceChildren(); this.preview.replaceChildren(); setUiText(this.message, ""); }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear();
        this.key = hostedCachePrefix("query", deployment, actor, generation) + "definition-proposals";
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw) {
                const value = JSON.parse(raw) as {
                    label: string;
                    records: DefinitionRecord[];
                };
                if (typeof value.label !== "string" || !Array.isArray(value.records) || value.records.some(item => !item.operationId || !item.resource || item.request && (item.request.operationId !== item.operationId || item.request.resource !== item.resource)))
                    throw new Error();
                this.label = value.label;
                this.records = value.records.map(item => ({ ...item, receipt: undefined }));
                this.selected = this.records[0]?.operationId ?? "";
            }
        }
        catch {
            this.unreadable = raw ?? "";
        }
        this.render();
        const epoch = this.epoch;
        for (const record of [...this.records]) {
            try {
                await this.lookup(record);
            }
            catch (error) {
                if (epoch === this.epoch)
                    setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
            if (epoch !== this.epoch)
                return;
        }
        await this.authorChanged();
    }
    private persist(records = this.records, beforePrepare = false) {
        if (!this.key || this.unreadable !== undefined)
            throw new UiError(uiMessage("definitionProposal.cacheUnavailable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify({ label: this.label, records }));
        }
        catch {
            throw new UiError(beforePrepare ? uiMessage("definitionProposal.persistBeforeSendFailed") : uiMessage("definitionProposal.persistenceUnknown"));
        }
    }
    async authorChanged(force = false) {
        this.renderControls();
        this.renderPreview();
        const context = this.author.context(), ownerSession = context && this.eligible(context.resource) ? `${context.resource}:${context.sessionId}` : "";
        if (!ownerSession || !force && ownerSession === this.ownerSession)
            return;
        this.ownerSession = ownerSession;
        const epoch = this.epoch, generation = ++this.ownerGeneration;
        try {
            const response = await this.request<{
                resource: string;
                reservations: DefinitionReceipt[];
            }>("query/definition/reservations", { sessionId: context!.sessionId });
            if (epoch !== this.epoch || generation !== this.ownerGeneration || this.ownerSession !== ownerSession || this.author.context()?.sessionId !== context!.sessionId || this.author.context()?.baseId !== context!.baseId)
                return;
            if (response.resource !== context!.resource || !Array.isArray(response.reservations))
                throw new UiError(uiMessage("definitionProposal.ownerMismatch"));
            for (const receipt of response.reservations) {
                const existing = this.records.find(item => item.operationId === receipt.operationId);
                if (existing)
                    Object.assign(existing, retainDefinitionReceipt(existing, receipt));
                else
                    this.records.push(retainDefinitionReceipt({ operationId: receipt.operationId, resource: response.resource }, receipt));
            }
            // This list intentionally contains only pending reservations. Its absence
            // is not evidence of admission; query the exact IDs after every save/open.
            this.author.reservations(response.resource, response.reservations);
            this.persist();
            if (!this.selected)
                this.selected = this.records.find(item => item.resource === response.resource)?.operationId ?? "";
            this.render();
        }
        catch (error) {
            if (epoch === this.epoch && this.ownerSession === ownerSession)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
        }
        if (force) {
            const ids = new Set([...(context!.definitionReservations ?? []), ...this.records.filter(item => item.resource === context!.resource && (item.request || item.applied) && (!item.receipt || ["reserved", "recoveryPending", "notRegistered"].includes(item.receipt.status))).map(item => item.operationId)]);
            for (const id of ids) {
                if (epoch !== this.epoch || generation !== this.ownerGeneration || this.author.context()?.sessionId !== context!.sessionId)
                    return;
                let record = this.records.find(item => item.operationId === id);
                if (!record) {
                    record = { operationId: id, resource: context!.resource };
                    this.records.push(record);
                }
                try {
                    await this.lookup(record);
                }
                catch (error) {
                    if (epoch === this.epoch && generation === this.ownerGeneration)
                        setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
                }
            }
        }
    }
    private render() { this.renderControls(); this.renderPreview(); }
    private unknown(resource: string) { return this.records.some(record => record.resource === resource && (!record.receipt || ["notRegistered", "recoveryPending"].includes(record.receipt.status))); }
    private renderControls() {
        const active = document.activeElement, focused = active instanceof HTMLInputElement && active.dataset.controlRole === "definition-label", selection = focused ? { from: active.selectionStart, to: active.selectionEnd } : null;
        this.controls.replaceChildren();
        if (this.unreadable !== undefined) {
            this.controls.append(ui("p", "hint", uiMessage("definitionProposal.unreadableCache")), ui("pre", "", this.unreadable));
            return;
        }
        const context = this.author.context(), label = ui("input", "data-input");
        label.dataset.controlRole = "definition-label";
        setUiAttr(label, "aria-label", uiMessage("definitionProposal.labelInputLabel"));
        label.value = this.label;
        const inputOwner = context ? JSON.stringify([context.resource, context.sessionId]) : "";
        if (inputOwner !== this.inputOwner) {
            this.inputSession?.retire();
            this.inputSession = undefined;
            this.inputOwner = inputOwner;
        }
        if (context && !this.inputSession) {
            const epoch = this.epoch, selected = context;
            this.inputSession = this.author.authorInputs?.({ kind: "body", resource: context.resource }, () => this.epoch === epoch && this.author.context()?.sessionId === selected.sessionId && this.author.context()?.resource === selected.resource);
        }
        this.inputSession?.bind(label, "definition-label");
        label.oninput = () => {
            this.label = label.value;
            try {
                this.persist();
            }
            catch (error) {
                setUiText(this.message, errorUiText(error));
            }
        };
        const create = action(uiMessage("definitionProposal.promotePlan"), () => this.prepare());
        create.disabled = this.busy || !context?.writable || context.pending || context.compare || !this.eligible(context.resource) || this.unknown(context.resource) || (context.definitionReservations?.length ?? 0) >= 32;
        this.controls.append(ui("p", "hint", context ? displayMessage("definitionProposal.ownerHelp", { p0: uiDisplayParameter(context.resource) }) : uiMessage("definitionProposal.writableBodyRequired")), label, create);
        if (context && this.unknown(context.resource))
            this.controls.append(ui("p", "hint", uiMessage("definitionProposal.originalCheckRequired")));
        if (context)
            this.controls.append(ui("p", "hint", displayMessage("definitionProposal.nextSaveCount", { p0: context.definitionReservations?.length ?? 0 })));
        const select = ui("select", "data-input");
        setUiAttr(select, "aria-label", uiMessage("definitionProposal.pickerLabel"));
        select.append(Object.assign(ui("option", "", uiMessage("definitionProposal.selectPlaceholder")), { value: "" }));
        for (const record of this.records)
            select.append(Object.assign(ui("option", "", `${record.resource} · ${record.operationId}`), { value: record.operationId }));
        select.value = this.selected;
        select.onchange = () => {
            if (select.isConnected === false)
                return;
            this.selected = select.value;
            this.render();
        };
        this.controls.append(select);
        const record = this.records.find(item => item.operationId === this.selected), check = action(uiMessage("definitionProposal.inspectOriginal"), () => this.perform(() => this.lookup(record!))), retry = action(uiMessage("definitionProposal.retryOriginal"), () => this.perform(() => this.send(record!, false))), settle = action(uiMessage("definitionProposal.invalidateOriginal"), () => this.perform(async () => {
            const epoch = this.epoch, generation = this.beginOperation(record!);
            const receipt = await this.request<DefinitionReceipt>("query/definition/settle", { resource: record!.resource, operationId: record!.operationId });
            if (!this.currentOperation(record!, epoch, generation))
                return;
            this.install(record!, receipt);
            setUiText(this.message, statusText[receipt.status]);
        }));
        check.disabled = this.busy || !record;
        retry.disabled = this.busy || !record?.request || !!record.receipt && !["notRegistered", "reserved"].includes(record.receipt.status);
        settle.disabled = this.busy || !record || !!record.receipt && !["notRegistered", "reserved"].includes(record.receipt.status);
        this.controls.append(check, retry, settle, ui("p", "hint", uiMessage("definitionProposal.originalIdentityHelp")));
        if (focused) {
            label.focus({ preventScroll: true });
            if (selection?.from !== null && selection?.from !== undefined && selection.to !== null)
                label.setSelectionRange(selection.from, selection.to);
        }
    }
    private renderPreview() {
        this.preview.replaceChildren();
        const record = this.records.find(item => item.operationId === this.selected);
        if (!record)
            return;
        this.preview.append(ui("p", "hint", displayMessage("definitionProposal.originalSummary", { p0: uiDisplayParameter(record.operationId), p1: uiDisplayParameter(record.receipt ? statusText[record.receipt.status] : uiMessage("definitionProposal.uncheckedStatus")) })));
        if (record.receipt?.proposalUnavailable)
            this.preview.append(ui("p", "hint", uiMessage("definitionProposal.sourceUnavailable")));
        const proposal = record.receipt?.proposal;
        if (!proposal || !["reserved", "admitted"].includes(record.receipt!.status))
            return;
        this.preview.append(ui("p", "hint", displayMessage("definitionProposal.definitionKeySummary", { p0: uiDisplayParameter(proposal.definition.label), p1: uiDisplayParameter(proposal.definition.key) })));
        const details = ui("details");
        details.append(ui("summary", "", uiMessage("definitionProposal.compareDisclosure")), ui("h4", "", uiMessage("definitionProposal.originalBasisHeading")), ui("pre", "", proposal.basisSource ?? record.request?.source ?? uiMessage("definitionProposal.originalBasisUnavailable")), ui("h4", "", uiMessage("definitionProposal.proposalSourceHeading")), ui("pre", "", proposal.source));
        const context = this.author.context();
        if (context?.resource === record.resource)
            details.append(ui("h4", "", uiMessage("definitionProposal.currentDraftHeading")), ui("pre", "", context.source));
        this.preview.append(details);
        if (!context?.writable || context.resource !== record.resource || context.pending || context.compare || record.receipt!.status !== "reserved")
            return;
        if (context.source === proposal.source) {
            const link = action(uiMessage("definitionProposal.associateOriginal"), () => this.perform(async () => {
                const current = this.author.context();
                if (!current || current.resource !== record.resource || current.source !== proposal.source || current.pending || current.compare || !current.writable || (current.definitionReservations?.length ?? 0) >= 32)
                    throw new UiError(uiMessage("definitionProposal.associationBasisChanged"));
                this.author.reservations(record.resource, [record.receipt!], true);
                record.applied = true;
                this.persist();
                setUiText(this.message, uiMessage("definitionProposal.associatedUnsaved"));
            }));
            link.disabled = this.busy || (context.definitionReservations?.length ?? 0) >= 32 || !!context.definitionReservations?.includes(record.operationId);
            this.preview.append(ui("p", "hint", uiMessage("definitionProposal.proposalAlreadyPresent")), link);
            return;
        }
        const sameSource = context.source === (proposal.basisSource ?? record.request?.source), approval = ui("input");
        approval.type = "checkbox";
        setUiAttr(approval, "aria-label", uiMessage("definitionProposal.restoreApprovalLabel"));
        const label = ui("label", "data-label", sameSource ? uiMessage("definitionProposal.sameSourceApproval") : uiMessage("definitionProposal.replaceSourceApproval"));
        label.prepend(approval);
        const apply = action(sameSource ? uiMessage("definitionProposal.restorePatch") : uiMessage("definitionProposal.replaceDraft"), () => this.perform(async () => {
            const current = this.author.context();
            if (!approval.checked || !current || current.resource !== context.resource || current.sessionId !== context.sessionId || current.baseId !== context.baseId || current.source !== context.source || current.serial !== context.serial || current.inputId !== context.inputId || current.pending || current.compare || !current.writable)
                throw new UiError(uiMessage("definitionProposal.restoreBasisChanged"));
            const epoch = this.epoch, basis = await this.author.prepareBasis();
            if (epoch !== this.epoch || !this.records.includes(record))
                return;
            if (basis.source !== context.source || basis.sessionId !== context.sessionId || basis.serial !== context.serial || basis.inputId !== context.inputId)
                throw new UiError(uiMessage("definitionProposal.registrationBasisChanged"));
            const patches: SourcePatch[] = sameSource ? validateDefinitionPatches(context.source, proposal) : [{ from: 0, to: context.source.length, expected: context.source, insert: proposal.source }];
            if ((current.definitionReservations?.length ?? 0) >= 32 && !current.definitionReservations?.includes(record.operationId))
                throw new UiError(uiMessage("definitionProposal.restoreReservationLimit"));
            this.author.reservations(record.resource, [record.receipt!], true);
            await this.author.apply(patches, basis);
            if (epoch !== this.epoch)
                return;
            record.applied = true;
            this.persist();
            setUiText(this.message, uiMessage("definitionProposal.restoredUnsaved"));
        }));
        apply.disabled = true;
        approval.onchange = () => apply.disabled = !approval.checked || this.busy;
        this.preview.append(label, apply);
    }
    private async perform(run: () => Promise<void>) {
        if (this.busy)
            return;
        const epoch = this.epoch;
        this.busy = true;
        this.renderControls();
        try {
            await run();
        }
        catch (error) {
            if (epoch === this.epoch)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.render();
            }
        }
    }
    private async prepare() {
        await this.perform(async () => {
            const epoch = this.epoch, plan: QueryPlan = parseQueryPlan(this.plan()), label = this.label, before = this.author.context();
            if (!before?.writable || before.pending || before.compare || !this.eligible(before.resource) || this.unknown(before.resource) || (before.definitionReservations?.length ?? 0) >= 32)
                throw new UiError(uiMessage("definitionProposal.creationUnavailable"));
            const basis = await this.author.prepareBasis();
            const context = this.author.context();
            if (epoch !== this.epoch || !context || context.resource !== before.resource || context.baseId !== before.baseId || context.sessionId !== basis.sessionId || context.serial !== basis.serial || context.inputId !== basis.inputId || context.source !== basis.source)
                throw new UiError(uiMessage("definitionProposal.creationBasisChanged"));
            const sourceHash = await digest(context.source);
            if (epoch !== this.epoch || !this.author.context() || this.author.context()!.source !== context.source || this.author.context()!.serial !== context.serial || this.author.context()!.inputId !== context.inputId)
                throw new UiError(uiMessage("definitionProposal.sourceChangedBeforeReserve"));
            const operationId = crypto.randomUUID(), request: DefinitionPrepareRequest = { resource: context.resource, sessionId: context.sessionId, baseId: context.baseId, source: context.source, serial: context.serial, sourceHash, operationId, label, plan }, record: DefinitionRecord = { operationId, resource: context.resource, inputId: context.inputId, request };
            const records = [...this.records, record];
            this.persist(records, true);
            this.records = records;
            this.selected = operationId;
            this.render();
            await this.send(record, true);
        });
    }
    private async send(record: DefinitionRecord, allowApply: boolean) {
        const epoch = this.epoch, generation = this.beginOperation(record);
        if (!record.request)
            throw new UiError(uiMessage("definitionProposal.originalRequestMissing"));
        this.persist();
        const incoming = await this.request<DefinitionPrepared>("query/definition/prepare", record.request);
        if (!this.currentOperation(record, epoch, generation))
            return;
        if (incoming.status !== "reserved") {
            this.install(record, { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, key: incoming.key, binding: incoming.binding, saveOperationId: incoming.saveOperationId });
            setUiText(this.message, statusText[incoming.status]);
            return;
        }
        const proposal = validatePreparedDefinition(record.request, incoming);
        this.install(record, { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, key: incoming.key, binding: incoming.binding, saveOperationId: incoming.saveOperationId, proposal });
        if (allowApply && incoming.canApply === true && incoming.recoveryOnly === false && sameDefinitionBasis(this.author.context(), record.request, record.inputId)) {
            this.author.reservations(record.resource, [record.receipt!], true);
            await this.author.apply(validateDefinitionPatches(record.request.source, proposal), { ...record.request, inputId: record.inputId });
            if (!this.currentOperation(record, epoch, generation))
                return;
            record.applied = true;
            this.persist();
            setUiText(this.message, uiMessage("definitionProposal.insertedUnsaved"));
        }
        else
            setUiText(this.message, uiMessage("definitionProposal.originalCheckedNotApplied"));
    }
    private install(record: DefinitionRecord, receipt: DefinitionReceipt) { Object.assign(record, retainDefinitionReceipt(record, receipt)); this.author.reservations(record.resource, [receipt], record.applied === true); this.persist(); this.render(); }
    private beginOperation(record: DefinitionRecord) { const generation = (this.operationGeneration.get(record.operationId) ?? 0) + 1; this.operationGeneration.set(record.operationId, generation); return generation; }
    private currentOperation(record: DefinitionRecord, epoch: number, generation: number) { return epoch === this.epoch && this.records.includes(record) && this.operationGeneration.get(record.operationId) === generation; }
    private async lookup(record: DefinitionRecord) {
        const epoch = this.epoch, generation = this.beginOperation(record), receipt = await this.request<DefinitionReceipt>("query/definition/operation", { resource: record.resource, operationId: record.operationId });
        if (!this.currentOperation(record, epoch, generation))
            return;
        this.install(record, receipt);
        setUiText(this.message, statusText[receipt.status]);
    }
}
