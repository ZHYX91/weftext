import { ProposalCloseGuard } from "./proposal-close-guard";
import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody } from "./api-error";
import { submitOriginal, TerminatedAttempt, type OriginalFact } from "./original-operation";
import { message, displayMessage, setUiText, setUiAttr, uiDisplayParameter, uiTextValue, joinUiText, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui, diagnosticText, formatNativeValue, nativeMember, type DataSchema, type SchemaRead, type PanelHost, type NativeValue } from "./data-panel";
import type { OpenedDocument } from "./types";
import type { QueryRow } from "./query-panel";

function labeled(label: UiText, input: HTMLElement): HTMLElement { const element = ui("label", "query-control-label", label); element.append(input); return element; }
function selectControl(options: [string, UiText][], value: string): HTMLSelectElement { const input = ui("select", "data-input"); options.forEach(([key, label]) => { const option = ui("option", "", label); option.value = key; input.append(option); }); input.value = value; return input; }
const rowTitle = (row: QueryRow): UiText => row.cells.title?.state === "value" && row.cells.title.value ? formatNativeValue(row.cells.title.value) : row.path?.replaceAll("\\", "/").split("/").at(-1) || row.entityKey || row.key;

interface PeriodLookup {
    period: NativeValue;
    bounds: NativeValue;
    policy: {
        multiplicity: string;
        scope?: {
            kind?: string;
        };
    };
    matches: QueryRow[];
    coverage: "complete" | "partial";
    canCreate: boolean;
    cutId?: string;
    diagnostics?: unknown[];
}
/** Period creation is a separate explicit Core action after actual resolution. */
export class PeriodNavigator {
    private readonly closeGuard = new ProposalCloseGuard();
    private lookupDraft = JSON.stringify({ seriesKey: "", kind: "day", key: "" });
    private lookupBaseline = this.lookupDraft;
    private lookupMode: "key" | "date" = "date";
    private creationDraft = "";
    private creationBaseline = "";
    private unresolved(): boolean {
        return this.invalidDateInput() || this.pending !== null || this.lookupDraft !== this.lookupBaseline || this.creationDraft !== this.creationBaseline;
    }
    private invalidDateInput(): boolean { return this.date.validity?.badInput === true || this.date.validity?.valid === false; }
    assertLibraryDeparture(): void { this.closeGuard.assertMutable(); if (this.invalidDateInput()) throw new UiError(message("shell.closeAuthorInputPending")); }
    private canCreate(): boolean { return this.schemaAvailable && this.result?.coverage === "complete" && this.result.canCreate; }
    assertCloseReady(): void { this.closeGuard.assertReady(this.unresolved()); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.unresolved()); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    private lookupInput(): string { return JSON.stringify({ seriesKey: this.series.value, kind: this.kind.value, key: this.key.value, date: this.date.value, mode: this.key.value || this.date.value ? this.lookupMode : null }); }
    private creationInput(): string { return JSON.stringify({ parent: this.parent.value, name: this.name.value, title: this.title.value, body: this.body.value }); }
    readonly element = ui("details", "query-period-navigation");
    private readonly series = ui("input", "data-input");
    private readonly seriesChoices = ui("datalist");
    private policies: NonNullable<DataSchema["series_policies"]> | null = null;
    private schemaIdentity: string | null = null;
    private readonly kind = selectControl([["day", message("periodNavigation.day")], ["week", message("periodNavigation.week")], ["month", message("periodNavigation.month")], ["quarter", message("periodNavigation.quarter")], ["year", message("periodNavigation.year")]], "day");
    private readonly key = ui("input", "data-input");
    private readonly date = ui("input", "data-input");
    private readonly resultHost = ui("div", "period-lookup-result");
    private readonly parent = ui("input", "data-input");
    private readonly name = ui("input", "data-input");
    private readonly title = ui("input", "data-input");
    private readonly body = ui("textarea", "data-input");
    private readonly createButton: HTMLButtonElement;
    private result: PeriodLookup | null = null;
    private pending: Record<string, unknown> | null = null;
    private loading = false;
    private lookupGeneration = 0;
    private schemaAvailable = true;
    private readonly schemaMessage = ui("p", "data-warning");
    constructor(private readonly host: PanelHost, private root: string, scope: () => string) {
        this.closeGuard.bind(this.element);
        this.element.open = true;
        this.element.append(ui("summary", "", message("periodNavigation.disclosure")));
        setUiAttr(this.series, "placeholder", message("periodNavigation.seriesPlaceholder"));
        setUiAttr(this.series, "aria-label", message("periodNavigation.seriesLabel"));
        this.seriesChoices.id = `period-series-${crypto.randomUUID()}`;
        this.series.setAttribute("list", this.seriesChoices.id);
        setUiAttr(this.key, "placeholder", message("periodNavigation.keyPlaceholder"));
        setUiAttr(this.key, "aria-label", message("periodNavigation.keyLabel"));
        const navigation = ui("div", "period-navigation-form");
        navigation.append(labeled(message("periodNavigation.series"), this.series), this.seriesChoices, labeled(message("periodNavigation.kind"), this.kind));
        const dateLookup = ui("form", "period-date-form");
        this.date.type = "date";
        this.date.min = "0001-01-01";
        this.date.max = "9999-12-31";
        setUiAttr(this.date, "aria-label", message("periodCalendar.selectDate"));
        const selectDate = ui("button", "data-button", message("periodCalendar.dateLookup"));
        selectDate.type = "submit";
        dateLookup.append(labeled(message("periodCalendar.selectDate"), this.date), selectDate, action(message("periodCalendar.today"), () => {
            if (this.closeGuard.frozen || this.loading) return;
            const today = new Date();
            this.date.value = `${String(today.getFullYear()).padStart(4, "0")}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
            this.retainLookup("date");
            return this.lookup(0, true);
        }, "data-button compact"));
        dateLookup.onsubmit = event => { event.preventDefault(); void this.lookup(0, true); };
        navigation.append(dateLookup, action(message("periodNavigation.previous"), () => this.lookup(-1), "data-button compact"), action(message("periodNavigation.next"), () => this.lookup(1), "data-button compact"));
        const exact = ui("details", "period-key-lookup");
        exact.append(ui("summary", "", message("periodCalendar.exactKey")));
        const lookup = ui("form", "period-lookup-form");
        const submit = ui("button", "data-button", message("periodNavigation.lookup"));
        submit.type = "submit";
        lookup.append(labeled(message("periodNavigation.key"), this.key), submit);
        lookup.onsubmit = event => { event.preventDefault(); void this.lookup(0, false); };
        exact.append(lookup, ui("p", "data-help", message("periodNavigation.explicitPeriodHelp")));
        this.element.append(navigation, ui("p", "data-help", message("periodCalendar.dateHelp")), exact, this.schemaMessage, this.resultHost);
        const creation = ui("details", "period-create-form");
        creation.append(ui("summary", "", message("periodNavigation.createDisclosure")));
        this.parent.value = scope();
        setUiAttr(this.parent, "aria-label", message("periodNavigation.parentLabel"));
        setUiAttr(this.name, "placeholder", message("periodNavigation.namePlaceholder"));
        setUiAttr(this.name, "aria-label", message("periodNavigation.nameLabel"));
        setUiAttr(this.title, "aria-label", message("periodNavigation.titleLabel"));
        this.body.rows = 4;
        setUiAttr(this.body, "aria-label", message("periodNavigation.bodyLabel"));
        this.createButton = action(message("periodNavigation.createOpen"), () => this.create(), "data-button primary");
        this.createButton.disabled = true;
        creation.append(labeled(message("periodNavigation.parent"), this.parent), labeled(message("periodNavigation.requiredName"), this.name), labeled(message("periodNavigation.optionalTitle"), this.title), labeled(message("periodNavigation.nativeBody"), this.body), this.createButton, ui("p", "data-help", message("periodNavigation.uniqueSeriesHelp")));
        this.element.append(creation);
        // Authored raw values live in this owner even when validation rejects them.
        this.lookupBaseline = this.lookupDraft = this.lookupInput();
        this.creationBaseline = this.creationDraft = this.creationInput();
        const retainCreation = (): void => {
            if (!this.closeGuard.frozen) this.creationDraft = this.creationInput();
        };
        for (const input of [this.series, this.key, this.kind, this.date]) {
            const mode = input === this.date ? "date" : input === this.key ? "key" : undefined;
            input.addEventListener("input", () => this.retainLookup(mode));
            input.addEventListener("change", () => this.retainLookup(mode));
        }
        for (const input of [this.parent, this.name, this.title, this.body]) {
            input.addEventListener("input", retainCreation);
            input.addEventListener("change", retainCreation);
        }
    }
    private retainLookup(mode?: "key" | "date"): void {
        if (this.closeGuard.frozen) return;
        if (mode) this.lookupMode = mode;
        this.lookupDraft = this.lookupInput();
        this.lookupGeneration++;
        this.result = null;
        this.createButton.disabled = this.loading || !this.pending;
        this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.changedLookup")));
        this.updatePolicy();
    }
    private updatePolicy(): void {
        if (!this.schemaAvailable) {
            this.schemaMessage.hidden = false;
            setUiText(this.schemaMessage, message("periodNavigation.schemaUnavailable"));
            return;
        }
        const policy = this.policies?.find(policy => policy.key === this.series.value);
        const warning = this.policies === null ? null : !this.policies.length ? message("periodCalendar.noSeries") : !policy ? message("periodCalendar.chooseSeries") : !policy.kinds.includes(this.kind.value) ? message("periodCalendar.kindUnavailable") : null;
        this.schemaMessage.hidden = warning === null;
        setUiText(this.schemaMessage, warning ?? "");
        for (const option of this.kind.children as HTMLCollectionOf<HTMLOptionElement>) option.disabled = !!policy && !policy.kinds.includes(option.value);
    }
    setSchemaAvailable(available: boolean): void {
        if (this.closeGuard.frozen) return;
        if (this.schemaAvailable !== available) { this.lookupGeneration++; this.result = null; }
        this.schemaAvailable = available;
        this.createButton.disabled = this.loading || !this.pending && !this.canCreate();
        this.updatePolicy();
    }
    setSchema(schema: DataSchema): void {
        if (this.closeGuard.frozen) return;
        const policies = schema.series_policies ?? [];
        const identity = JSON.stringify(schema);
        if (this.schemaIdentity !== identity) {
            this.lookupGeneration++;
            this.result = null;
            this.createButton.disabled = !this.pending;
            this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.changedLookup")));
        }
        this.schemaIdentity = identity;
        this.policies = policies;
        this.seriesChoices.replaceChildren(...policies.map(policy => { const option = ui("option"); option.value = policy.key; return option; }));
        this.setSchemaAvailable(schema.fields.some(field => field.id === "calendar/period" && field.key === "period"));
    }
    invalidate(): void {
        if (this.closeGuard.frozen) return;
        this.lookupGeneration++;
        this.result = null;
        this.createButton.disabled = this.loading || !this.pending;
        this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.changedLookup")));
    }
    private async lookup(steps = 0, fromDate = this.lookupMode === "date"): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            if (!this.schemaAvailable) {
                this.host.notify(message("periodNavigation.definitionUnavailable"), true);
                return;
            }
            if (this.loading)
                return;
            if (this.policies !== null && !this.policies.some(policy => policy.key === this.series.value && policy.kinds.includes(this.kind.value))) {
                this.updatePolicy();
                return;
            }
            if (!this.series.value || !(fromDate ? this.date.value : this.key.value)) {
                this.host.notify(message(fromDate ? "periodCalendar.dateRequired" : "periodNavigation.exactKeyRequired"), true);
                return;
            }
            const generation = ++this.lookupGeneration;
            const basis = this.lookupInput();
            const request = { root: this.root, seriesKey: this.series.value, kind: this.kind.value, ...(fromDate ? { date: this.date.value } : { key: this.key.value }), ...(steps ? { steps } : {}) };
            this.loading = true;
            this.createButton.disabled = true;
            this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.resolving")));
            try {
                const result = await this.host.request<PeriodLookup>("/api/data/period/resolve", request);
                if (this.closeGuard.frozen || generation !== this.lookupGeneration || this.lookupInput() !== basis)
                    return;
                this.result = result;
                const actualKey = nativeMember(result.period, "key");
                if (actualKey?.kind === "text") {
                    this.key.value = actualKey.value;
                    this.lookupMode = "key";
                }
                this.lookupBaseline = this.lookupDraft = this.lookupInput();
                const start = nativeMember(result.bounds, "start"), end = nativeMember(result.bounds, "end_exclusive");
                const kind = this.kind.value as "day" | "week" | "month" | "quarter" | "year";
                this.resultHost.replaceChildren(ui("h3", "", displayMessage("periodCalendar.identity", { p0: this.series.value, p1: uiDisplayParameter(message(`periodNavigation.${kind}`)), p2: actualKey?.kind === "text" ? actualKey.value : formatNativeValue(result.period) })), ui("p", "data-help", start?.kind === "date" && end?.kind === "date" ? displayMessage("periodCalendar.range", { p0: start.value, p1: end.value }) : displayMessage("periodNavigation.resolvedBounds", { p0: uiDisplayParameter(formatNativeValue(result.bounds)) })), ui("p", result.coverage === "complete" ? "query-result-status" : "data-warning", displayMessage("periodNavigation.lookupSummary", { p0: uiDisplayParameter(result.coverage === "complete" ? message("periodNavigation.completeFrozenScope") : message("periodNavigation.partialScope")), p1: result.matches.length, p2: uiDisplayParameter(result.policy.multiplicity === "unique" ? message("periodNavigation.uniqueSeries") : message("periodNavigation.multipleSeries")) })));
                for (const row of result.matches)
                    this.resultHost.append(action(rowTitle(row), async () => {
                        if (this.closeGuard.frozen) return;
                        const endOpen = this.closeGuard.begin();
                        try {
                            await this.host.prepareAction?.();
                            if (this.closeGuard.frozen) return;
                            if (row.entityKey) {
                                if (!this.host.openResolved)
                                    throw new UiError(message("queryValidation.stableOpenRequired"));
                                const opened = await this.host.request<OpenedDocument>("/api/data/open", { root: this.root, entityKey: row.entityKey });
                                if (this.closeGuard.frozen) return;
                                await this.host.openResolved(opened);
                            }
                            else if (row.path)
                                await this.host.openDocument(row.path, this.root);
                            else
                                throw new UiError(message("periodNavigation.existingTargetUnavailable"));
                        }
                        catch (error) {
                            if (!this.closeGuard.frozen) this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
                        }
                        finally { endOpen(); }
                    }, "query-row-title"));
                result.diagnostics?.forEach(item => this.resultHost.append(ui("p", "data-warning", diagnosticText(item))));
                if (!result.matches.length)
                    this.resultHost.append(ui("p", "data-help", result.coverage === "complete" ? message("periodNavigation.completeNoMatch") : message("periodNavigation.partialNoMatch")));
                this.createButton.disabled = !this.canCreate() && !this.pending;
            }
            catch (error) {
                if (!this.closeGuard.frozen && generation === this.lookupGeneration) {
                    this.result = null;
                    this.resultHost.replaceChildren(ui("p", "data-warning", error instanceof Error ? errorUiText(error) : message("error.invalidResponse")));
                    this.createButton.disabled = !this.pending;
                }
            }
            finally {
                if (!this.closeGuard.frozen) {
                    this.loading = false;
                    this.createButton.disabled = !this.pending && !this.canCreate();
                }
            }
        } finally { endClose(); }
    }
    private async create(): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            if (!this.schemaAvailable && !this.pending) {
                this.host.notify(message("periodNavigation.definitionUnavailable"), true);
                return;
            }
            if ((!this.canCreate() && !this.pending) || this.loading)
                return;
        if (!this.pending && (!this.name.value || !this.parent.value)) {
                this.host.notify(message("periodNavigation.creationLocationRequired"), true);
                return;
            }
            const retry = this.pending !== null;
            const actualKey = this.result ? nativeMember(this.result.period, "key") : undefined;
            this.pending ??= { root: this.root, parent: this.parent.value, name: this.name.value, seriesKey: this.series.value, kind: this.kind.value, key: actualKey?.kind === "text" ? actualKey.value : this.key.value, title: this.title.value || undefined, body: this.body.value, operationId: crypto.randomUUID() };
            const submitted = this.pending;
            const lookupAtSubmission = this.lookupInput();
            let receiptReceived = false;
            this.loading = true;
            this.createButton.disabled = true;
            try {
                const result = await submitOriginal<OriginalFact & {
                    document?: OpenedDocument;
                    documentError?: ApiErrorBody;
                }>(this.host.request, "/api/data/period/create", this.pending, "period", retry);
                if (this.closeGuard.frozen) return;
                receiptReceived = true;
                if (result.status !== "committed")
                    throw new UiError(message("queryDefinition.periodCommittedReceiptRequired"));
                this.host.environmentChanged?.(this.root);
                const document = result.document;
                const documentError = result.documentError === undefined ? undefined : decodeApiErrorBody(result.documentError);
                if (result.documentError !== undefined && !documentError) throw new UiError(message("error.invalidResponse"));
                const deliveryError = result.deliveryError === undefined ? undefined : decodeApiErrorBody(result.deliveryError);
                if (result.deliveryError !== undefined && !deliveryError) throw new UiError(message("error.invalidResponse"));
                const deliveryErrors = [documentError, deliveryError].filter((error): error is ApiErrorBody => !!error);
                if (deliveryErrors.length || !document)
                    throw new UiError(displayMessage("periodNavigation.submittedOpenPending", { p0: uiDisplayParameter(deliveryErrors.length ? joinUiText(deliveryErrors.map(apiErrorUiText), "\n") : message("common.confirmOriginalOperation")) }));
                if (!this.host.created)
                    throw new UiError(message("queryDefinition.periodDocumentDeliveryRequired"));
                await this.host.created(document, result.operationId);
                if (this.closeGuard.frozen) return;
                const unchanged = this.name.value === submitted.name && this.parent.value === submitted.parent && (this.title.value || undefined) === submitted.title && this.body.value === submitted.body && this.series.value === submitted.seriesKey && this.kind.value === submitted.kind && this.key.value === submitted.key && this.lookupMode === "key";
                this.pending = null;
                if (unchanged) {
                    this.name.value = "";
                    this.title.value = "";
                    this.body.value = "";
                    this.creationBaseline = this.creationDraft = this.creationInput();
                }
                this.host.notify(unchanged ? message("periodNavigation.createdOpened") : message("periodNavigation.createdWithNewInput"));
                this.loading = false;
                if (unchanged && this.lookupInput() === lookupAtSubmission) await this.lookup();
                else this.invalidate();
            }
            catch (error) {
                if (this.closeGuard.frozen) return;
                if (!receiptReceived && error instanceof TerminatedAttempt) {
                    this.pending = null;
                    this.result = null;
                }
                const hint = this.pending ? displayMessage("periodNavigation.originalOperationRetained", { p0: String(this.pending.seriesKey), p1: String(this.pending.key) }) : message("periodNavigation.lookupBeforeCreating");
                this.resultHost.append(ui("p", "data-warning", () => `${uiTextValue(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"))}${uiTextValue(hint)}`));
            }
            finally {
                if (!this.closeGuard.frozen) {
                    this.loading = false;
                    this.createButton.disabled = !this.pending && !this.canCreate();
                    setUiText(this.createButton, this.pending ? message("periodNavigation.confirmPreviousCreation") : message("periodNavigation.createOpen"));
                }
            }
        } finally { endClose(); }
    }
}

interface CalendarLibrary {
    root: string;
    workspaceId: string;
    scope: string | null;
    navigator: PeriodNavigator;
    epoch: number;
}

/** One retained periodic-note owner per library, independent of Query and dock geometry. */
export class CalendarPanel {
    readonly element = ui("section", "calendar-period-panel");
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly status = ui("p", "data-help", message("periodCalendar.noLibrary"));
    private readonly content = ui("div", "calendar-period-content");
    private readonly libraries = new Map<string, CalendarLibrary>();
    private readonly reads = new Map<CalendarLibrary, { epoch: number; promise: Promise<void> }>();
    private current: CalendarLibrary | null = null;

    constructor(private readonly host: PanelHost) {
        this.closeGuard.bind(this.element);
        this.element.append(ui("h2", "", message("workspaceViews.calendar")), action(message("common.refresh"), () => this.refresh()), this.status, this.content);
    }
    assertCloseReady(): void { this.closeGuard.assertReady(false); for (const library of this.libraries.values()) library.navigator.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        const frozen: PeriodNavigator[] = [];
        try {
            for (const library of this.libraries.values()) {
                frozen.push(library.navigator);
                if (!library.navigator.freezeForClose()) throw new Error("Calendar proposal owner refused close");
            }
            if (!this.closeGuard.freeze(false)) throw new Error("Calendar read changed during close");
            return true;
        } catch {
            for (const navigator of frozen.reverse()) navigator.thawAfterClose();
            this.closeGuard.thaw();
            return false;
        }
    }
    thawAfterClose(): void { this.closeGuard.thaw(); for (const library of this.libraries.values()) library.navigator.thawAfterClose(); }
    assertLibraryDeparture(): void { this.closeGuard.assertMutable(); this.current?.navigator.assertLibraryDeparture(); }
    assertLibraryChange(library: { root: string; workspaceId?: string } | null): void {
        this.closeGuard.assertMutable();
        if (this.current?.root !== library?.root || this.current?.workspaceId !== library?.workspaceId) this.assertLibraryDeparture();
    }
    setLibrary(library: { root: string; workspaceId?: string } | null, scope: string | null): void {
        this.assertLibraryChange(library);
        if (!library?.workspaceId) {
            this.current = null;
            this.content.replaceChildren();
            this.status.hidden = false;
            setUiText(this.status, message("periodCalendar.noLibrary"));
            return;
        }
        const identity = JSON.stringify([library.root, library.workspaceId]);
        let state = this.libraries.get(identity);
        if (!state) {
            const root = library.root;
            const creationScope = (): string => { const directory = state?.scope ?? scope; return directory && !directory.startsWith("weftext-") ? directory : root; };
            state = { root, workspaceId: library.workspaceId, scope, epoch: 0, navigator: new PeriodNavigator(this.host, root, creationScope) };
            state.navigator.setSchemaAvailable(false);
            this.libraries.set(identity, state);
        }
        state.scope = scope;
        const changed = this.current !== state;
        this.current = state;
        if (this.content.firstChild !== state.navigator.element) this.content.replaceChildren(state.navigator.element);
        if (changed) void this.refresh();
    }
    environmentChanged(root: string): void {
        if (this.closeGuard.frozen) return;
        for (const state of this.libraries.values()) if (state.root === root) { state.epoch++; state.navigator.invalidate(); }
        if (this.current?.root === root) void this.refresh();
    }
    async refresh(): Promise<void> {
        if (this.closeGuard.frozen || !this.current) return;
        const state = this.current;
        const running = this.reads.get(state);
        if (running) {
            await running.promise;
            if (state === this.current && running.epoch !== state.epoch) await this.refresh();
            return;
        }
        const epoch = state.epoch;
        const end = this.closeGuard.begin();
        this.status.hidden = false;
        setUiText(this.status, message("periodCalendar.schemaLoading"));
        const promise = Promise.resolve().then(async () => {
            try {
                const result = await this.host.request<SchemaRead>("/api/data/schema", { root: state.root });
                if (this.closeGuard.frozen || epoch !== state.epoch) return;
                state.navigator.setSchema(result.schema);
                if (state === this.current) this.status.hidden = true;
            } catch (error) {
                if (this.closeGuard.frozen || epoch !== state.epoch) return;
                state.navigator.setSchemaAvailable(false);
                if (state === this.current) { this.status.hidden = false; setUiText(this.status, errorUiText(error)); }
            } finally { this.reads.delete(state); end(); }
        });
        this.reads.set(state, { epoch, promise });
        await promise;
    }
}

