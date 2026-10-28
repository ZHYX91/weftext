import { ProposalCloseGuard } from "./proposal-close-guard";
import { readNoteProjection, sameProjectionInput } from "./data-projection-read";
import { joinUiText } from "./ui-locale";
import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody } from "./api-error";
import type { UiMessage } from "./ui-locale";
import type { CoreDescription } from "./ui-diagnostics";
import { decodeLocalQueryResult, queryDiagnostics, type QueryEngineGroup, type QueryProductError } from "./query-result";
import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { submitOriginal, TerminatedAttempt, type OriginalFact } from "./original-operation";
import { action, cellLabels, decodeSourcePatch, diagnosticText, fieldNames, formatNativeValue, nativeMember, nativeFormControls, QuickCapture, resolveShape, scalarFromInput, typeNames, ui, unionVariant, valueEditor, loadTagFormReferences, type FormReferences, type Cell, type DataSchema, type DocumentBasis, type NativeValue, type PanelHost, type Shape, type SourcePatch, type SchemaField } from "./data-panel";
import { mountListThumbnail, type ListPreview } from "./list-thumbnail";
import type { OpenedDocument } from "./types";
import type { FrozenQuery } from "./export-state";
import { TagTree, tagPredicate, decodeTagShortcutCollection, type TagMatchMode } from "./tag-tree";
import { tagPicker } from "./tag-picker";
import { tagCancellationStatus, tagOperationStatus } from "./tag-navigation-state";
import { QueryRefreshQueue, queryRefreshKey } from "./query-refresh";
import { updateQueryChoices, type QueryChoice } from "./query-control-state";
export interface QueryPlan {
    scope: {
        directory?: string;
        descendants: boolean;
        lifecycle: "live" | "trash";
    };
    types: string[];
    where?: string;
    select: {
        name: string;
        expr: string;
    }[];
    order: {
        expr: string;
        descending: boolean;
        nulls: "first" | "last";
    }[];
    take?: number;
    group?: {
        keys: {
            name: string;
            expr: string;
        }[];
        aggregates: {
            name: string;
            op: "count" | "sum" | "min" | "max";
            expr?: string;
        }[];
    };
    unnest?: {
        array: string;
        binding: string;
    };
    advanced?: {
        parameters?: {
            name: string;
            shape: Shape;
            nullable?: boolean;
            default?: NativeValue;
        }[];
        unionAll?: QueryPlan[];
        distinct?: {
            name: string;
            expr: string;
        }[];
        related?: {
            relation: string;
            from: QueryPlan;
            direction?: "forward" | "inverse";
        };
        call?: {
            owner: string;
            selection: {
                mode: "current";
                key: string;
            } | {
                mode: "pinned";
                key: string;
                revision: string;
            };
            arguments?: Record<string, string>;
        };
    };
}
/** Reject duplicate JSON members before parsing: a plan editor must not silently choose the last value. */
export function parseQueryJson(source: string): unknown {
    const parsed: unknown = JSON.parse(source);
    let position = 0;
    const whitespace = (): void => {
        while (/\s/.test(source[position] ?? "") && position < source.length)
            position++;
    };
    const string = (): string => {
        const start = position++;
        while (position < source.length) {
            const char = source[position++];
            if (char === "\\")
                position++;
            else if (char === '"')
                return JSON.parse(source.slice(start, position)) as string;
        }
        throw new UiError(message("queryValidation.incompleteJsonString"));
    };
    const value = (depth: number): void => {
        if (depth > 100)
            throw new UiError(message("queryValidation.jsonNestingExceeded"));
        whitespace();
        const char = source[position];
        if (char === '"') {
            string();
            return;
        }
        if (char === "{") {
            position++;
            whitespace();
            const keys = new Set<string>();
            if (source[position] === "}") {
                position++;
                return;
            }
            for (;;) {
                whitespace();
                const key = string();
                if (keys.has(key))
                    throw new UiError(displayMessage("queryValidation.duplicateJsonMember", { p0: uiDisplayParameter(key) }));
                keys.add(key);
                whitespace();
                position++;
                value(depth + 1);
                whitespace();
                if (source[position++] === "}")
                    return;
            }
        }
        if (char === "[") {
            position++;
            whitespace();
            if (source[position] === "]") {
                position++;
                return;
            }
            for (;;) {
                value(depth + 1);
                whitespace();
                if (source[position++] === "]")
                    return;
            }
        }
        while (position < source.length && !/[\s,}\]]/.test(source[position]))
            position++;
    };
    value(0);
    return parsed;
}
/** Structural checks only; the shared compiler owns CEL, schema and producer validation. */
export function parseQueryPlan(source: string): QueryPlan {
    const parsed = parseQueryJson(source);
    let nodes = 0;
    const visit = (value: unknown, depth: number): QueryPlan => {
        if (++nodes > 1024 || depth > 64)
            throw new UiError(message("queryValidation.planBudgetExceeded"));
        if (!value || typeof value !== "object" || Array.isArray(value))
            throw new UiError(message("queryValidation.planObjectRequired"));
        const plan = value as Record<string, unknown>;
        const allowed = ["scope", "types", "where", "select", "order", "take", "group", "unnest", "advanced"];
        const unknown = Object.keys(plan).find(key => !allowed.includes(key));
        if (unknown)
            throw new UiError(displayMessage("queryValidation.unknownPlanMember", { p0: uiDisplayParameter(unknown) }));
        const scope = plan.scope ?? { descendants: true, lifecycle: "live" };
        if (!scope || typeof scope !== "object" || Array.isArray(scope))
            throw new UiError(message("queryValidation.scopeObjectRequired"));
        const actualScope = scope as Record<string, unknown>;
        if (Object.keys(actualScope).some(key => !["directory", "descendants", "lifecycle"].includes(key)) || actualScope.directory != null && typeof actualScope.directory !== "string" || actualScope.descendants != null && typeof actualScope.descendants !== "boolean" || actualScope.lifecycle != null && !["live", "trash"].includes(String(actualScope.lifecycle)))
            throw new UiError(message("queryValidation.invalidScope"));
        if (plan.types != null && (!Array.isArray(plan.types) || plan.types.some(item => typeof item !== "string")))
            throw new UiError(message("queryValidation.typeArrayRequired"));
        for (const key of ["select", "order"])
            if (plan[key] != null && (!Array.isArray(plan[key]) || (plan[key] as unknown[]).some(item => !item || typeof item !== "object" || typeof (item as Record<string, unknown>).expr !== "string")))
                throw new UiError(displayMessage("queryValidation.expressionArrayRequired", { p0: uiDisplayParameter(key) }));
        if (plan.where != null && typeof plan.where !== "string" || plan.take != null && (!Number.isSafeInteger(plan.take) || Number(plan.take) < 1))
            throw new UiError(message("queryValidation.invalidWhereOrTake"));
        const advanced = plan.advanced as QueryPlan["advanced"];
        if (advanced != null) {
            if (typeof advanced !== "object" || Array.isArray(advanced))
                throw new UiError(message("queryValidation.advancedObjectRequired"));
            if (advanced.unionAll) {
                if (!Array.isArray(advanced.unionAll) || !advanced.unionAll.length)
                    throw new UiError(message("queryValidation.unionSourcesRequired"));
                advanced.unionAll.forEach(branch => visit(branch, depth + 1));
            }
            if (advanced.related)
                visit(advanced.related.from, depth + 1);
        }
        return { ...plan, scope: { ...actualScope, descendants: actualScope.descendants ?? true, lifecycle: actualScope.lifecycle ?? "live" }, types: plan.types ?? [], select: plan.select ?? [], order: plan.order ?? [] } as QueryPlan;
    };
    return visit(parsed, 0);
}
export function parseQueryParameters(source: string): Record<string, NativeValue> {
    const parsed = parseQueryJson(source);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
        throw new UiError(message("queryValidation.parameterObjectRequired"));
    const check = (value: unknown, depth = 0): void => {
        if (depth > 64 || !value || typeof value !== "object" || Array.isArray(value))
            throw new UiError(message("queryValidation.invalidNativeParameter"));
        const native = value as Record<string, unknown>;
        if (native.kind === "none" && Object.keys(native).length === 1)
            return;
        if (native.kind === "array" && Array.isArray(native.value)) {
            native.value.forEach(item => check(item, depth + 1));
            return;
        }
        if (native.kind === "record" && Array.isArray(native.value)) {
            const keys = new Set<string>();
            for (const item of native.value) {
                if (!item || typeof item.key !== "string" || keys.has(item.key))
                    throw new UiError(message("queryValidation.invalidRecordParameter"));
                keys.add(item.key);
                check(item.value, depth + 1);
            }
            return;
        }
        if (native.kind === "bool" && typeof native.value === "boolean" || ["text", "int", "float", "decimal", "date", "time", "localDatetime", "duration"].includes(String(native.kind)) && typeof native.value === "string")
            return;
        throw new UiError(message("queryValidation.nativeParameterKindRequired"));
    };
    Object.values(parsed).forEach(value => check(value));
    return parsed as Record<string, NativeValue>;
}
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
    // query-child-close-ownership-v1
    private readonly closeGuard = new ProposalCloseGuard();
    private lookupDraft = JSON.stringify({ seriesKey: "", kind: "day", key: "" });
    private lookupBaseline = this.lookupDraft;
    private creationDraft = "";
    private creationBaseline = "";
    private unresolved(): boolean {
        return this.pending !== null || this.lookupDraft !== this.lookupBaseline || this.creationDraft !== this.creationBaseline;
    }
    assertCloseReady(): void { this.closeGuard.assertReady(this.unresolved()); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.unresolved()); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    private lookupInput(): string { return JSON.stringify({ seriesKey: this.series.value, kind: this.kind.value, key: this.key.value }); }
    private creationInput(): string { return JSON.stringify({ parent: this.parent.value, name: this.name.value, title: this.title.value, body: this.body.value }); }
    readonly element = ui("details", "query-period-navigation");
    private readonly series = ui("input", "data-input");
    private readonly kind = selectControl([["day", message("periodNavigation.day")], ["week", message("periodNavigation.week")], ["month", message("periodNavigation.month")], ["quarter", message("periodNavigation.quarter")], ["year", message("periodNavigation.year")]], "day");
    private readonly key = ui("input", "data-input");
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
        this.element.append(ui("summary", "", message("periodNavigation.disclosure")));
        setUiAttr(this.series, "placeholder", message("periodNavigation.seriesPlaceholder"));
        setUiAttr(this.series, "aria-label", message("periodNavigation.seriesLabel"));
        setUiAttr(this.key, "placeholder", message("periodNavigation.keyPlaceholder"));
        setUiAttr(this.key, "aria-label", message("periodNavigation.keyLabel"));
        const lookup = ui("form", "period-lookup-form");
        const submit = ui("button", "data-button", message("periodNavigation.lookup"));
        submit.type = "submit";
        lookup.append(labeled(message("periodNavigation.series"), this.series), labeled(message("periodNavigation.kind"), this.kind), labeled(message("periodNavigation.key"), this.key), submit, action(message("periodNavigation.previous"), () => this.lookup(-1), "data-button compact"), action(message("periodNavigation.next"), () => this.lookup(1), "data-button compact"));
        lookup.onsubmit = event => { event.preventDefault(); void this.lookup(); };
        this.element.append(lookup, ui("p", "data-help", message("periodNavigation.explicitPeriodHelp")), this.schemaMessage, this.resultHost);
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
        const retainLookup = (): void => {
            if (this.closeGuard.frozen) return;
            this.lookupDraft = this.lookupInput();
            this.lookupGeneration++;
            this.result = null;
            this.createButton.disabled = this.loading || !this.pending;
            this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.changedLookup")));
        };
        const retainCreation = (): void => {
            if (!this.closeGuard.frozen) this.creationDraft = this.creationInput();
        };
        for (const input of [this.series, this.key, this.kind]) {
            input.addEventListener("input", retainLookup);
            input.addEventListener("change", retainLookup);
        }
        for (const input of [this.parent, this.name, this.title, this.body]) {
            input.addEventListener("input", retainCreation);
            input.addEventListener("change", retainCreation);
        }
    }
    setSchemaAvailable(available: boolean): void { if (this.closeGuard.frozen) return; this.schemaAvailable = available; this.schemaMessage.hidden = available; setUiText(this.schemaMessage, available ? "" : message("periodNavigation.schemaUnavailable")); }
    private async lookup(steps = 0): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            if (!this.schemaAvailable) {
                this.host.notify(message("periodNavigation.definitionUnavailable"), true);
                return;
            }
            if (this.loading)
                return;
            if (!this.series.value || !this.key.value) {
                this.host.notify(message("periodNavigation.exactKeyRequired"), true);
                return;
            }
            const generation = ++this.lookupGeneration;
            const basis = { seriesKey: this.series.value, kind: this.kind.value, key: this.key.value };
            this.loading = true;
            this.createButton.disabled = true;
            this.resultHost.replaceChildren(ui("p", "data-help", message("periodNavigation.resolving")));
            try {
                const result = await this.host.request<PeriodLookup>("/api/data/period/resolve", { root: this.root, ...basis, ...(steps ? { steps } : {}) });
                if (this.closeGuard.frozen || generation !== this.lookupGeneration || this.series.value !== basis.seriesKey || this.kind.value !== basis.kind || this.key.value !== basis.key)
                    return;
                this.result = result;
                const actualKey = nativeMember(result.period, "key");
                if (actualKey?.kind === "text")
                    this.key.value = actualKey.value;
                this.lookupBaseline = this.lookupDraft = this.lookupInput();
                this.resultHost.replaceChildren(ui("p", "data-help", displayMessage("periodNavigation.resolvedPeriod", { p0: uiDisplayParameter(formatNativeValue(result.period)) })), ui("p", "data-help", displayMessage("periodNavigation.resolvedBounds", { p0: uiDisplayParameter(formatNativeValue(result.bounds)) })), ui("p", result.coverage === "complete" ? "query-result-status" : "data-warning", displayMessage("periodNavigation.lookupSummary", { p0: uiDisplayParameter(result.coverage === "complete" ? message("periodNavigation.completeFrozenScope") : message("periodNavigation.partialScope")), p1: result.matches.length, p2: uiDisplayParameter(result.policy.multiplicity === "unique" ? message("periodNavigation.uniqueSeries") : message("periodNavigation.multipleSeries")) })));
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
                this.createButton.disabled = !result.canCreate && !this.pending;
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
                    this.createButton.disabled = !this.pending && !this.result?.canCreate;
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
            if ((!this.result?.canCreate && !this.pending) || this.loading)
                return;
        if (!this.pending && (!this.name.value || !this.parent.value)) {
                this.host.notify(message("periodNavigation.creationLocationRequired"), true);
                return;
            }
            const retry = this.pending !== null;
            const actualKey = this.result ? nativeMember(this.result.period, "key") : undefined;
            this.pending ??= { root: this.root, parent: this.parent.value, name: this.name.value, seriesKey: this.series.value, kind: this.kind.value, key: actualKey?.kind === "text" ? actualKey.value : this.key.value, title: this.title.value || undefined, body: this.body.value, operationId: crypto.randomUUID() };
            const submitted = this.pending;
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
                const unchanged = this.name.value === submitted.name && this.parent.value === submitted.parent && (this.title.value || undefined) === submitted.title && this.body.value === submitted.body && this.series.value === submitted.seriesKey && this.kind.value === submitted.kind && this.key.value === submitted.key;
                this.pending = null;
                if (unchanged) {
                    this.name.value = "";
                    this.title.value = "";
                    this.body.value = "";
                    this.creationBaseline = this.creationDraft = this.creationInput();
                }
                this.host.notify(unchanged ? message("periodNavigation.createdOpened") : message("periodNavigation.createdWithNewInput"));
                this.loading = false;
                await this.lookup();
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
                    this.createButton.disabled = !this.pending && !this.result?.canCreate;
                    setUiText(this.createButton, this.pending ? message("periodNavigation.confirmPreviousCreation") : message("periodNavigation.createOpen"));
                }
            }
        } finally { endClose(); }
    }
}
interface TagRegistry {
    tags: {
        tagRef: string;
        tagId: string;
        displayName: string;
        parentTagId: string | null;
    }[];
    baseId: string;
    complete: boolean;
    warnings?: unknown[];
}
interface NavigationPreparation {
    targetRef: string;
    preview: {
        operationId: string;
        planId: string;
        kind?: string;
        changes?: unknown[];
    };
}
export class NavigationControls {
    // query-child-close-ownership-v1
    private readonly closeGuard = new ProposalCloseGuard();
    private createNameDraft = "";
    private appliedParentRef: string | null = null;
    private unresolved(): boolean {
        return this.pending !== null || this.recoveryError !== undefined || this.createNameDraft !== "" || this.parentRef !== this.appliedParentRef;
    }
    assertCloseReady(): void { this.closeGuard.assertReady(this.unresolved()); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.unresolved()); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    readonly element = ui("details", "query-registered-navigation");
    private readonly tagsHost = ui("div", "registered-tag-list");
    private readonly status = ui("p", "data-help");
    private readonly createName = ui("input", "data-input");
    private readonly parentHost = ui("div");
    private parentRef: string | null = null;
    private tree: TagTree | null = null;
    private search = "";
    private offset = 0;
    private expanded = new Set<string>();
    private recoveryError?: UiText;
    private busy = false;
    private registryRead = 0;
    private registry: TagRegistry | null = null;
    private pending: {
        route: string;
        body: Record<string, unknown>;
        prepared?: NavigationPreparation;
        addKind?: "directory" | "tag";
        label: string;
        confirmed?: boolean;
        context?: DocumentBasis;
        cancelRequested?: boolean;
        recoveredPlanId?: string;
    } | null = null;
    constructor(private host: PanelHost, private root: string, private workspaceId: string, private scopePath: () => string, private chooseTag: (ref: string, label: string, mode: TagMatchMode) => void, private saveShortcut: (ref: string, label: string, kind: "directory" | "tag", matchMode?: TagMatchMode) => void, private tagField: () => string | undefined, private registryChanged: () => void) {
        this.closeGuard.bind(this.element);
        const retainName = (): void => { if (!this.closeGuard.frozen) this.createNameDraft = this.createName.value; };
        this.createName.addEventListener("input", retainName);
        this.createName.addEventListener("change", retainName);
        this.element.append(ui("summary", "", message("tagNavigation.disclosure")), action(message("tagNavigation.readRegistry"), () => this.loadTags()), action(message("tagNavigation.addCurrentDirectoryShortcut"), () => this.addDirectory()));
        try {
            const raw = localStorage.getItem(this.pendingKey());
            if (raw) {
                const saved = JSON.parse(raw);
                if (!saved || !["/api/navigation/create-tag", "/api/navigation/rename-tag", "/api/navigation/move-tag", "/api/navigation/admit-directory"].includes(saved.route) || saved.body?.root !== root || typeof saved.body.operationId !== "string" || typeof saved.label !== "string" || saved.prepared && (saved.prepared.preview?.operationId !== saved.body.operationId || typeof saved.prepared.preview?.planId !== "string") || saved.recoveredPlanId !== undefined && (typeof saved.recoveredPlanId !== "string" || !saved.recoveredPlanId || saved.prepared && saved.prepared.preview.planId !== saved.recoveredPlanId))
                    throw new UiError(message("tagNavigation.invalidRecoveryRecord"));
                this.pending = saved;
            }
        }
        catch (error) {
            this.recoveryError = (error instanceof Error ? errorUiText(error) : message("error.invalidResponse"));
            setUiText(this.status, this.recoveryError);
        }
        setUiAttr(this.createName, "placeholder", message("tagNavigation.newNameLabel"));
        setUiAttr(this.createName, "aria-label", message("tagNavigation.newNameLabel"));
        this.element.append(this.tagsHost, this.createName, this.parentHost, action(message("tagNavigation.createTag"), () => this.createTag(false)), action(message("tagNavigation.createAssignSaved"), () => this.createTag(true)), action(message("tagNavigation.confirmOriginal"), () => this.resume()), action(message("tagNavigation.cancelOriginal"), async () => {
            if (this.closeGuard.frozen || this.busy || !this.pending)
                return;
            this.pending.cancelRequested = true;
            this.persistPending();
            await this.resume();
        }), this.status, ui("p", "data-help", message("tagNavigation.treeScopeHelp")));
    }
    private async createTag(assign: boolean): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            try {
                if (!this.createName.value)
                    return;
                if (!this.registry)
                    await this.loadTags();
                if (this.closeGuard.frozen || !this.registry || !this.tree)
                    return;
                const basis = assign ? this.host.getDocument() : null, fieldId = this.tagField();
                if (assign && (!basis || basis.root !== this.root || basis.dirty !== false || this.host.canEdit?.() === false || !fieldId || !this.host.applyOperationReceipt))
                    throw new UiError(message("tagNavigation.savedNoteRequired"));
                await this.mutate("/api/navigation/create-tag", { displayName: this.createName.value, parentRef: this.parentRef, baseId: this.registry.baseId, ...(assign && basis ? { assignment: { path: basis.path, baseId: basis.baseId, fieldId } } : {}) }, this.createName.value, "tag", basis ?? undefined);
            }
            catch (error) {
                if (this.closeGuard.frozen) return;
                setUiText(this.status, (error instanceof Error ? errorUiText(error) : message("error.invalidResponse")));
            }
        } finally { endClose(); }
    }
    private pendingKey(): string { return `weftext-navigation-operation:${JSON.stringify([this.workspaceId, this.root])}`; }
    private persistPending(): void {
        if (this.pending)
            localStorage.setItem(this.pendingKey(), JSON.stringify(this.pending));
        else
            localStorage.removeItem(this.pendingKey());
    }
    private async loadTags(): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            const read = ++this.registryRead;
            setUiText(this.status, message("tagNavigation.readingRegistry"));
            this.tree = null;
            this.createName.oninput = null;
            this.parentHost.replaceChildren();
            this.tagsHost.replaceChildren();
            try {
                const registry = await this.host.request<TagRegistry>("/api/navigation/tags", { root: this.root });
                if (this.closeGuard.frozen || read !== this.registryRead)
                    return;
                this.registry = registry;
                this.tree = new TagTree(registry.tags.map(tag => ({ ...tag, reference: tag.tagRef })), registry.complete === true);
                this.renderTags();
                this.parentHost.append(tagPicker(this.tree, { label: message("tagNavigation.newParentLabel"), value: this.parentRef ?? "", root: true, changed: ref => { if (!this.closeGuard.frozen) this.parentRef = ref; } }));
                const duplicates = ui("div", "data-help");
                const showNames = () => {
                    if (this.closeGuard.frozen) return;
                    duplicates.replaceChildren();
                    const same = this.registry!.tags.filter(tag => tag.displayName === this.createName.value);
                    if (same.length)
                        duplicates.append(ui("p", "", displayMessage("tagNavigation.sameNameWarning", { p0: same.length })), ...same.slice(0, 100).map(tag => ui("p", "", this.tree!.label(tag.tagRef))));
                };
                this.createName.oninput = showNames;
                showNames();
                this.parentHost.append(duplicates);
                setUiText(this.status, this.recoveryError ?? message("tagNavigation.completeRegistryRead"));
            }
            catch (error) {
                if (this.closeGuard.frozen || read !== this.registryRead)
                    return;
                this.registry = null;
                this.tagsHost.replaceChildren();
                setUiText(this.status, displayMessage("tagNavigation.registryUnavailable", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : message("error.invalidResponse")) }));
            }
        } finally { endClose(); }
    }
    private renderTags(): void {
        if (this.closeGuard.frozen) return;
        const tree = this.tree;
        this.tagsHost.replaceChildren();
        if (!tree)
            return;
        const search = ui("input", "data-input");
        search.type = "search";
        search.value = this.search;
        setUiAttr(search, "aria-label", message("tagNavigation.searchLabel"));
        const rows = ui("div");
        const render = () => {
            if (this.closeGuard.frozen) return;
            rows.replaceChildren();
            const page = this.search ? tree.page(this.search, this.offset) : tree.visible(this.expanded, this.offset);
            this.offset = page.offset;
            rows.append(ui("p", "data-help", displayMessage("tagNavigation.treePageSummary", { p0: tree.byId.size, p1: uiDisplayParameter(this.search ? message("tagNavigation.searchMatches") : message("tagNavigation.expandedItems")), p2: page.total, p3: page.total ? page.offset + 1 : 0, p4: page.offset + page.items.length })));
            for (const tag of page.items) {
                const row = ui("div", "registered-tag-row"), label = tree.label(tag.reference);
                row.setAttribute("role", "group");
                setUiAttr(row, "aria-label", label);
                if (tree.children.get(tag.tagId)?.length) {
                    const expand = action(this.expanded.has(tag.tagId) ? message("tagNavigation.collapseDescendants") : message("tagNavigation.expandDescendants"), () => {
                        if (this.expanded.has(tag.tagId))
                            this.expanded.delete(tag.tagId);
                        else
                            this.expanded.add(tag.tagId);
                        render();
                        rows.querySelector<HTMLButtonElement>(`[data-expand-tag="${CSS.escape(tag.tagId)}"]`)?.focus({ preventScroll: true });
                    }, "data-button compact");
                    expand.dataset.expandTag = tag.tagId;
                    expand.setAttribute("aria-expanded", String(this.expanded.has(tag.tagId)));
                    row.append(expand);
                }
                row.append(action(displayMessage("tagNavigation.subtreeEntry", { p0: uiDisplayParameter(label) }), () => this.chooseTag(tag.reference, label, "subtree"), "query-facet"), action(message("tagNavigation.exactEntry"), () => this.chooseTag(tag.reference, label, "exact"), "data-button compact"), action(message("tagNavigation.saveSubtreeShortcut"), () => this.saveShortcut(tag.reference, label, "tag", "subtree"), "data-button compact"), action(message("tagNavigation.saveExactShortcut"), () => this.saveShortcut(tag.reference, label, "tag", "exact"), "data-button compact"), action(message("tagNavigation.rename"), async () => {
                    const name = window.prompt(uiTextValue(message("tagNavigation.renamePrompt")), tag.displayName);
                    if (name == null || name === tag.displayName || !name)
                        return;
                    await this.mutate("/api/navigation/rename-tag", { tagRef: tag.reference, displayName: name, baseId: this.registry!.baseId }, name);
                }, "data-button compact"), action(message("tagNavigation.changeParent"), () => this.move(tag.reference), "data-button compact"));
                const full = ui("details");
                full.append(ui("summary", "", message("tagNavigation.completePath")));
                full.ontoggle = () => {
                    if (!this.closeGuard.frozen && full.open && full.childElementCount === 1)
                        full.append(ui("p", "", tree.label(tag.reference, true)));
                };
                row.append(full);
                rows.append(row);
            }
            const focusPage = (direction: string) => (rows.querySelector<HTMLButtonElement>(`.tag-page-${direction}:not(:disabled)`) ?? rows.querySelector<HTMLButtonElement>(".tag-page-previous:not(:disabled), .tag-page-next:not(:disabled)") ?? search).focus({ preventScroll: true });
            const previous = action(message("tagNavigation.previousPage"), () => { this.offset -= 100; render(); focusPage("previous"); }, "data-button tag-page-previous"), next = action(message("tagNavigation.nextPage"), () => { this.offset += 100; render(); focusPage("next"); }, "data-button tag-page-next");
            previous.disabled = page.offset === 0;
            next.disabled = page.offset + page.items.length >= page.total;
            rows.append(previous, next);
        };
        search.oninput = () => { if (this.closeGuard.frozen) return; this.search = search.value; this.offset = 0; render(); };
        this.tagsHost.append(search, rows);
        render();
    }
    private async move(reference: string): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            const tree = this.tree, tag = tree?.byRef.get(reference);
            if (!tree || !tag || !this.registry)
                return;
            const parent = tag.parentTagId === null ? null : tree.byId.get(tag.parentTagId)!.reference;
            const chosen = await new Promise<{
                parent: string | null;
            } | null>(resolve => { const dialog = ui("dialog", "data-confirm-dialog"), form = ui("form"); form.method = "dialog"; let next = parent; form.append(ui("h2", "", message("tagNavigation.parentDialogTitle")), ui("p", "", tree.label(reference)), tagPicker(tree, { label: message("tagNavigation.parentLabel"), value: parent ?? "", root: true, excludedRoot: tag.tagId, changed: ref => { next = ref; } })); const cancel = ui("button", "data-button", message("common.cancel")); cancel.value = "cancel"; const prepare = ui("button", "data-button primary", message("tagNavigation.previewParentChange")); prepare.value = "prepare"; form.append(cancel, prepare); dialog.append(form); document.body.append(dialog); dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "prepare"; dialog.remove(); resolve(accepted ? { parent: next } : null); }, { once: true }); dialog.showModal(); });
            if (!this.closeGuard.frozen && chosen && chosen.parent !== parent)
                await this.mutate("/api/navigation/move-tag", { tagRef: reference, parentRef: chosen.parent, baseId: this.registry.baseId }, tree.label(reference));
        } finally { endClose(); }
    }
    private async addDirectory(): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            try {
                const path = this.scopePath();
                const target = await this.host.request<NavigationTarget>(path.startsWith("weftext-directory://") || path.startsWith("weftext-node://") ? "/api/navigation/resolve" : "/api/navigation/capture", { root: this.root, ...(path.startsWith("weftext-directory://") || path.startsWith("weftext-node://") ? { targetRef: path } : { path }) });
                if (this.closeGuard.frozen) return;
                if (target.status === "managed" && target.targetRef) {
                    this.saveShortcut(target.targetRef, target.label, "directory");
                    setUiText(this.status, message("tagNavigation.directoryShortcutAdded"));
                    return;
                }
                if (target.status !== "unmanaged" || !target.path)
                    throw new UiError(displayMessage("tagNavigation.directoryTargetState", { p0: uiDisplayParameter(target.status === "missing" ? message("tagNavigation.missingTarget") : message("tagNavigation.unavailableIdentity")) }));
                await this.mutate("/api/navigation/admit-directory", { path: target.path }, target.label, "directory");
            }
            catch (error) {
                if (this.closeGuard.frozen) return;
                setUiText(this.status, error instanceof Error ? errorUiText(error) : message("error.invalidResponse"));
            }
        } finally { endClose(); }
    }
    private async mutate(route: string, payload: Record<string, unknown>, label: string, addKind?: "directory" | "tag", context?: DocumentBasis): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            if (this.recoveryError) {
                setUiText(this.status, this.recoveryError);
                return;
            }
            if (this.pending) {
                await this.resume();
                return;
            }
            this.pending = { route, body: { root: this.root, ...payload, operationId: crypto.randomUUID() }, label, addKind, context };
            try {
                this.persistPending();
                await this.resume();
            }
            catch (error) {
                if (this.closeGuard.frozen) return;
                setUiText(this.status, displayMessage("tagNavigation.persistenceBeforeSendFailed", { p0: uiDisplayParameter((error instanceof Error ? errorUiText(error) : message("error.invalidResponse"))) }));
            }
        } finally { endClose(); }
    }
    private async resume(): Promise<void> {
        if (this.closeGuard.frozen) return;
        const endClose = this.closeGuard.begin();
        try {
            if (this.busy)
                return;
            if (!this.pending) {
                setUiText(this.status, this.recoveryError ?? message("tagNavigation.noPendingOperation"));
                return;
            }
            const pending = this.pending;
            this.busy = true;
            try {
                this.persistPending(); // A retry must satisfy the same durable-before-effect boundary.
                let recovering = !!pending.confirmed;
                if (!pending.cancelRequested) {
                    if (!pending.prepared) {
                        const prepared = await this.host.request<NavigationPreparation>(pending.route, structuredClone(pending.body));
                        if (this.closeGuard.frozen) return;
                        pending.prepared = prepared;
                        if (pending.prepared.preview.operationId !== pending.body.operationId)
                            throw new UiError(message("tagNavigation.preparationMismatch"));
                        this.persistPending();
                    }
                    if (!pending.confirmed) {
                        const confirmed = await this.confirm(pending);
                        if (this.closeGuard.frozen) return;
                        if (!confirmed)
                            pending.cancelRequested = true;
                        else
                            pending.confirmed = true;
                        this.persistPending();
                    }
                }
                if (pending.cancelRequested) {
                    const expected = { operationId: String(pending.body.operationId), planId: pending.prepared?.preview.planId ?? pending.recoveredPlanId ?? null };
                    const cancelled = await this.host.request<{
                        operationId: string;
                        planId: string | null;
                        status: string;
                    }>("/api/navigation/cancel", { root: this.root, ...expected });
                    if (this.closeGuard.frozen) return;
                    if (tagCancellationStatus(expected, cancelled) === "resolved") {
                        this.pending = null;
                        this.persistPending();
                        setUiText(this.status, message("tagNavigation.originalEnded"));
                        return;
                    }
                    if (typeof cancelled.planId !== "string")
                        throw new UiError(message("tagNavigation.committedPlanIdMissing"));
                    pending.recoveredPlanId = cancelled.planId;
                    this.persistPending();
                    recovering = true;
                }
                const planId = pending.prepared?.preview.planId ?? pending.recoveredPlanId;
                if (!planId)
                    throw new UiError(message("tagNavigation.originalPlanIdMissing"));
                const expected = { operationId: String(pending.body.operationId), planId };
                let committed = false, receipt: {
                    operationId?: string;
                    planId?: string;
                    status?: string;
                    receipt?: {
                        status?: string;
                    };
                } | undefined;
                if (recovering) {
                    receipt = await this.host.request("/api/operations/status", { operationId: expected.operationId, context: pending.context });
                    if (this.closeGuard.frozen) return;
                    committed = tagOperationStatus(expected, receipt!) === "committed";
                }
                if (pending.cancelRequested && !committed)
                    throw new UiError(message("tagNavigation.cancelCommitUnconfirmed"));
                if (!committed) {
                    receipt = await this.host.request("/api/operations/commit", { ...expected, context: pending.context });
                    if (this.closeGuard.frozen) return;
                    if (tagOperationStatus(expected, receipt!) !== "committed")
                        throw new UiError(message("tagNavigation.registrationUnconfirmed"));
                }
                if (this.closeGuard.frozen) return;
                this.registryChanged();
                if (pending.context) {
                    if (!this.host.applyOperationReceipt)
                        throw new UiError(message("tagNavigation.savedNoteRequired"));
                    await this.host.applyOperationReceipt(receipt, pending.context);
                    if (this.closeGuard.frozen) return;
                    const current = this.host.getDocument();
                    if (current && current.root === pending.context.root && current.path === pending.context.path && current.sessionId !== pending.context.sessionId)
                        this.host.notify(message("tagNavigation.assignmentCommittedSessionChanged"));
                }
                if (pending.addKind && pending.prepared)
                    this.saveShortcut(pending.prepared.targetRef, pending.label, pending.addKind, pending.addKind === "tag" ? "subtree" : undefined);
                else if (pending.addKind)
                    this.host.notify(message("tagNavigation.committedWithoutPreparation"));
                if (pending.route.endsWith("create-tag")) {
                    if (this.createName.value === pending.body.displayName) {
                        this.createName.value = "";
                        this.createNameDraft = "";
                    }
                    this.appliedParentRef = typeof pending.body.parentRef === "string" ? pending.body.parentRef : null;
                }
                this.pending = null;
                this.persistPending();
                setUiText(this.status, message("tagNavigation.registrationComplete"));
                await this.loadTags();
            }
            catch (error) {
                if (this.closeGuard.frozen) return;
                let persistence: UiText = "";
                try {
                    this.persistPending();
                }
                catch (storageError) {
                    persistence = displayMessage("tagNavigation.recoveryPersistenceFailed", { p0: uiDisplayParameter((storageError instanceof Error ? errorUiText(storageError) : message("error.invalidResponse"))) });
                }
                setUiText(this.status, displayMessage("tagNavigation.originalRetained", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : message("error.invalidResponse")), p1: uiDisplayParameter(persistence) }));
            }
            finally {
                if (!this.closeGuard.frozen) this.busy = false;
            }
        } finally { endClose(); }
    }
    private async confirm(pending: NonNullable<NavigationControls["pending"]>): Promise<boolean> {
        if (this.closeGuard.frozen) return false;
        const endClose = this.closeGuard.begin();
        try {
            return await new Promise(resolve => {
                const dialog = ui("dialog", "data-confirm-dialog"), form = ui("form");
                form.method = "dialog";
                const title = pending.route.endsWith("admit-directory") ? message("tagNavigation.directoryConfirmationTitle") : pending.route.endsWith("rename-tag") ? message("tagNavigation.renameConfirmationTitle") : pending.route.endsWith("move-tag") ? message("tagNavigation.parentDialogTitle") : pending.context ? message("tagNavigation.createAssignConfirmationTitle") : message("tagNavigation.createConfirmationTitle");
                form.append(ui("h2", "", title), ui("p", "", pending.label));
                if (pending.route.endsWith("admit-directory"))
                    form.append(ui("p", "data-help", message("tagNavigation.directoryConfirmationHelp")));
                if (pending.context) {
                    const path = pending.context.path;
                    form.append(ui("p", "data-help", displayMessage("tagNavigation.assignmentConfirmationHelp", { p0: path })));
                }
                const preview = ui("details");
                preview.append(ui("summary", "", message("tagNavigation.exactPlanDisclosure")), ui("pre", "", JSON.stringify(pending.prepared?.preview, null, 2)));
                form.append(preview);
                const cancel = ui("button", "data-button", message("common.cancel"));
                cancel.value = "cancel";
                const commit = ui("button", "data-button primary", message("tagNavigation.commitPreview"));
                commit.value = "apply";
                form.append(cancel, commit);
                dialog.append(form);
                document.body.append(dialog);
                dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "apply"; dialog.remove(); resolve(accepted); }, { once: true });
                dialog.showModal();
            });
        } finally { endClose(); }
    }
}
export interface QueryRow {
    /** Present only in the separately decoded hosted navigation projection. */
    resource?:string;
    canOpen?:boolean;
    key: string;
    entityKey?: string|null;
    path?: string|null;
    revision?: string|null;
    lifecycle?: "live" | "trash";
    cells: Record<string, Cell>;
    readOnly: boolean;
    preview?: {
        state: "absent" | "building" | "ready" | "failed" | "unsupported" | "unavailable";
        url?: string;
        message?: string;
    };
}
export function queryRowLifecycle(row: QueryRow, plan: QueryPlan | null): "live" | "trash" | null {
    if (row.lifecycle === "live" || row.lifecycle === "trash")
        return row.lifecycle;
    if (plan?.advanced?.unionAll || plan?.advanced?.related || plan?.advanced?.call)
        return null;
    return plan?.scope.lifecycle ?? null;
}
export interface QueryResult {
    rows: QueryRow[];
    columns?: string[];
    coverage: "complete" | "partial";
    freshness: "current" | "stale";
    diagnostics: readonly CoreDescription[];
    engineDiagnostics?: readonly QueryEngineGroup[];
    productErrors?: readonly QueryProductError[];
    exportUnavailable?: UiMessage;
    totalKnown: number;
    limited: boolean;
    queryId?: string;
    cutId?: string;
    schemaRevision?: string;
    facets?: QueryFacet[];
    snapshotMode?: string;
    snapshotLabel?: UiMessage;
    externalPending?: boolean;
    aggregateExact?: boolean;
    rawScope?: {
        complete?: boolean;
        pendingCount?: number;
    };
    superseded?: boolean;
}
export interface QueryFacet {
    fieldId: string;
    key: string;
    label?: string;
    value?: NativeValue;
    expr?: string;
    count: number;
    missing?: boolean;
}
export type ResultView = "list" | "table" | "board" | "calendar" | "chart";
interface Condition {
    id: string;
    label: UiText;
    expr: string;
    tag?: {
        reference: string;
        label: string;
        mode: TagMatchMode;
    };
}
export interface ViewPreference {
    view: ResultView;
    density: "compact" | "comfortable";
    summaries: boolean;
    thumbnails: boolean;
    columns: string[];
    pinned: string[];
    sort: string;
    descending: boolean;
    groupField: string;
    dateField: string;
    chartField: string;
    fieldBindings?: Record<string, string>;
}
interface Shortcut {
    id: string;
    label: string;
    kind: "query" | "node" | "savedQuery" | "directory" | "tag";
    plan?: QueryPlan;
    entityKey?: string;
    path?: string;
    targetRef?: string;
    definition?: SavedSelection;
    matchMode?: TagMatchMode;
}
interface NavigationTarget {
    targetRef?: string | null;
    path?: string | null;
    status: string;
    kind: string;
    label: string;
    displayName?: string;
}
interface SavedQuery {
    key: string;
    label: string;
    plan: QueryPlan;
}
interface SavedDefinitions {
    definitions: SavedQuery[];
    ownerRevision: string | null;
    unavailable?: unknown[];
}
interface SavedSelection {
    owner: string;
    key: string;
    mode: "current" | "pinned";
    revision?: string;
    label: string;
}
export interface QueryPlanInputs {
    scope: string | null;
    descendants: boolean;
    lifecycle: "live" | "trash";
    types: string[];
    search: string;
    includeBody: boolean;
    conditions: {
        expr: string;
    }[];
    expression: string;
    exactPlan: QueryPlan | null;
    group: QueryPlan["group"];
    take: number;
    preference: ViewPreference;
}
interface QueryRefreshIntent {
    root: string;
    workspaceId: string;
    epoch: number;
    inputs: QueryPlanInputs;
    selection: SavedSelection | null;
    parameters: Record<string, NativeValue>;
}
type RowPreview = ListPreview;
export const defaultPreference = (): ViewPreference => ({ view: "list", density: "comfortable", summaries: true, thumbnails: false, columns: ["status", "due", "project", "tags"], pinned: [], sort: "title", descending: false, groupField: "status", dateField: "due", chartField: "count" });
export function queryResultState(result: QueryResult): UiText {
    if (result.superseded)
        return message("queryStatus.superseded");
    if (result.snapshotMode === "admittedFrozen")
        return displayMessage("queryStatus.frozenSummary", { p0: uiDisplayParameter(result.coverage === "complete" ? message("queryStatus.completeFrozen") : message("queryStatus.partialFrozen")), p1: result.totalKnown, p2: uiDisplayParameter(result.limited ? displayMessage("queryStatus.displayedCountSuffix", { p0: result.rows.length }) : ""), p3: uiDisplayParameter(result.freshness === "stale" ? message("queryStatus.unconfirmedCurrentnessSuffix") : ""), p4: uiDisplayParameter(result.coverage === "complete" && !result.totalKnown ? message("queryStatus.completeFrozenEmptySuffix") : "") });
    if (result.freshness === "stale")
        return message("queryStatus.stale");
    if (result.coverage === "partial")
        return displayMessage("queryStatus.partialSummary", { p0: result.totalKnown, p1: uiDisplayParameter(result.limited ? message("queryStatus.limitReachedSuffix") : "") });
    if (result.limited)
        return displayMessage("queryStatus.limitedCompleteSummary", { p0: result.totalKnown, p1: result.rows.length });
    return result.rows.length ? displayMessage("queryStatus.completeSummary", { p0: result.totalKnown }) : message("queryStatus.completeEmpty");
}
export function pinEligibleRows(rows: QueryRow[], pins: string[]): QueryRow[] {
    const wanted = new Set(pins);
    return [...rows.filter(row => wanted.has(row.entityKey ?? row.key)), ...rows.filter(row => !wanted.has(row.entityKey ?? row.key))];
}
export function preferenceFieldErrors(preference: ViewPreference, schema: DataSchema): UiText[] {
    return Object.entries(preference.fieldBindings ?? {}).filter(([key, id]) => [preference.sort, ...preference.columns, ...(preference.view === "board" ? [preference.groupField] : []), ...(preference.view === "calendar" ? [preference.dateField] : []), ...(preference.view === "chart" ? [preference.chartField] : [])].includes(key) && !schema.fields.some(field => field.id === id && field.key === key)).map(([key]) => displayMessage("queryView.invalidFieldBinding", { p0: uiDisplayParameter(fieldNames[key] ?? key) }));
}
/** Match only native plan inputs and the field bindings validated before execution. */
export function queryPlanRefreshInput(input: QueryPlanInputs): unknown {
    const common = { scope: input.scope, descendants: input.descendants, lifecycle: input.lifecycle, types: input.types,
        search: input.search, includeBody: input.includeBody, conditions: input.conditions.map(item => item.expr), expression: input.expression,
        exactPlan: input.exactPlan, group: input.group, take: input.take };
    if (input.exactPlan)
        return common;
    const preference = input.preference, columns = [...new Set([...preference.columns,
            ...(preference.view === "board" ? [preference.groupField] : []), ...(preference.view === "calendar" ? [preference.dateField] : []), ...(preference.view === "chart" ? [preference.chartField] : [])])];
    const fields = new Set([preference.sort, ...columns]);
    return { ...common, preference: { columns, sort: preference.sort, descending: !input.group && preference.descending,
            summaries: !input.group && preference.summaries, fieldBindings: Object.fromEntries(Object.entries(preference.fieldBindings ?? {}).filter(([key]) => fields.has(key))) } };
}
export function decodeViewPreference(value: unknown): ViewPreference {
    const preference = defaultPreference();
    if (!value || typeof value !== "object" || Array.isArray(value))
        return preference;
    const saved = value as Record<string, unknown>;
    if (["list", "table", "board", "calendar", "chart"].includes(String(saved.view)))
        preference.view = saved.view as ResultView;
    if (saved.density === "compact" || saved.density === "comfortable")
        preference.density = saved.density;
    for (const key of ["summaries", "thumbnails", "descending"] as const)
        if (typeof saved[key] === "boolean")
            preference[key] = saved[key];
    for (const key of ["sort", "groupField", "dateField", "chartField"] as const)
        if (typeof saved[key] === "string" && saved[key])
            preference[key] = saved[key];
    for (const key of ["columns", "pinned"] as const)
        if (Array.isArray(saved[key]))
            preference[key] = saved[key].filter((item): item is string => typeof item === "string");
    if (saved.fieldBindings && typeof saved.fieldBindings === "object" && !Array.isArray(saved.fieldBindings))
        preference.fieldBindings = Object.fromEntries(Object.entries(saved.fieldBindings).filter((item): item is [
            string,
            string
        ] => typeof item[1] === "string"));
    return preference;
}
export function viewPreferenceIdentity(root: string, scope: string, types: string[], predicates: string[], saved?: {
    owner: string;
    key: string;
    mode: string;
    revision?: string;
}): string {
    return saved ? JSON.stringify(["saved-query", saved.owner, saved.key, saved.mode, saved.mode === "pinned" ? saved.revision ?? "" : "current"]) : JSON.stringify(["query-context", root, scope, [...new Set(types)].sort(), [...new Set(predicates)].sort()]);
}
export function hasDirectStatusProjection(plan: QueryPlan | null, schema: DataSchema | null): boolean { const field = schema?.fields.find(field => field.key === "status"); const column = plan?.select.find(column => column.name === "status"); return !!field && !!column && [fieldExpr(field.id), field.key].includes(column.expr.trim()) && !plan?.group && !plan?.unnest && !plan?.advanced; }
export function directQueryField(plan: QueryPlan | null, schema: DataSchema | null, column: string): SchemaField | undefined {
    if (!plan || plan.group || plan.unnest || plan.advanced)
        return;
    const projection = plan.select.find(item => item.name === column);
    return schema?.fields.find(field => projection && [fieldExpr(field.id), field.key].includes(projection.expr.trim()));
}
export function distinctFacetCounts(rows: QueryRow[], column: string): {
    value?: NativeValue;
    count: number;
    missing: boolean;
}[] {
    const facets = new Map<string, {
        value?: NativeValue;
        entities: Set<string>;
        missing: boolean;
    }>();
    for (const row of rows) {
        if (row.readOnly && !row.entityKey)
            continue;
        const cell = row.cells[column];
        const values = cell?.state === "value" && cell.value ? cell.value.kind === "array" ? cell.value.value : [cell.value] : cell?.state === "absent" ? [undefined] : cell?.state === "null" ? [{ kind: "none" } as NativeValue] : [];
        for (const value of values) {
            const key = value ? canonicalNative(value) : "@absent";
            const facet = facets.get(key) ?? { value, entities: new Set<string>(), missing: !value };
            facet.entities.add(row.entityKey ?? row.key);
            facets.set(key, facet);
        }
    }
    return [...facets.values()].map(({ value, entities, missing }) => ({ value, count: entities.size, missing }));
}
function canonicalNative(value: NativeValue): string {
    if (value.kind === "record")
        return JSON.stringify({ kind: "record", value: [...value.value].sort((a, b) => a.key < b.key ? -1 : a.key > b.key ? 1 : 0).map(item => [item.key, canonicalNative(item.value)]) });
    if (value.kind === "array")
        return JSON.stringify({ kind: "array", value: value.value.map(canonicalNative) });
    return JSON.stringify(value);
}
export function scalarQueryLiteral(value: NativeValue): string {
    if (value.kind === "none")
        return "null";
    if (value.kind === "bool")
        return String(value.value);
    if (value.kind === "int")
        return value.value;
    if (value.kind === "float")
        return /[.eE]/.test(value.value) ? value.value : `${value.value}.0`;
    if (value.kind === "decimal")
        return `decimal(${JSON.stringify(value.value)})`;
    if (value.kind === "date")
        return `date(${JSON.stringify(value.value)})`;
    if (value.kind === "time")
        return `time(${JSON.stringify(value.value)})`;
    if (value.kind === "localDatetime")
        return `localDatetime(${JSON.stringify(value.value)})`;
    if (value.kind === "duration")
        return `duration(${JSON.stringify(value.value)})`;
    if (value.kind === "text")
        return JSON.stringify(value.value);
    throw new UiError(message("queryValidation.nestedLiteralUnsupported"));
}
export function shapedQueryLiteral(shape: Shape, schema: DataSchema, value: NativeValue): string {
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "union") {
        const branch = unionVariant(resolved, value, schema);
        if (branch < 0)
            throw new UiError(message("queryValidation.ambiguousValueBranch"));
        return shapedQueryLiteral(resolved.variants[branch], schema, value);
    }
    if (resolved.kind === "scalar") {
        const constructor = ({ node_ref: "nodeRef", tag_ref: "tagRef", resource_ref: "resourceRef", instant: "instant", exact_decimal: "exactDecimal", exact_seconds: "exactSeconds" } as Record<string, string>)[resolved.semantic];
        if (constructor) {
            if (value.kind !== "text")
                throw new UiError(message("queryValidation.scalarRepresentationMismatch"));
            return `${constructor}(${JSON.stringify(value.value)})`;
        }
    }
    return scalarQueryLiteral(value);
}
export function facetPredicate(facet: QueryFacet, schema: DataSchema): string {
    if (facet.expr)
        return facet.expr;
    const expr = fieldExpr(facet.fieldId);
    if (facet.missing)
        return `isAbsent(${expr})`;
    if (facet.value?.kind === "none")
        return `isNull(${expr})`;
    if (!facet.value)
        throw new UiError(message("queryValidation.navigationValueMissing"));
    const definition = schema.fields.find(field => field.id === facet.fieldId);
    if (!definition)
        throw new UiError(message("queryValidation.navigationFieldInvalid"));
    const shape = resolveShape(definition.shape, schema);
    if (shape.kind !== "list")
        return `${expr} == ${shapedQueryLiteral(shape, schema, facet.value)}`;
    let item = resolveShape(shape.items, schema);
    if (item.kind === "union") {
        const branch = unionVariant(item, facet.value, schema);
        if (branch < 0)
            throw new UiError(message("queryValidation.navigationBranchAmbiguous"));
        item = resolveShape(item.variants[branch], schema);
    }
    if (facet.value.kind === "record" && item.kind === "record") {
        const key = nativeMember(facet.value, "ref") ? "ref" : nativeMember(facet.value, "text") ? "text" : undefined;
        if (!key || !item.members[key])
            throw new UiError(message("queryValidation.nestedNavigationUnsupported"));
        const value = nativeMember(facet.value, key)!;
        return `${expr}.exists(item, has(item.${key}) && item.${key} == ${shapedQueryLiteral(item.members[key].shape, schema, value)})`;
    }
    return `${expr}.exists(item, item == ${shapedQueryLiteral(item, schema, facet.value)})`;
}
export function composeWhere(search: string, includeBody: boolean, conditions: readonly {
    expr: string;
}[], expression: string): string | undefined {
    const pieces = conditions.map(condition => `(${condition.expr})`);
    if (search) {
        const literal = JSON.stringify(search);
        pieces.unshift(includeBody ? `(title.contains(${literal}) || body.contains(${literal}))` : `(title.contains(${literal}))`);
    }
    if (expression.trim())
        pieces.push(`(${expression.trim()})`);
    return pieces.length ? pieces.join(" && ") : undefined;
}
/** Civil date grouping uses the exact day string, never a Date/host timezone. */
export function calendarDay(value: NativeValue | undefined): string | null { return value?.kind === "date" && /^\d{4}-\d{2}-\d{2}$/.test(value.value) ? value.value : null; }
const fieldExpr = (fieldId: string): string => `field(${JSON.stringify(fieldId)})`;
function planForInputs(input: QueryPlanInputs, schema: DataSchema | null): QueryPlan {
    if (input.exactPlan) {
        const plan = structuredClone(input.exactPlan);
        plan.types = [...input.types];
        plan.where = composeWhere(input.search, input.includeBody, input.conditions, plan.where ?? "");
        return plan;
    }
    const field = (key: string) => schema?.fields.find(item => item.key === key)?.id;
    const preference = input.preference, columns = [{ name: "title", expr: "title" }, { name: "types", expr: "types" }];
    if (preference.summaries && !input.group)
        columns.push({ name: "summary", expr: "body" });
    const requested = new Set(preference.columns);
    if (preference.view === "board")
        requested.add(preference.groupField);
    if (preference.view === "calendar")
        requested.add(preference.dateField);
    if (preference.view === "chart")
        requested.add(preference.chartField);
    for (const key of requested) {
        const id = field(key);
        if (id)
            columns.push({ name: key, expr: fieldExpr(id) });
    }
    const sort = preference.sort === "title" ? "title" : field(preference.sort) ? fieldExpr(field(preference.sort)!) : preference.sort;
    return { scope: { ...(input.scope ? { directory: input.scope } : {}), descendants: input.descendants, lifecycle: input.lifecycle }, types: [...input.types], where: composeWhere(input.search, input.includeBody, input.conditions, input.expression), select: input.group ? [] : columns, order: input.group ? [] : [{ expr: sort, descending: preference.descending, nulls: "last" }], take: input.take, ...(input.group ? { group: input.group } : {}) };
}
const rowTitle = (row: QueryRow): UiText => row.cells.title?.state === "value" && row.cells.title.value ? formatNativeValue(row.cells.title.value) || row.path || row.key : row.path?.replaceAll("\\", "/").split("/").at(-1) || (row.readOnly && !row.entityKey ? joinUiText(Object.entries(row.cells).filter(([name]) => !["count", "amount"].includes(name)).map(([, cell]) => stateText(cell)), " · ") : row.entityKey || row.key);
const stateText = (cell: Cell | undefined): UiText => !cell ? message("queryValue.unprojected") : cell.state === "value" ? formatNativeValue(cell.value) : cellLabels[cell.state];
function labeled(label: UiText, input: HTMLElement): HTMLElement { const element = ui("label", "query-control-label", label); element.append(input); return element; }
function selectControl(options: [
    string,
    UiText
][], value: string): HTMLSelectElement { const input = ui("select", "data-input"); options.forEach(([key, label]) => { const option = ui("option", "", label); option.value = key; input.append(option); }); input.value = value; return input; }
interface QueryInlineDraft {
    value: NativeValue;
    raw: Record<string, string>;
    binding: string;
    revision: number;
}
export class QueryPanel {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly draftFlags = new Set<string>();
    private discardQueryProposals(): void {
        this.closeGuard.assertMutable();
        this.advancedDraft = JSON.stringify(this.getPlan(), null, 2);
        this.parameterDraft = "{}";
        this.definitionName = this.expression = this.search = "";
        this.draftFlags.clear();
        for (const update of this.controlUpdates ?? []) update(true);
        this.renderControls();
    }
    private transientProposal(): boolean { return this.draftFlags.size !== 0 || this.definitionName.length !== 0; }
    private unresolved(): boolean { return this.inlineDrafts.size !== 0 || this.savedPending !== null || this.transientProposal(); }
    private closeOwners(): (QuickCapture | PeriodNavigator | NavigationControls)[] { return [...this.captureForms.values(), ...this.periodNavigators.values(), ...this.navigationControls.values()]; }
    assertCloseReady(): void { this.closeGuard.assertReady(this.unresolved()); for (const owner of this.closeOwners()) owner.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        const frozen: (QuickCapture | PeriodNavigator | NavigationControls)[] = [];
        try {
            for (const owner of this.closeOwners()) {
                frozen.push(owner);
                if (!owner.freezeForClose()) throw new Error("Query proposal owner refused close");
            }
            if (!this.closeGuard.freeze(this.unresolved())) throw new Error("Query proposal changed during close");
        }
        catch {
            for (const owner of frozen.reverse()) owner.thawAfterClose();
            this.closeGuard.thaw();
            return false;
        }
        clearTimeout(this.searchTimer);
        this.thumbnailDisposers.splice(0).forEach(dispose => dispose());
        return true;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); for (const owner of this.closeOwners()) owner.thawAfterClose(); this.renderResults(); }
    private action(label: UiText, run: () => void | Promise<void>, className = "data-button"): HTMLButtonElement {
        return action(label, () => this.closeGuard.run(async () => { await run(); }).catch(error => this.host.notify(errorUiText(error), true)), className);
    }
    private inlineDrafts = new Map<string, QueryInlineDraft>();
    private retainInlineDraft(key: string, draft: QueryInlineDraft, value?: NativeValue, raw?: Record<string, string>): void {
        if (this.closeGuard.frozen) return;
        if (value !== undefined) draft.value = value;
        if (raw !== undefined) draft.raw = raw;
        draft.revision++;
        this.inlineDrafts.set(key, draft);
    }
    private async applyInlineDraft(row: QueryRow, field: SchemaField, key: string, draft: QueryInlineDraft): Promise<void> {
        return this.closeGuard.run(async () => {
            const revision = draft.revision, submitted = structuredClone(draft.value);
            await this.changeTaskField(row, field.key, submitted, field.id, () => {
                if (this.inlineDrafts.get(key) === draft && draft.revision === revision)
                    this.inlineDrafts.delete(key);
            });
        });
    }
    readonly element = ui("section", "query-panel");
    private root: string | null = null;
    private workspaceId?: string;
    private directory: string | null = null;
    private scopeIdentity: string | null = null;
    private navigationDirectory: string | null = null;
    private navigationScopeIdentity: string | null = null;
    private descendants = true;
    private lifecycle: "live" | "trash" = "live";
    private schema: DataSchema | null = null;
    private result: QueryResult | null = null;
    private resultPlan: QueryPlan | null = null;
    private search = "";
    private includeBody = false;
    private expression = "";
    private conditions: Condition[] = [];
    private types: string[] = [];
    private preference = defaultPreference();
    private shortcuts: Shortcut[] = [];
    private shortcutError?: UiText;
    private tagReferences: FormReferences = { nodes: [], state: "unavailable", messages: [] };
    private recent: {
        entityKey: string;
        path: string;
        label: string;
        lifecycle?: "live" | "trash";
        revision?: string;
    }[] = [];
    private selected: string | null = null;
    private generation = 0;
    private environmentEpoch = 0;
    private refreshQueue = new QueryRefreshQueue<QueryRefreshIntent>((intent, current) => this.executeRefresh(intent, current));
    private loading = false;
    private error: UiText | null = null;
    private group: QueryPlan["group"];
    private take = 200;
    private captureDialog: HTMLDialogElement | null = null;
    private captureDialogs = new Map<string, HTMLDialogElement>();
    private captureForms = new Map<string, QuickCapture>();
    private exactPlan: QueryPlan | null = null;
    private advancedDraft = "";
    private parameterDraft = "{}";
    private advancedOpen = false;
    private savedSelection: SavedSelection | null = null;
    private savedPlan: QueryPlan | null = null;
    private savedPending: {
        body: Record<string, unknown>;
        basis: NonNullable<ReturnType<PanelHost["getDocument"]>>;
    } | null = null;
    private definitionRevision: string | null = null;
    private definitionName = "";
    private thumbnailDisposers: (() => void)[] = [];
    private previewCache = new Map<string, Promise<RowPreview>>();
    private searchTimer?: ReturnType<typeof setTimeout>;
    private periodNavigators = new Map<string, PeriodNavigator>();
    private navigationControls = new Map<string, NavigationControls>();
    private controlsContext: string | null = null;
    private controlUpdates: ((reset: boolean) => void)[] | null = null;
    private readonly registeredHost = ui("div");
    private readonly periodHost = ui("div");
    private readonly savedHost = ui("div");
    private readonly shortcutsHost = ui("div");
    private readonly resultsHost = ui("div", "query-results");
    private readonly controlsHost = ui("div", "query-controls");
    private readonly navigationHost = ui("nav", "query-facets");
    constructor(private readonly host: PanelHost, private readonly exportResult?: (query: FrozenQuery) => void | Promise<void>) { this.closeGuard.bind(this.element); setUiAttr(this.element, "aria-label", message("queryPanel.title")); setUiAttr(this.navigationHost, "aria-label", message("queryPanel.valueNavigationTitle")); this.element.append(this.controlsHost, this.navigationHost, this.resultsHost); this.renderControls(); this.renderResults(); }
    /** New-library creation must settle proposals before opening its original-operation dialog. */
    assertLibraryDeparture(): void {
        this.closeGuard.assertMutable();
        if (this.transientProposal()) throw new UiError(message("shell.closeAuthorInputPending"));
    }
    /** Validate before a caller publishes any matching shell or panel context. */
    assertLibraryChange(library: { root: string; workspaceId?: string } | null): void {
        this.closeGuard.assertMutable();
        if (this.root !== (library?.root ?? null) || this.workspaceId !== library?.workspaceId)
            this.assertLibraryDeparture();
    }
    setLibrary(library: {
        root: string;
        workspaceId?: string;
    } | null): void {
        this.assertLibraryChange(library);
        if (this.root === (library?.root ?? null) && this.workspaceId === library?.workspaceId)
            return;
        this.tagReferences = { nodes: [], state: "unavailable", messages: [] };
        this.refreshQueue.invalidate();
        this.environmentEpoch++;
        this.loading = false;
        this.generation++;
        this.captureDialog?.close();
        this.root = library?.root ?? null;
        this.captureDialog = this.root ? this.captureDialogs.get(this.root) ?? null : null;
        this.workspaceId = library?.workspaceId;
        this.directory = null;
        this.scopeIdentity = null;
        this.navigationDirectory = null;
        this.navigationScopeIdentity = null;
        this.schema = null;
        this.result = null;
        this.conditions = [];
        this.types = [];
        this.selected = null;
        this.savedSelection = null;
        this.savedPlan = null;
        this.exactPlan = null;
        this.advancedDraft = "";
        this.parameterDraft = "{}";
        this.expression = "";
        this.group = undefined;
        this.definitionRevision = null;
        this.loadPreferences();
        this.renderControls();
        this.renderResults();
        if (this.root && this.workspaceId)
            void this.refresh();
    }
    setScope(directory: string | null, identity?: string): void {
        if (this.closeGuard.frozen) return;
        if (directory === this.navigationDirectory && (identity ?? null) === this.navigationScopeIdentity)
            return;
        this.navigationDirectory = directory;
        this.navigationScopeIdentity = identity ?? null;
        this.applyScope(directory, identity);
    }
    private applyScope(directory: string | null, identity?: string): void {
        if (this.closeGuard.frozen) return;
        this.savedSelection = null;
        this.savedPlan = null;
        this.definitionRevision = null;
        this.exactPlan = null;
        this.group = undefined;
        this.directory = directory;
        this.scopeIdentity = identity ?? null;
        this.result = null;
        this.generation++;
        this.loadPreferences();
        this.renderControls();
        if (this.root && this.workspaceId)
            void this.refresh();
    }
    documentChanged(): void {
        if (this.closeGuard.frozen) return;
        const document = this.host.getDocument();
        const selected = document?.root === this.root && document.nodeId && this.workspaceId ? `weftext-node://${this.workspaceId}/${document.nodeId}` : null;
        if (selected !== this.selected) {
            this.selected = selected;
            this.renderResults();
        }
        this.renderControls();
    }
    setSearch(text: string, includeBody: boolean): void {
        if (this.closeGuard.frozen) return;
        if (text === this.search && includeBody === this.includeBody)
            return;
        this.savedSelection = null;
        this.savedPlan = null;
        this.definitionRevision = null;
        this.search = text;
        this.includeBody = includeBody;
        this.renderControls();
        clearTimeout(this.searchTimer);
        if (this.root && this.workspaceId)
            this.searchTimer = setTimeout(() => { void this.refresh(); }, 300);
    }
    setPlan(plan: QueryPlan): void { if (this.closeGuard.frozen) return; this.draftFlags.delete("advanced"); this.adoptPlan(plan); this.renderControls(); void this.refresh(); }
    private adoptPlan(plan: QueryPlan): void { this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; this.exactPlan = structuredClone(plan); this.advancedDraft = JSON.stringify(plan, null, 2); this.directory = plan.scope.directory ?? null; this.scopeIdentity = this.directory?.startsWith("weftext-") ? this.directory : null; this.descendants = plan.scope.descendants; this.lifecycle = plan.scope.lifecycle; this.types = [...plan.types]; this.conditions = []; this.search = ""; this.expression = ""; this.group = plan.group; this.take = plan.take ?? 200; this.loadPreferences(); }
    private libraryIdentity(): string { return this.workspaceId ? `weftext-workspace://${this.workspaceId}` : this.root ?? ""; }
    private preferenceKey(): string { const predicates = [...this.conditions.map(item => item.expr), ...((this.exactPlan?.where ?? this.expression).trim() ? [(this.exactPlan?.where ?? this.expression).trim()] : []), ...(this.search ? [composeWhere(this.search, this.includeBody, [], "")!] : []), ...(this.exactPlan ? [JSON.stringify(this.exactPlan)] : [])]; return `weftext-query-view:${viewPreferenceIdentity(this.libraryIdentity(), this.scopeIdentity ?? this.directory ?? "@library", this.types, predicates, this.savedSelection ?? undefined)}`; }
    private contextChanged(): void { this.loadPreferences(); this.renderControls(); void this.refresh(); }
    private loadPreferences(): void {
        this.preference = defaultPreference();
        this.shortcuts = [];
        this.recent = [];
        this.shortcutError = undefined;
        try {
            this.preference = decodeViewPreference(JSON.parse(localStorage.getItem(this.preferenceKey()) ?? "null"));
        }
        catch { /* Display preferences do not change author semantics. */ }
        try {
            const shortcutKey = `weftext-query-shortcuts:${this.libraryIdentity()}`, raw = localStorage.getItem(shortcutKey), collection = decodeTagShortcutCollection(raw ? JSON.parse(raw) : { version: 2, items: [] });
            if (collection.items.some(item => !item || typeof item.id !== "string" || typeof item.label !== "string" || !["query", "node", "savedQuery", "directory", "tag"].includes(String(item.kind))))
                throw new UiError(message("queryShortcut.invalidItem"));
            this.shortcuts = collection.items as unknown as Shortcut[];
            if (raw)
                localStorage.setItem(shortcutKey, JSON.stringify({ version: 2, items: this.shortcuts }));
        }
        catch (error) {
            this.shortcutError = displayMessage("queryShortcut.recoveryFailed", { p0: uiDisplayParameter((error instanceof Error ? errorUiText(error) : message("error.invalidResponse"))) });
        }
        try {
            const recent: unknown = JSON.parse(localStorage.getItem(`weftext-query-recents:${this.libraryIdentity()}`) ?? "[]");
            if (Array.isArray(recent))
                this.recent = recent.filter((item): item is typeof this.recent[number] => item && typeof item.entityKey === "string" && typeof item.path === "string" && typeof item.label === "string");
        }
        catch { /* Recent navigation is independent from the original shortcuts. */ }
    }
    private savePreferences(bindFields = true): void {
        if (bindFields) {
            this.preference.fieldBindings ??= {};
            for (const key of [...this.preference.columns, this.preference.sort, this.preference.groupField, this.preference.dateField, this.preference.chartField]) {
                const id = this.fieldByKey(key);
                if (id && !this.preference.fieldBindings[key])
                    this.preference.fieldBindings[key] = id;
            }
        }
        localStorage.setItem(this.preferenceKey(), JSON.stringify(this.preference));
    }
    private bindPreferenceField(key: string): void {
        const id = this.fieldByKey(key);
        if (id) {
            this.preference.fieldBindings ??= {};
            this.preference.fieldBindings[key] = id;
        }
    }
    private fieldByKey(key: string): string | undefined { return this.schema?.fields.find(field => field.key === key)?.id; }
    private async resolveScopeLocator(generation: number): Promise<void> {
        return this.closeGuard.run(async () => {
            if (!this.root || !this.scopeIdentity)
                return;
            const targetRef = this.scopeIdentity;
            const target = await this.host.request<NavigationTarget>("/api/navigation/resolve", { root: this.root, targetRef });
            if (generation !== this.generation)
                return;
            if (target.status !== "managed" || target.targetRef !== targetRef || !target.path)
                throw new UiError(message("queryValidation.directoryIdentityUnavailable"));
            this.directory = target.path;
        });
    }
    private planInputs(): QueryPlanInputs {
        return structuredClone({ scope: this.scopeIdentity ?? this.directory, descendants: this.descendants, lifecycle: this.lifecycle,
            types: this.types, search: this.search, includeBody: this.includeBody, conditions: this.conditions.map(({ expr }) => ({ expr })), expression: this.expression,
            exactPlan: this.exactPlan, group: this.group, take: this.take, preference: this.preference });
    }
    getPlan(): QueryPlan { return this.savedSelection && this.savedPlan ? structuredClone(this.savedPlan) : planForInputs(this.planInputs(), this.schema); }
    private refreshIntent(): QueryRefreshIntent {
        if (!this.root || !this.workspaceId)
            throw new UiError(message("queryValidation.libraryIdentityUnavailable"));
        return { root: this.root, workspaceId: this.workspaceId, epoch: this.environmentEpoch, inputs: this.planInputs(),
            selection: this.savedSelection ? structuredClone(this.savedSelection) : null, parameters: parseQueryParameters(this.parameterDraft) };
    }
    private intentKey(intent: QueryRefreshIntent): string {
        return queryRefreshKey({ root: intent.root, workspaceId: intent.workspaceId, epoch: intent.epoch,
            selection: intent.selection ? { owner: intent.selection.owner, key: intent.selection.key, mode: intent.selection.mode, ...(intent.selection.mode === "pinned" ? { revision: intent.selection.revision } : {}) } : null,
            inputs: intent.selection ? undefined : queryPlanRefreshInput(intent.inputs),
            parameters: Object.entries(intent.parameters).sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0).map(([name, value]) => [name, canonicalNative(value)]) });
    }
    /** A successful author/configuration mutation retires read work, never a Draft edit. */
    environmentChanged(root: string): void {
        if (this.closeGuard.frozen) return;
        const coordinate = (value: string) => value.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
        if (!this.root || coordinate(this.root) !== coordinate(root))
            return;
        const pending = this.refreshQueue.invalidate();
        this.environmentEpoch++;
        this.generation++;
        this.loading = false;
        this.tagReferences = { nodes: [], state: "unavailable", messages: [] };
        if (this.result)
            this.result = { ...this.result, freshness: "stale", superseded: true };
        if (pending) {
            pending.epoch = this.environmentEpoch;
            this.loading = true;
            void this.closeGuard.run(() => this.refreshQueue.request(this.intentKey(pending), pending));
        }
        this.renderResults();
    }
    async refresh(): Promise<void> {
        return this.closeGuard.run(async () => {
            clearTimeout(this.searchTimer);
            if (!this.root || !this.workspaceId) {
                this.refreshQueue.invalidate();
                this.loading = false;
                this.renderResults();
                return;
            }
            try {
                const intent = this.refreshIntent();
                this.generation++;
                this.loading = true;
                this.error = null;
                if (this.result)
                    this.result = { ...this.result, freshness: "stale", superseded: true };
                this.renderResults();
                await this.refreshQueue.request(this.intentKey(intent), intent);
            }
            catch (error) {
                this.refreshQueue.invalidate();
                this.loading = false;
                this.error = error instanceof Error ? errorUiText(error) : message("error.invalidResponse");
                this.renderResults();
            }
        });
    }
    private async resolveIntentScope(root: string, scope: string | null, current: () => boolean): Promise<string | null> {
        return this.closeGuard.run(async () => {
            if (!scope || !scope.startsWith("weftext-"))
                return scope;
            const target = await this.host.request<NavigationTarget>("/api/navigation/resolve", { root, targetRef: scope });
            if (!current())
                return null;
            if (target.status !== "managed" || target.targetRef !== scope || !target.path)
                throw new UiError(message("queryValidation.directoryIdentityUnavailable"));
            return target.path;
        });
    }
    private async executeRefresh(intent: QueryRefreshIntent, selected: () => boolean): Promise<void> {
        return this.closeGuard.run(async () => {
            const key = this.intentKey(intent);
            const current = (): boolean => {
                if (this.closeGuard.frozen || !selected())
                    return false;
                try {
                    return this.intentKey(this.refreshIntent()) === key;
                }
                catch {
                    return false;
                }
            };
            try {
                if (!current())
                    return;
                const schemaResult = await this.host.request<DataSchema | {
                    schema: DataSchema;
                }>("/api/data/schema", { root: intent.root });
                if (!current())
                    return;
                const schema = "schema" in schemaResult ? schemaResult.schema : schemaResult;
                const directory = await this.resolveIntentScope(intent.root, intent.inputs.scope, current);
                if (!current())
                    return;
                this.schema = schema;
                if (intent.inputs.scope?.startsWith("weftext-"))
                    this.directory = directory;
                this.renderControls();
                if (intent.selection) {
                    const selection = intent.selection;
                    const response = await this.host.request<QueryResult & {
                        result?: QueryResult;
                        definitionRevision?: string;
                        definition?: SavedQuery;
                        definitionChanged?: boolean;
                    }>("/api/query/definition/run", { root: intent.root, owner: selection.owner, parameters: intent.parameters, selection: { mode: selection.mode, key: selection.key, ...(selection.mode === "pinned" ? { revision: selection.revision } : {}) } });
                    if (!current())
                        return;
                    const savedPlan = response.definition?.plan ?? null;
                    const savedDirectory = await this.resolveIntentScope(intent.root, savedPlan?.scope.directory ?? null, current);
                    if (!current())
                        return;
                    const previous = this.definitionRevision;
                    this.definitionRevision = response.definitionRevision ?? selection.revision ?? null;
                    this.result = decodeLocalQueryResult(response.result);
                    this.savedPlan = savedPlan;
                    this.resultPlan = savedPlan;
                    if (savedPlan) {
                        this.exactPlan = structuredClone(savedPlan);
                        this.directory = savedDirectory;
                        this.scopeIdentity = savedPlan.scope.directory?.startsWith("weftext-") ? savedPlan.scope.directory : null;
                        this.descendants = savedPlan.scope.descendants;
                        this.lifecycle = savedPlan.scope.lifecycle;
                        this.types = [...savedPlan.types];
                        this.conditions = [];
                        this.search = "";
                        this.expression = "";
                        this.group = savedPlan.group;
                    }
                    if (selection.mode === "current" && (response.definitionChanged || previous && this.definitionRevision !== previous))
                        this.host.notify(message("queryStatus.savedDefinitionUpdated"));
                    else if (selection.mode === "pinned" && response.definitionChanged)
                        this.host.notify(message("queryStatus.pinnedDefinitionDiffers"));
                }
                else {
                    const errors = intent.inputs.exactPlan ? [] : preferenceFieldErrors(intent.inputs.preference, schema);
                    if (errors.length)
                        throw new UiError(joinUiText(errors, "；"));
                    const plan = planForInputs(intent.inputs, schema), result = await this.host.request<QueryResult>("/api/query/run", { root: intent.root, plan, parameters: intent.parameters });
                    if (!current())
                        return;
                    this.result = decodeLocalQueryResult(result);
                    this.resultPlan = plan;
                    if (this.selected && !result.rows.some(row => (row.entityKey ?? row.key) === this.selected))
                        this.host.notify(result.coverage === "complete" && result.freshness === "current" ? message("queryStatus.selectionLeftResult") : message("queryStatus.selectionCoverageUnknown"));
                }
                if (intent.inputs.search === this.search) this.draftFlags.delete("search");
                if (intent.inputs.expression === this.expression) this.draftFlags.delete("expression");
                this.previewCache.clear();
                this.renderNavigation();
            }
            catch (error) {
                if (current()) {
                    this.error = error instanceof Error ? errorUiText(error) : message("error.invalidResponse");
                    if (intent.selection)
                        this.result = null;
                }
            }
            finally {
                if (!this.closeGuard.frozen && selected()) {
                    this.loading = false;
                    this.renderControls();
                    this.renderResults();
                }
            }
        });
    }
    /** Controls own their native nodes for the library lifetime; read delivery only patches state. */
    private renderControls(): void {
        if (this.closeGuard.frozen) return;
        const context = JSON.stringify([this.root, this.workspaceId]), reset = this.controlsContext !== context;
        this.controlsContext = context;
        if (!this.controlUpdates)
            this.mountControls();
        for (const update of this.controlUpdates!)
            update(reset);
        this.renderRegisteredNavigation();
        this.renderPeriodNavigation();
    }
    private draftInput(input: HTMLInputElement | HTMLTextAreaElement, read: () => string, write: (value: string) => void): (reset: boolean) => void {
        let composing = false;
        input.oninput = () => { if (!this.closeGuard.frozen) write(input.value); };
        input.addEventListener("compositionstart", () => { if (!this.closeGuard.frozen) composing = true; });
        input.addEventListener("compositionend", () => { if (!this.closeGuard.frozen) { composing = false; write(input.value); } });
        return reset => {
            if ((reset || !composing) && input.value !== read())
                input.value = read();
        };
    }
    private mountControls(): void {
        this.controlUpdates = [];
        const heading = ui("div", "query-heading"), capture = this.action(message("queryCapture.title"), () => this.showCapture(), "data-button primary");
        heading.append(ui("h2", "", message("queryPanel.libraryTitle")), this.action(message("common.refresh"), () => this.refresh(), "data-button compact"), capture,
            this.action(message("common.cancel"), () => this.discardQueryProposals()),
            this.action(message("queryEdit.discardAllFieldInputs"), () => { this.inlineDrafts.clear(); this.renderResults(); }));
        const savedNotice = ui("p", "query-result-status"), copy = this.action(message("queryPanel.copyDefinitionToTemporary"), () => {
            const plan = this.savedPlan;
            this.savedSelection = null;
            this.savedPlan = null;
            this.definitionRevision = null;
            if (plan)
                this.setPlan(plan);
            else {
                this.renderControls();
                void this.refresh();
            }
        });
        const exactNotice = ui("p", "query-result-status", message("queryPanel.exactPlanHelp"));
        const searchForm = ui("form", "query-search-form"), search = ui("input", "data-input"), run = ui("button", "data-button", message("queryPanel.queryDisclosure"));
        search.type = "search";
        setUiAttr(search, "placeholder", message("queryPanel.searchTitlePlaceholder"));
        setUiAttr(search, "aria-label", message("queryPanel.searchTitleLabel"));
        run.type = "submit";
        const updateSearch = this.draftInput(search, () => this.search, value => { this.search = value; this.draftFlags.add("search"); });
        searchForm.append(search, run, this.action(message("common.cancel"), () => { this.search = ""; search.value = ""; this.draftFlags.delete("search"); }));
        searchForm.onsubmit = event => { event.preventDefault(); void this.refresh(); };
        const scopeRow = ui("div", "query-scope-row"), scopeText = ui("span", "query-scope"), all = this.action(message("queryPanel.wholeLibrary"), () => this.applyScope(null), "data-button compact");
        const descendants = ui("input"), body = ui("input"), lifecycle = selectControl([["live", message("queryPanel.liveNotes")], ["trash", message("queryPanel.readOnlyTrash")]], this.lifecycle);
        descendants.type = body.type = "checkbox";
        descendants.onchange = () => { this.descendants = descendants.checked; void this.refresh(); };
        body.onchange = () => { this.includeBody = body.checked; void this.refresh(); };
        lifecycle.onchange = () => { this.lifecycle = lifecycle.value as "live" | "trash"; void this.refresh(); };
        scopeRow.append(scopeText, all, labeled(message("queryPanel.includeDescendants"), descendants), labeled(message("queryPanel.searchBody"), body), labeled(message("queryPanel.scopeLabel"), lifecycle));
        const chips = ui("div", "query-chips");
        let chipsKey = "";
        const presentation = ui("div", "query-presentation"), view = selectControl([["list", message("queryView.list")], ["table", message("queryView.table")], ["board", message("queryView.board")], ["calendar", message("queryView.calendar")], ["chart", message("queryView.chart")]], this.preference.view);
        view.onchange = () => { this.preference.view = view.value as ResultView; this.savePreferences(); this.renderControls(); void this.refresh(); };
        const density = selectControl([["comfortable", message("queryView.detailedDensity")], ["compact", message("queryView.compactDensity")]], this.preference.density);
        density.onchange = () => { this.preference.density = density.value as ViewPreference["density"]; this.savePreferences(false); this.renderResults(); };
        const sort = selectControl([], "");
        sort.onchange = () => {
            const field = this.schema?.fields.find(field => field.id === sort.value);
            if (sort.value !== "@title" && !field)
                return;
            this.preference.sort = field?.key ?? "title";
            this.bindPreferenceField(this.preference.sort);
            this.savePreferences();
            void this.refresh();
        };
        const descending = this.action("", () => { this.preference.descending = !this.preference.descending; this.savePreferences(); this.renderControls(); void this.refresh(); }, "data-button compact");
        const summary = ui("input"), thumbnails = ui("input");
        summary.type = thumbnails.type = "checkbox";
        summary.onchange = () => { this.preference.summaries = summary.checked; this.savePreferences(); void this.refresh(); };
        thumbnails.onchange = () => { this.preference.thumbnails = thumbnails.checked; this.savePreferences(false); this.renderResults(); };
        presentation.append(labeled(message("queryView.disclosure"), view), labeled(message("queryView.densityLabel"), density), labeled(message("queryPanel.sortLabel"), sort), descending, labeled(message("queryView.bodySummaries"), summary), labeled(message("queryView.thumbnails"), thumbnails));
        this.controlsHost.append(heading, savedNotice, copy, exactNotice, searchForm, scopeRow, chips, presentation);
        this.controlUpdates.push(reset => {
            capture.disabled = !this.workspaceId;
            const selected = this.savedSelection;
            savedNotice.hidden = copy.hidden = !selected;
            setUiText(savedNotice, selected ? displayMessage("queryPanel.savedSelectionSummary", { p0: uiDisplayParameter(selected.mode === "pinned" ? message("queryPanel.pinnedDefinition") : message("queryPanel.currentDefinition")), p1: uiDisplayParameter(selected.label), p2: uiDisplayParameter(this.definitionRevision ? ` · ${this.definitionRevision}` : "") }) : "");
            exactNotice.hidden = !this.exactPlan || !!selected;
            updateSearch(reset);
            search.disabled = run.disabled = !!selected;
            setUiText(scopeText, this.directory ? displayMessage("queryPanel.directoryChip", { p0: uiDisplayParameter(this.directory) }) : message("queryPanel.entireLibrary"));
            all.hidden = !this.directory;
            descendants.checked = this.descendants;
            body.checked = this.includeBody;
            lifecycle.value = this.lifecycle;
            descendants.disabled = body.disabled = lifecycle.disabled = !!selected || !!this.exactPlan;
            // Display labels are reactive local UI values, never serializable Query state.
            const nextChips = queryRefreshKey([this.types, this.conditions.map(({ id, expr, tag }) => [id, expr, tag?.reference, tag?.mode])]);
            if (nextChips !== chipsKey) {
                chipsKey = nextChips;
                chips.replaceChildren();
                this.types.forEach(token => chips.append(this.action(displayMessage("queryPanel.typeChip", { p0: uiDisplayParameter(typeNames[token] ?? token) }), () => { this.types = this.types.filter(item => item !== token); this.contextChanged(); }, "query-chip")));
                this.conditions.forEach(condition => {
                    chips.append(this.action(() => `${uiTextValue(condition.label)} ×`, () => { this.conditions = this.conditions.filter(item => item.id !== condition.id); this.contextChanged(); }, "query-chip"));
                    if (condition.tag)
                        chips.append(this.action(condition.tag.mode === "subtree" ? message("queryPanel.switchToExactTag") : message("queryPanel.switchToSubtreeTag"), () => { const tag = condition.tag!; this.conditions = this.conditions.filter(item => item.id !== condition.id); this.applyTag(tag.reference, tag.label, tag.mode === "subtree" ? "exact" : "subtree"); }, "data-button compact"));
                });
            }
            for (const button of chips.querySelectorAll("button"))
                button.disabled = !!selected;
            view.value = this.preference.view;
            density.value = this.preference.density;
            summary.checked = this.preference.summaries;
            thumbnails.checked = this.preference.thumbnails;
            const sortValue = this.preference.sort === "title" ? "@title" : this.preference.fieldBindings?.[this.preference.sort] ?? this.fieldByKey(this.preference.sort) ?? this.preference.sort;
            updateQueryChoices(sort, [{ value: "@title", label: message("queryColumn.title") }, ...(this.schema?.fields.filter(field => { const shape = resolveShape(field.shape, this.schema!); return shape.kind === "scalar" || shape.kind === "enum"; }).map(field => ({ value: field.id, label: fieldNames[field.key] ?? field.key })) ?? [])], sortValue);
            sort.disabled = descending.disabled = !!selected || !!this.exactPlan;
            setUiText(descending, this.preference.descending ? message("queryPanel.descendingToggle") : message("queryPanel.ascendingToggle"));
        });
        this.mountBuilderControls();
        this.mountAdvancedControls();
        this.mountSavedControls();
        this.mountShortcutControls();
        this.controlsHost.append(this.registeredHost, this.periodHost, this.savedHost, this.shortcutsHost);
    }
    private mountBuilderControls(): void {
        const builder = ui("details", "query-builder");
        builder.append(ui("summary", "", message("queryFilter.disclosure")));
        const type = selectControl([], "");
        type.onchange = () => {
            const definition = this.schema?.types.find(item => item.id === type.value);
            if (!definition || !this.schema)
                return;
            const token = Object.entries(this.schema.aliases).find(([, id]) => id === definition.id)?.[0] ?? definition.id;
            if (!this.types.includes(token))
                this.types.push(token);
            type.value = "";
            this.contextChanged();
        };
        builder.append(labeled(message("queryFilter.typeLabel"), type));
        const filterForm = ui("form", "query-filter-builder"), field = selectControl([], ""), operator = selectControl([["==", message("queryOperator.equals")], ["!=", message("queryOperator.notEquals")], [">", message("queryOperator.greaterThan")], [">=", message("queryOperator.atLeast")], ["<", message("queryOperator.lessThan")], ["<=", message("queryOperator.atMost")], ["contains", message("queryOperator.containsText")], ["absent", message("queryValue.absent")], ["null", message("queryOperator.explicitNone")]], "==");
        const value = ui("input", "data-input"), branch = selectControl([], ""), branchLabel = labeled(message("queryFilter.valueTypeLabel"), branch), filterMessage = ui("p", "data-warning");
        setUiAttr(value, "aria-label", message("queryFilter.exactValueLabel"));
        setUiAttr(value, "placeholder", message("queryFilter.exactValue"));
        const updateBranch = (reset: boolean): void => {
            const schema = this.schema, selected = schema?.fields.find(item => item.id === field.value);
            const shape = selected && schema ? resolveShape(selected.shape, schema) : undefined;
            const variants = shape?.kind === "union" ? shape.variants.map(variant => resolveShape(variant, schema!)).filter(item => item.kind === "scalar" || item.kind === "enum") : [];
            if (reset)
                branch.value = "";
            updateQueryChoices(branch, variants.map(item => ({ value: queryRefreshKey(item), label: item.kind === "scalar" ? item.semantic : "enum" })));
            branchLabel.hidden = shape?.kind !== "union";
        };
        branch.onchange = () => this.renderControls();
        operator.onchange = () => this.renderControls();
        field.onchange = () => { branch.value = ""; updateBranch(true); this.renderControls(); };
        const add = ui("button", "data-button", message("queryFilter.add"));
        add.type = "submit";
        filterForm.append(labeled(message("queryFilter.fieldLabel"), field), labeled(message("queryFilter.conditionLabel"), operator), branchLabel, value, add);
        for (const control of [field, operator, branch, value]) {
            control.addEventListener("input", () => this.draftFlags.add("filter"));
            control.addEventListener("change", () => this.draftFlags.add("filter"));
        }
        builder.append(filterForm, filterMessage, this.action(message("common.cancel"), () => { value.value = ""; this.draftFlags.delete("filter"); }));
        filterForm.onsubmit = event => {
            event.preventDefault();
            try {
                const schema = this.schema, selected = schema?.fields.find(item => item.id === field.value);
                if (!schema || !selected)
                    throw new UiError(message("queryFilter.unavailableOriginalField"));
                const expr = fieldExpr(selected.id);
                let condition: string;
                if (operator.value === "absent")
                    condition = `isAbsent(${expr})`;
                else if (operator.value === "null")
                    condition = `isNull(${expr})`;
                else {
                    const shape = resolveShape(selected.shape, schema), resolved = shape.kind === "union" ? shape.variants.map(item => resolveShape(item, schema)).find(item => queryRefreshKey(item) === branch.value) : shape;
                    if (!resolved || !["scalar", "enum"].includes(resolved.kind))
                        throw new UiError(message("queryFilter.unavailableOriginalBranch"));
                    const native = scalarFromInput(resolved, schema, value.value);
                    if (operator.value === "contains") {
                        if (native.kind !== "text")
                            throw new UiError(message("queryValidation.containsTextFieldRequired"));
                        condition = `${expr}.contains(${JSON.stringify(native.value)})`;
                    }
                    else
                        condition = `${expr} ${operator.value} ${shapedQueryLiteral(resolved, schema, native)}`;
                }
                const selectedKey = selected.key, operatorKey = operator.value, rawValue = value.value;
                const operatorLabels: Readonly<Record<string, UiText>> = { "==": message("queryOperator.equals"), "!=": message("queryOperator.notEquals"), ">": message("queryOperator.greaterThan"), ">=": message("queryOperator.atLeast"), "<": message("queryOperator.lessThan"), "<=": message("queryOperator.atMost"), contains: message("queryOperator.containsText"), absent: message("queryValue.absent"), null: message("queryOperator.explicitNone") };
                this.draftFlags.delete("filter");
                this.conditions.push({ id: crypto.randomUUID(), label: () => `${uiTextValue(fieldNames[selectedKey] ?? selectedKey)} ${uiTextValue(operatorLabels[operatorKey] ?? operatorKey)} ${rawValue}`, expr: condition });
                this.contextChanged();
            }
            catch (error) {
                this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
            }
        };
        const expression = ui("textarea", "data-input query-expression");
        expression.rows = 3;
        setUiAttr(expression, "placeholder", 'phones.exists(p, p.number == "138…" && p.label == "工作")');
        setUiAttr(expression, "aria-label", message("queryFilter.advancedDisclosure"));
        const updateExpression = this.draftInput(expression, () => this.expression, input => { this.expression = input; this.draftFlags.add("expression"); });
        builder.append(labeled(message("queryFilter.celLabel"), expression), this.action(message("queryFilter.applyCel"), () => this.refresh()), this.action(message("common.cancel"), () => { this.expression = ""; expression.value = ""; this.draftFlags.delete("expression"); }));
        const columns = ui("fieldset", "query-columns");
        columns.append(ui("legend", "", message("queryFilter.projectionLabel")));
        const columnRows = new Map<string, {
            label: HTMLElement;
            input: HTMLInputElement;
            key: string;
        }>();
        builder.append(columns);
        const viewField = selectControl([], ""), viewFieldLabel = labeled("", viewField);
        viewField.classList.add("query-view-field");
        viewField.onchange = () => {
            const property = this.preference.view === "board" ? "groupField" : this.preference.view === "calendar" ? "dateField" : "chartField";
            if (this.exactPlan || this.savedSelection || this.group)
                this.preference[property] = viewField.value;
            else {
                const selected = this.schema?.fields.find(item => item.id === viewField.value);
                if (!selected && viewField.value !== "@count")
                    return;
                this.preference[property] = selected?.key ?? "count";
                this.bindPreferenceField(this.preference[property]);
            }
            this.savePreferences();
            void this.refresh();
        };
        builder.append(viewFieldLabel);
        const aggregateForm = ui("form", "query-aggregate-builder"), groupKey = selectControl([], ""), op = selectControl([["count", message("queryAggregate.noteCount")], ["sum", message("queryAggregate.sum")], ["min", message("queryAggregate.minimum")], ["max", message("queryAggregate.maximum")]], "count"), amount = selectControl([], "");
        const aggregateSubmit = ui("button", "data-button", message("queryAggregate.run")), aggregateMessage = ui("p", "data-warning");
        aggregateSubmit.type = "submit";
        aggregateForm.append(labeled(message("queryAggregate.groupLabel"), groupKey), labeled(message("queryValue.aggregateFallback"), op), labeled(message("queryAggregate.numericFieldLabel"), amount), aggregateSubmit);
        for (const control of [groupKey, op, amount]) control.addEventListener("change", () => this.draftFlags.add("aggregate"));
        builder.append(aggregateForm, aggregateMessage, this.action(message("common.cancel"), () => { groupKey.value = amount.value = ""; op.value = "count"; this.draftFlags.delete("aggregate"); this.renderControls(); }));
        aggregateForm.onsubmit = event => {
            event.preventDefault();
            const schema = this.schema, groupField = schema?.fields.find(item => item.id === groupKey.value), amountField = schema?.fields.find(item => item.id === amount.value);
            if (!schema || groupKey.value && !groupField || op.value !== "count" && !amountField) {
                this.host.notify(message("queryAggregate.unavailableOriginalField"), true);
                return;
            }
            this.draftFlags.delete("aggregate");
            this.group = { keys: groupField ? [{ name: groupField.key, expr: fieldExpr(groupField.id) }] : [], aggregates: [{ name: op.value === "count" ? "count" : "amount", op: op.value as "count", ...(op.value === "count" ? {} : { expr: fieldExpr(amountField!.id) }) }] };
            this.preference.chartField = op.value === "count" ? "count" : "amount";
            this.renderControls();
            void this.refresh();
        };
        const grouped = ui("div"), returnRows = this.action(message("queryAggregate.returnToNotes"), () => { this.group = undefined; this.renderControls(); void this.refresh(); });
        grouped.append(ui("p", "data-help", message("queryAggregate.partialInputHelp")), returnRows);
        builder.append(grouped);
        const limit = ui("input", "data-input");
        limit.type = "number";
        limit.min = "1";
        limit.max = "10000";
        limit.addEventListener("input", () => this.draftFlags.add("limit"));
        limit.onchange = () => {
            const next = Number(limit.value);
            if (Number.isInteger(next) && next >= 1 && next <= 10000) {
                this.draftFlags.delete("limit");
                this.take = next;
                void this.refresh();
            }
        };
        let priorLimit = this.take;
        limit.value = String(priorLimit);
        builder.append(labeled(message("queryPanel.displayLimitLabel"), limit), this.action(message("common.cancel"), () => { limit.value = String(this.take); this.draftFlags.delete("limit"); }), this.action(message("queryView.resetLocal"), () => { this.preference = defaultPreference(); this.savePreferences(); this.renderControls(); void this.refresh(); }));
        this.controlsHost.append(builder);
        this.controlUpdates!.push(reset => {
            const schema = this.schema, fields = schema?.fields ?? [], locked = !schema || !!this.savedSelection || !!this.exactPlan;
            builder.hidden = !schema;
            if (reset) {
                field.value = groupKey.value = amount.value = "";
                operator.value = "==";
                op.value = "count";
                value.value = "";
                builder.open = false;
            }
            updateQueryChoices(type, [{ value: "", label: message("queryFilter.addTypePlaceholder") }, ...(schema?.types.map(item => { const alias = Object.entries(schema.aliases).find(([, id]) => id === item.id)?.[0] ?? item.id; return { value: item.id, label: typeNames[alias] ?? alias }; }) ?? [])]);
            const choices = fields.map(item => ({ value: item.id, label: fieldNames[item.key] ?? item.key }));
            const fieldValid = updateQueryChoices(field, choices), groupValid = updateQueryChoices(groupKey, [{ value: "", label: message("queryAggregate.noFieldGrouping") }, ...choices]), amountValid = updateQueryChoices(amount, choices);
            updateBranch(reset);
            filterMessage.hidden = fieldValid;
            setUiText(filterMessage, fieldValid ? "" : message("queryFilter.unavailableRetainedValue"));
            aggregateMessage.hidden = groupValid && (op.value === "count" || amountValid);
            setUiText(aggregateMessage, aggregateMessage.hidden ? "" : message("queryAggregate.unavailableRetainedChoice"));
            updateExpression(reset);
            const wanted = new Set(fields.map(item => item.id));
            for (const item of fields) {
                let row = columnRows.get(item.id);
                if (!row) {
                    const input = ui("input");
                    input.type = "checkbox";
                    row = { input, label: labeled(fieldNames[item.key] ?? item.key, input), key: item.key };
                    columnRows.set(item.id, row);
                    columns.append(row.label);
                    input.onchange = () => {
                        const selected = this.schema?.fields.find(field => field.id === item.id);
                        if (!selected) {
                            input.checked = !input.checked;
                            this.host.notify(message("queryFilter.unavailableProjection"), true);
                            return;
                        }
                        if (input.checked) {
                            if (!this.preference.columns.includes(selected.key))
                                this.preference.columns.push(selected.key);
                            this.bindPreferenceField(selected.key);
                        }
                        else
                            this.preference.columns = this.preference.columns.filter(key => key !== selected.key);
                        this.savePreferences();
                        void this.refresh();
                    };
                }
                row.key = item.key;
                if (row.label.firstChild?.nodeType === 3)
                    setUiText(row.label.firstChild, fieldNames[item.key] ?? item.key);
                setUiAttr(row.label, "title", "");
                const binding = this.preference.fieldBindings?.[item.key];
                row.input.checked = this.preference.columns.includes(item.key) && (!binding || binding === item.id);
                row.input.disabled = locked;
                row.input.setAttribute("aria-invalid", "false");
            }
            for (const [id, row] of columnRows)
                if (!wanted.has(id)) {
                    const selected = this.preference.columns.includes(row.key) && this.preference.fieldBindings?.[row.key] === id;
                    if (selected) {
                        row.input.checked = true;
                        row.input.disabled = true;
                        row.input.setAttribute("aria-invalid", "true");
                        setUiAttr(row.label, "title", message("queryFilter.unavailableProjectionChoice"));
                    }
                    else {
                        row.label.remove();
                        columnRows.delete(id);
                    }
                }
            const property = this.preference.view === "board" ? "groupField" : this.preference.view === "calendar" ? "dateField" : "chartField", selectedViewField = this.preference[property];
            viewFieldLabel.hidden = !["board", "calendar", "chart"].includes(this.preference.view);
            const caption = this.preference.view === "board" ? message("queryView.boardGroupFieldLabel") : this.preference.view === "calendar" ? message("queryView.calendarDateFieldLabel") : message("queryView.chartValueFieldLabel");
            if (viewFieldLabel.firstChild?.nodeType === 3)
                setUiText(viewFieldLabel.firstChild, caption);
            const derived = !!this.exactPlan || !!this.savedSelection || !!this.group;
            const viewChoices = derived ? [...new Set([...(this.resultPlan?.select ?? this.getPlan().select).map(item => item.name), ...Object.keys(this.result?.rows[0]?.cells ?? {})])].filter(name => !["title", "summary", "types"].includes(name)).map(name => ({ value: name, label: fieldNames[name] ?? name })) : [...(this.preference.view === "chart" ? [{ value: "@count", label: message("queryAggregate.countOutput") }] : []), ...choices];
            updateQueryChoices(viewField, viewChoices, derived ? selectedViewField : selectedViewField === "count" ? "@count" : this.preference.fieldBindings?.[selectedViewField] ?? this.fieldByKey(selectedViewField) ?? selectedViewField);
            for (const control of builder.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | HTMLButtonElement>("input,select,textarea,button"))
                if (!control.classList.contains("query-view-field") && !columns.contains(control))
                    control.disabled = locked;
            add.disabled = locked || !fieldValid || !field.value || operator.value !== "absent" && operator.value !== "null" && !branchLabel.hidden && branch.getAttribute("aria-invalid") === "true";
            aggregateSubmit.disabled = locked || !groupValid || op.value !== "count" && (!amountValid || !amount.value);
            grouped.hidden = !this.group;
            if (reset || priorLimit !== this.take) {
                limit.value = String(this.take);
                priorLimit = this.take;
            }
        });
        op.onchange = () => this.renderControls();
        groupKey.onchange = amount.onchange = () => this.renderControls();
    }
    private mountAdvancedControls(): void {
        const section = ui("details", "query-advanced-plan");
        section.append(ui("summary", "", message("queryAdvanced.disclosure")));
        section.ontoggle = () => { this.advancedOpen = section.open; };
        const planInput = ui("textarea", "data-input query-plan-input"), parameterInput = ui("textarea", "data-input query-expression");
        planInput.rows = 14;
        parameterInput.rows = 4;
        planInput.spellcheck = parameterInput.spellcheck = false;
        setUiAttr(planInput, "aria-label", message("queryAdvanced.planLabel"));
        setUiAttr(parameterInput, "aria-label", message("queryAdvanced.parametersLabel"));
        let planInitialized = false;
        planInput.addEventListener("compositionstart", () => { planInitialized = true; });
        const updatePlan = this.draftInput(planInput, () => this.advancedDraft, value => { this.advancedDraft = value; this.draftFlags.add("advanced"); planInitialized = true; });
        const updateParameters = this.draftInput(parameterInput, () => this.parameterDraft, value => { this.parameterDraft = value; this.draftFlags.add("parameters"); });
        const feedback = ui("p", "data-help", message("queryAdvanced.planHelp"));
        feedback.setAttribute("aria-live", "polite");
        section.append(this.action(message("queryAdvanced.loadCurrentPlan"), () => { this.draftFlags.delete("advanced"); this.advancedDraft = JSON.stringify(this.getPlan(), null, 2); planInput.value = this.advancedDraft; planInitialized = true; }), labeled(message("queryAdvanced.planField"), planInput), labeled(message("queryAdvanced.parametersField"), parameterInput));
        section.append(this.action(message("queryAdvanced.runPlan"), async () => {
            try {
                const plan = parseQueryPlan(planInput.value);
                parseQueryParameters(parameterInput.value);
                this.parameterDraft = parameterInput.value;
                this.draftFlags.delete("parameters");
                this.advancedOpen = true;
                this.setPlan(plan);
            }
            catch (error) {
                feedback.className = "data-warning";
                setUiText(feedback, error instanceof Error ? errorUiText(error) : message("error.invalidResponse"));
            }
        }, "data-button primary"), this.action(message("queryAdvanced.rerunParameters"), async () => {
            try {
                parseQueryParameters(parameterInput.value);
                this.parameterDraft = parameterInput.value;
                this.draftFlags.delete("parameters");
                await this.refresh();
            }
            catch (error) {
                feedback.className = "data-warning";
                setUiText(feedback, error instanceof Error ? errorUiText(error) : message("error.invalidResponse"));
            }
        }));
        const back = this.action(message("queryAdvanced.returnToVisual"), () => { this.exactPlan = null; this.group = undefined; this.expression = ""; this.conditions = []; this.loadPreferences(); this.renderControls(); void this.refresh(); });
        const example = this.action(message("queryAdvanced.loadUnnestExample"), () => {
            const multivalue = this.schema?.fields.find(field => field.key === "phones" && resolveShape(field.shape, this.schema!).kind === "list") ?? this.schema?.fields.find(field => resolveShape(field.shape, this.schema!).kind === "list");
            if (!multivalue)
                return;
            const plan: QueryPlan = { scope: { ...(this.scopeIdentity || this.directory ? { directory: this.scopeIdentity ?? this.directory! } : {}), descendants: this.descendants, lifecycle: this.lifecycle }, types: [...this.types], select: [{ name: "title", expr: "title" }, { name: "item", expr: "item" }], order: [], unnest: { array: fieldExpr(multivalue.id), binding: "item" }, take: this.take };
            this.advancedDraft = JSON.stringify(plan, null, 2);
            this.draftFlags.add("advanced");
            planInput.value = this.advancedDraft;
            planInitialized = true;
        });
        section.append(this.action(message("common.cancel"), () => {
            this.advancedDraft = JSON.stringify(this.getPlan(), null, 2); this.parameterDraft = "{}";
            this.draftFlags.delete("advanced"); this.draftFlags.delete("parameters");
            planInput.value = this.advancedDraft; parameterInput.value = this.parameterDraft;
        }), back, example, feedback, ui("p", "data-help", message("queryAdvanced.nativeSyntaxHelp")));
        this.controlsHost.append(section);
        this.controlUpdates!.push(reset => {
            if (reset)
                planInitialized = false;
            if (!planInitialized && !this.advancedDraft && this.schema) {
                this.advancedDraft = JSON.stringify(this.getPlan(), null, 2);
                planInput.value = this.advancedDraft;
                planInitialized = true;
            }
            else if (reset || planInitialized || this.advancedDraft)
                updatePlan(reset);
            updateParameters(reset);
            if (reset && section.open !== this.advancedOpen)
                section.open = this.advancedOpen;
            back.hidden = !this.exactPlan || !!this.savedSelection;
            example.hidden = !this.schema?.fields.some(field => resolveShape(field.shape, this.schema!).kind === "list");
        });
    }
    private renderNavigation(): void {
        this.navigationHost.replaceChildren();
        if (!this.result || !this.schema)
            return;
        const data = this.result;
        const header = ui("p", "data-help", data.facets ? data.coverage === "complete" ? message("queryFacets.currentScopeTitle") : message("queryFacets.currentScopeKnownTitle") : message("queryFacets.loadedResultsTitle"));
        this.navigationHost.append(header);
        const facets: QueryFacet[] = data.facets ?? ["tags", "status", "project"].flatMap(key => {
            const id = this.fieldByKey(key);
            if (!id)
                return [];
            return distinctFacetCounts(data.rows, key).map(facet => ({ fieldId: id, key, ...facet }));
        });
        const types = distinctFacetCounts(data.rows, "types");
        types.forEach(facet => {
            if (facet.value?.kind !== "text")
                return;
            const token = facet.value.value;
            this.navigationHost.append(this.action(() => `${uiTextValue(typeNames[token] ?? token)} · ${uiTextValue(facet.count)}`, () => {
                this.savedSelection = null;
                this.savedPlan = null;
                this.definitionRevision = null;
                if (!this.types.includes(token))
                    this.types.push(token);
                this.contextChanged();
            }, "query-facet"));
        });
        for (const facet of facets) {
            const label = facet.missing ? message("queryValue.absent") : facet.label ?? formatNativeValue(facet.value);
            const button = this.action(() => `${uiTextValue(fieldNames[facet.key] ?? facet.key)}：${uiTextValue(label)} · ${uiTextValue(facet.count)}`, () => {
                try {
                    const expr = facetPredicate(facet, this.schema!);
                    this.savedSelection = null;
                    this.savedPlan = null;
                    this.definitionRevision = null;
                    this.conditions.push({ id: crypto.randomUUID(), label: () => `${uiTextValue(fieldNames[facet.key] ?? facet.key)}：${uiTextValue(label)}`, expr });
                    this.contextChanged();
                }
                catch (error) {
                    this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
                }
            }, "query-facet");
            this.navigationHost.append(button);
        }
    }
    private mountShortcutControls(): void {
        const shortcuts = ui("details", "query-shortcuts"), warning = ui("div"), list = ui("div"), recent = ui("div");
        shortcuts.append(ui("summary", "", message("queryShortcut.disclosure")));
        shortcuts.append(this.action(message("queryShortcut.addCurrent"), () => {
            const label = window.prompt(uiTextValue(message("queryShortcut.namePrompt")), this.savedSelection?.label || this.search || uiTextValue(message("queryShortcut.defaultNewName")));
            if (!label)
                return;
            this.shortcuts.push(this.savedSelection ? { id: crypto.randomUUID(), label, kind: "savedQuery", definition: { ...this.savedSelection } } : { id: crypto.randomUUID(), label, kind: "query", plan: this.getPlan() });
            this.saveShortcuts();
            this.renderControls();
        }), warning, list, recent, ui("p", "data-help", message("queryShortcut.localStorageHelp")));
        this.shortcutsHost.append(shortcuts);
        let previous = "", previousError: UiText | undefined;
        this.controlUpdates!.push(reset => {
            const key = queryRefreshKey([this.shortcuts, this.recent]);
            if (!reset && key === previous && this.shortcutError === previousError)
                return;
            previous = key;
            previousError = this.shortcutError;
            warning.replaceChildren();
            list.replaceChildren();
            recent.replaceChildren();
            if (reset)
                shortcuts.open = false;
            if (this.shortcutError) {
                const key = `weftext-query-shortcuts:${this.libraryIdentity()}`, raw = localStorage.getItem(key);
                warning.append(ui("p", "data-warning", this.shortcutError), ui("pre", "", raw ?? ""), this.action(message("queryShortcut.backupResetInvalid"), () => {
                    if (raw !== null)
                        localStorage.setItem(`${key}:preserved:${crypto.randomUUID()}`, raw);
                    localStorage.setItem(key, JSON.stringify({ version: 2, items: [] }));
                    this.loadPreferences();
                    this.renderControls();
                }));
            }
            this.shortcuts.forEach((shortcut, index) => {
                const row = ui("div", "query-shortcut-row"), label = shortcut.kind === "tag" ? () => `${uiTextValue(shortcut.label)} · ${uiTextValue(shortcut.matchMode === "subtree" ? message("tagNavigation.subtreeMode") : message("tagNavigation.exactEntry"))}` : shortcut.label;
                row.append(this.action(label, async () => {
                    if (shortcut.kind === "directory" || shortcut.kind === "tag")
                        await this.openNavigationShortcut(shortcut);
                    else if (shortcut.kind === "savedQuery" && shortcut.definition)
                        await this.runSaved(shortcut.definition);
                    else if (shortcut.kind === "query" && shortcut.plan)
                        this.setPlan(shortcut.plan);
                    else if (shortcut.path)
                        await this.openTarget(shortcut.path, shortcut.entityKey);
                }, "data-button"));
                const up = this.action(message("queryShortcut.moveUp"), () => {
                    if (index === 0)
                        return;
                    [this.shortcuts[index - 1], this.shortcuts[index]] = [this.shortcuts[index], this.shortcuts[index - 1]];
                    this.saveShortcuts();
                    this.renderControls();
                }, "data-button compact");
                up.disabled = index === 0;
                row.append(up, this.action(message("queryShortcut.remove"), () => { this.shortcuts = this.shortcuts.filter(item => item.id !== shortcut.id); this.saveShortcuts(); this.renderControls(); }, "data-button compact"));
                list.append(row);
            });
            for (const item of this.recent)
                recent.append(this.action(displayMessage("queryShortcut.recentEntry", { p0: uiDisplayParameter(item.label), p1: uiDisplayParameter(item.lifecycle === "trash" ? message("queryShortcut.trashSuffix") : "") }), () => this.openTarget(item.path, item.entityKey, item.lifecycle, item.revision), "query-recent"));
        });
    }
    private saveShortcuts(): void {
        if (this.shortcutError) {
            this.host.notify(this.shortcutError, true);
            throw new UiError(this.shortcutError);
        }
        decodeTagShortcutCollection({ version: 2, items: this.shortcuts });
        localStorage.setItem(`weftext-query-shortcuts:${this.libraryIdentity()}`, JSON.stringify({ version: 2, items: this.shortcuts }));
    }
    private renderRegisteredNavigation(): void {
        if (!this.root || !this.workspaceId) {
            if (this.registeredHost.firstChild)
                this.registeredHost.replaceChildren();
            return;
        }
        const root = this.root, workspaceId = this.workspaceId, identity = JSON.stringify([root, workspaceId]), current = () => this.root === root && this.workspaceId === workspaceId;
        let controls = this.navigationControls.get(identity);
        if (!controls) {
            controls = new NavigationControls(this.host, root, workspaceId, () => this.scopeIdentity ?? this.directory ?? root, (targetRef, label, mode) => {
                if (current())
                    this.applyTag(targetRef, label, mode);
            }, (targetRef, label, kind, matchMode) => {
                if (!current())
                    throw new UiError(message("queryShortcut.originalLibraryRequired"));
                if (kind === "tag" && matchMode !== "exact" && matchMode !== "subtree")
                    throw new UiError(message("queryShortcut.tagModeRequired"));
                if (!this.shortcuts.some(item => item.targetRef === targetRef && item.kind === kind && item.matchMode === matchMode))
                    this.shortcuts.push({ id: crypto.randomUUID(), label, kind, targetRef, ...(kind === "tag" ? { matchMode } : {}) });
                this.saveShortcuts();
                this.renderControls();
            }, () => current() ? this.fieldByKey("tags") : undefined, () => {
                if (current())
                    this.tagRegistryChanged();
            });
            this.navigationControls.set(identity, controls);
        }
        if (this.registeredHost.firstChild !== controls.element)
            this.registeredHost.replaceChildren(controls.element);
    }
    private tagRegistryChanged(): void {
        if (this.root)
            this.environmentChanged(this.root);
    }
    private applyTag(targetRef: string, label: string, mode: TagMatchMode): void {
        try {
            const field = this.schema?.fields.find(field => field.key === "tags");
            if (!field || !this.schema)
                throw new UiError(message("queryValidation.tagsFieldUnavailable"));
            const expr = tagPredicate(targetRef, mode, fieldExpr(field.id));
            this.savedSelection = null;
            this.savedPlan = null;
            this.definitionRevision = null;
            if (!this.conditions.some(condition => condition.expr === expr))
                this.conditions.push({ id: crypto.randomUUID(), label: displayMessage("queryFilter.tagChip", { p0: uiDisplayParameter(label), p1: uiDisplayParameter(mode === "subtree" ? message("tagNavigation.subtreeMode") : message("tagNavigation.exactEntry")) }), expr, tag: { reference: targetRef, label, mode } });
            this.contextChanged();
        }
        catch (error) {
            this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
        }
    }
    private async openNavigationShortcut(shortcut: Shortcut): Promise<void> {
        return this.closeGuard.run(async () => {
            try {
                if (!shortcut.targetRef)
                    throw new UiError(message("queryShortcut.stableTargetMissing"));
                const root = this.root, workspace = this.workspaceId, target = await this.host.request<NavigationTarget>("/api/navigation/resolve", { root, targetRef: shortcut.targetRef });
                if (root !== this.root || workspace !== this.workspaceId)
                    return;
                if (target.status !== "managed" || target.targetRef !== shortcut.targetRef)
                    throw new UiError(displayMessage("queryShortcut.targetState", { p0: uiDisplayParameter(target.status === "missing" ? message("queryShortcut.targetMissing") : message("tagNavigation.unavailableIdentity")) }));
                if (shortcut.kind === "tag") {
                    if (!shortcut.matchMode)
                        throw new UiError(message("queryShortcut.tagModeMissing"));
                    this.applyTag(shortcut.targetRef, `${target.displayName ?? target.label} · ${shortcut.targetRef.slice(shortcut.targetRef.lastIndexOf("/") + 1)}`, shortcut.matchMode);
                }
                else if (target.path)
                    this.applyScope(target.path, shortcut.targetRef);
                else
                    throw new UiError(message("queryShortcut.directoryUnavailable"));
            }
            catch (error) {
                this.host.notify(displayMessage("queryShortcut.targetUnavailable", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : message("error.invalidResponse")) }), true);
            }
        });
    }
    private mountSavedControls(): void {
        const section = ui("details", "query-saved-definitions"), unavailable = ui("p", "data-help", message("queryDefinition.openOwnerRequired")), definitionsHost = ui("div", "saved-query-definitions"), version = ui("p", "data-help"), rows = ui("div");
        section.append(ui("summary", "", message("queryDefinition.disclosure")));
        definitionsHost.append(version, rows);
        let ownerContext = "", readGeneration = 0;
        const owner = (): {
            root: string;
            owner: string;
            identity: string;
        } | null => {
            const document = this.host.getDocument();
            if (!document?.nodeId || !this.workspaceId || document.root !== this.root || !this.root)
                return null;
            const owner = `weftext-node://${this.workspaceId}/${document.nodeId}`;
            return { root: this.root, owner, identity: JSON.stringify([this.root, owner]) };
        };
        const loaded = new Map<string, {
            element: HTMLElement;
            definition: SavedQuery;
            mode: HTMLSelectElement;
            revision: HTMLInputElement;
            title: HTMLElement;
            edited: boolean;
        }>();
        const load = this.action(message("queryDefinition.readCurrentNote"), async () => {
            const context = owner();
            if (!context)
                return;
            const generation = ++readGeneration;
            load.disabled = true;
            try {
                const result = await this.host.request<SavedDefinitions>("/api/query/definitions", { root: context.root, owner: context.owner });
                if (owner()?.identity !== context.identity || generation !== readGeneration)
                    return;
                setUiText(version, () => uiTextValue(displayMessage("queryDefinition.ownerRevision", { p0: uiDisplayParameter(result.ownerRevision ?? message("queryDefinition.noRetainedRevision")) })) + uiTextValue(!result.definitions.length ? message("queryDefinition.noSavedDefinitionsSuffix") : "") + (result.unavailable?.length ? ` · ${result.unavailable.map(diagnosticText).join("；")}` : ""));
                const wanted = new Set(result.definitions.map(item => item.key));
                for (const definition of result.definitions) {
                    let row = loaded.get(definition.key);
                    if (!row) {
                        const element = ui("section", "saved-query-row"), title = ui("h3"), mode = selectControl([["current", message("queryPanel.currentDefinition")], ["pinned", message("queryDefinition.pinExactRevision")]], "current"), revision = ui("input", "data-input");
                        revision.value = result.ownerRevision ?? "";
                        row = { element, definition, title, mode, revision, edited: false };
                        const currentRow = row;
                        mode.onchange = () => { currentRow.edited = true; this.draftFlags.add(`saved:${context.owner}:${definition.key}`); };
                        revision.oninput = () => { currentRow.edited = true; this.draftFlags.add(`saved:${context.owner}:${definition.key}`); };
                        element.append(title, labeled(message("queryDefinition.readModeLabel"), mode), labeled(message("queryDefinition.pinnedRevisionLabel"), revision), this.action(message("queryDefinition.openAndQuery"), () => {
                            if (owner()?.identity !== context.identity)
                                throw new UiError(message("queryDefinition.ownerChangedRetained"));
                            return this.runSaved({ owner: context.owner, key: currentRow.definition.key, label: currentRow.definition.label.replace(/^weftext-query-/, ""), mode: mode.value as "current" | "pinned", ...(mode.value === "pinned" ? { revision: revision.value } : {}) });
                        }, "data-button primary"), this.action(message("queryDefinition.updateAndSaveNote"), () => {
                            if (owner()?.identity !== context.identity)
                                throw new UiError(message("queryDefinition.ownerChangedBeforeSave"));
                            return this.saveDefinition(currentRow.definition.label, currentRow.definition.key);
                        }));
                        loaded.set(definition.key, row);
                        rows.append(element);
                    }
                    row.definition = definition;
                    setUiText(row.title, definition.label.replace(/^weftext-query-/, ""));
                    setUiAttr(row.revision, "aria-label", displayMessage("queryDefinition.namedRevisionLabel", { p0: row.definition.label.replace(/^weftext-query-/, "") }));
                }
                for (const [key, row] of loaded)
                    if (!wanted.has(key)) {
                        if (row.edited || row.element.contains(document.activeElement))
                            setUiText(row.title, displayMessage("queryDefinition.unavailableRetainedName", { p0: uiDisplayParameter(row.definition.label.replace(/^weftext-query-/, "")) }));
                        else {
                            row.element.remove();
                            loaded.delete(key);
                        }
                    }
            }
            catch (error) {
                if (owner()?.identity === context.identity && generation === readGeneration)
                    setUiText(version, error instanceof Error ? errorUiText(error) : message("error.invalidResponse"));
            }
            finally {
                if (owner()?.identity === context.identity && generation === readGeneration)
                    load.disabled = false;
            }
        });
        const label = ui("input", "data-input");
        setUiAttr(label, "placeholder", message("queryDefinition.namePlaceholder"));
        setUiAttr(label, "aria-label", message("queryDefinition.nameLabel"));
        const updateLabel = this.draftInput(label, () => this.definitionName, value => { this.definitionName = value; });
        const save = this.action(message("queryDefinition.saveQueryAndNote"), () => this.saveDefinition(label.value));
        section.append(unavailable, load, definitionsHost, label, save, this.action(message("common.cancel"), () => { this.definitionName = ""; label.value = ""; }), ui("p", "data-help", message("queryDefinition.saveScopeHelp")));
        this.savedHost.append(section);
        this.controlUpdates!.push(reset => {
            const context = owner(), next = context?.identity ?? "";
            if (next !== ownerContext) {
                ownerContext = next;
                readGeneration++;
                loaded.clear();
                rows.replaceChildren();
                setUiText(version, "");
                load.disabled = false;
            }
            if (reset) {
                section.open = false;
                for (const row of loaded.values()) { row.mode.value = "current"; row.revision.value = ""; row.edited = false; }
            }
            unavailable.hidden = !!context;
            load.hidden = definitionsHost.hidden = label.hidden = save.hidden = !context;
            setUiText(save, this.savedPending ? message("queryDefinition.confirmPreviousSave") : message("queryDefinition.saveQueryAndNote"));
            updateLabel(reset);
        });
    }
    private renderPeriodNavigation(): void {
        if (!this.root || !this.workspaceId) {
            if (this.periodHost.firstChild)
                this.periodHost.replaceChildren();
            return;
        }
        const root = this.root, workspaceId = this.workspaceId, identity = JSON.stringify([root, workspaceId]);
        let navigator = this.periodNavigators.get(identity);
        const available = !!this.schema?.fields.some(item => item.key === "period");
        if (!navigator && available) {
            navigator = new PeriodNavigator(this.host, root, () => this.root === root && this.workspaceId === workspaceId && !this.directory?.startsWith("weftext-") ? this.directory ?? root : root);
            this.periodNavigators.set(identity, navigator);
        }
        if (navigator) {
            if (this.periodHost.firstChild !== navigator.element)
                this.periodHost.replaceChildren(navigator.element);
            navigator.setSchemaAvailable(available);
        }
        else if (this.periodHost.firstChild)
            this.periodHost.replaceChildren();
    }
    private async saveDefinition(label: string, key?: string): Promise<void> {
        return this.closeGuard.run(async () => {
            if (!this.host.savedDefinition) {
                this.host.notify(message("queryDefinition.saveReceiptUnavailable"), true);
                return;
            }
            const initiator = this.host.getDocument();
            try {
                await this.host.prepareAction?.();
            }
            catch (error) {
                this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
                return;
            }
            const basis = this.host.getDocument();
            if (!basis?.nodeId || basis.root !== this.root || (!label.trim() && !this.savedPending)) {
                this.host.notify(message("queryDefinition.noteAndNameRequired"), true);
                return;
            }
            if (!this.savedPending && (!initiator || initiator.sessionId !== basis.sessionId)) {
                this.host.notify(message("shell.queryProposalBasisChanged"), true);
                return;
            }
            const nativeLabel = label.startsWith("weftext-query-") ? label : `weftext-query-${label}`;
            if (!this.savedPending && !/^weftext-query-[\p{L}\p{N}_-]+$/u.test(nativeLabel)) {
                this.host.notify(message("queryDefinition.invalidName"), true);
                return;
            }
            const retry = this.savedPending !== null;
            this.savedPending ??= { basis: { ...basis }, body: { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, baseId: basis.baseId, inputRevision: basis.inputRevision, key, label: nativeLabel, plan: this.getPlan(), operationId: crypto.randomUUID() } };
            const pending = this.savedPending;
            let receiptReceived = false;
            try {
                const result = await submitOriginal<OriginalFact & {
                    document?: unknown;
                    definition?: SavedQuery;
                    definitionRevision?: string;
                }>(this.host.request, "/api/query/definition/save", pending.body, "save", retry);
                receiptReceived = true;
                await this.host.savedDefinition(result, pending.basis);
                this.savedPending = null;
                if (this.definitionName === label) this.definitionName = "";
                this.host.notify(message("queryDefinition.saved"));
                this.renderControls();
                await this.refresh();
            }
            catch (error) {
                const code = error && typeof error === "object" && "code" in error ? String(error.code) : "unknown";
                if (code === "committedResultUnavailable")
                    this.environmentChanged(pending.basis.root);
                if (!receiptReceived && error instanceof TerminatedAttempt)
                    this.savedPending = null;
                this.host.notify(() => `${uiTextValue(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"))}${uiTextValue(this.savedPending ? message("queryDefinition.originalSaveRetainedSuffix") : message("queryDefinition.originalDraftRetainedSuffix"))}`, true);
                this.renderControls();
            }
        });
    }
    private async runSaved(selection: SavedSelection): Promise<void> {
        return this.closeGuard.run(async () => {
            if (!this.root || selection.mode === "pinned" && !selection.revision) {
                this.host.notify(message("queryDefinition.historicalRevisionRequired"), true);
                return;
            }
            const prior = this.preferenceKey();
            this.savedSelection = structuredClone(selection);
            this.draftFlags.delete(`saved:${selection.owner}:${selection.key}`);
            if (this.preferenceKey() !== prior)
                this.loadPreferences();
            this.renderControls();
            await this.refresh();
        });
    }
    private renderResults(): void {
        if (this.closeGuard.frozen) return;
        this.thumbnailDisposers.forEach(dispose => dispose());
        this.thumbnailDisposers = [];
        const scrollTop = this.resultsHost.scrollTop;
        this.resultsHost.replaceChildren();
        this.resultsHost.className = `query-results ${this.preference.density}`;
        this.resultsHost.setAttribute("aria-busy", String(this.loading));
        if (!this.root) {
            this.resultsHost.append(ui("p", "data-empty", message("queryPanel.openLibraryHelp")));
            return;
        }
        if (!this.workspaceId) {
            this.resultsHost.append(ui("p", "data-help", message("queryPanel.ordinaryFolderHelp")));
            return;
        }
        if (this.loading)
            this.resultsHost.append(ui("p", "query-result-status", message("queryStatus.readingScope")));
        if (this.error) {
            const error = ui("div", "query-error");
            error.setAttribute("role", "alert");
            error.append(ui("p", "data-warning", displayMessage("queryStatus.failed", { p0: uiDisplayParameter(this.error) })), this.action(message("queryPanel.retryQuery"), () => this.refresh()));
            this.resultsHost.append(error);
        }
        if (!this.result)
            return;
        const displayResult = this.result;
        const status = ui("p", `query-result-status ${displayResult.coverage} ${displayResult.freshness}`, queryResultState(displayResult));
        status.setAttribute("role", "status");
        this.resultsHost.append(status);
        if (this.exportResult) {
            const result = displayResult;
            const output = this.action(message("queryPanel.outputResults"), async () => {
                if (!result.queryId) {
                    this.host.notify(message("queryValidation.frozenOutputIdMissing"), true);
                    return;
                }
                await this.exportResult!({ queryId: result.queryId, label: result.snapshotLabel ?? uiTextValue(message("queryPanel.outputFallbackLabel")), rowCount: result.rows.length, coverage: result.coverage, freshness: result.freshness, limited: result.limited });
            }, "data-button compact");
            output.disabled = this.loading || !result.queryId || !!result.superseded;
            this.resultsHost.append(output);
        }
        if (displayResult.cutId)
            this.resultsHost.append(ui("p", "data-help query-basis", displayMessage("queryStatus.readRevision", { p0: uiDisplayParameter(displayResult.cutId) })));
        if (displayResult.snapshotMode === "perFileObserved")
            this.resultsHost.append(ui("p", "data-help query-basis", message("queryStatus.perFileSnapshotHelp")));
        if (displayResult.snapshotMode === "admittedFrozen")
            this.resultsHost.append(ui("p", "data-help query-basis", displayMessage("queryStatus.exactSnapshotHelp", { p0: uiDisplayParameter(displayResult.snapshotLabel ?? message("queryStatus.frozenSnapshotFallback")) })));
        const pendingCount = displayResult.rawScope?.pendingCount;
        if (pendingCount)
            this.resultsHost.append(ui("p", "data-help", displayMessage("queryStatus.pendingAdmissionCount", { p0: pendingCount })));
        if (displayResult.aggregateExact === false)
            this.resultsHost.append(ui("p", "data-warning", message("queryAggregate.knownInputOnly")));
        const resultDiagnostics = queryDiagnostics(displayResult);
        if (resultDiagnostics.length) {
            const diagnostics = ui("details", "query-diagnostics");
            diagnostics.append(ui("summary", "", displayMessage("queryStatus.diagnosticCount", { p0: resultDiagnostics.length })));
            resultDiagnostics.forEach(item => diagnostics.append(ui("p", "data-warning", item)));
            this.resultsHost.append(diagnostics);
        }
        if (!displayResult.rows.length)
            this.resultsHost.append(ui("p", "data-empty", displayResult.coverage === "complete" && displayResult.snapshotMode === "admittedFrozen" ? message("queryStatus.frozenNoMatchingNotes") : displayResult.coverage === "complete" && displayResult.freshness === "current" ? message("queryStatus.currentNoMatchingNotes") : message("queryStatus.partialNoMatchingRows")));
        const rows = pinEligibleRows(displayResult.rows, this.preference.pinned);
        if (this.preference.view === "list")
            this.renderList(rows, this.resultsHost);
        else if (this.preference.view === "table")
            this.renderTable(rows, this.resultsHost);
        else if (this.preference.view === "board")
            this.renderBoard(rows);
        else if (this.preference.view === "calendar")
            this.renderCalendar(rows);
        else
            this.renderChart(rows);
        if (!["list", "table"].includes(this.preference.view)) {
            const alternative = ui("details", "query-accessible-table");
            alternative.append(ui("summary", "", message("queryView.equivalentTable")));
            this.renderTable(rows, alternative);
            this.resultsHost.append(alternative);
        }
        this.resultsHost.scrollTop = scrollTop;
    }
    private rowOpen(row: QueryRow): HTMLButtonElement {
        const button = this.action(rowTitle(row), async () => {
            if (!row.path) {
                this.host.notify(message("queryValidation.generatedRowReadOnly"), true);
                return;
            }
            try {
                const lifecycle = queryRowLifecycle(row, this.resultPlan);
                if (!lifecycle)
                    throw new UiError(message("queryValidation.rowLifecycleMissing"));
                let path = row.path;
                if (row.entityKey) {
                    const opened = await this.openStrong(row.entityKey, lifecycle, row.revision ?? undefined);
                    path = opened.path;
                }
                else {
                    if (lifecycle === "trash")
                        throw new UiError(message("queryValidation.trashRowIdentityMissing"));
                    await this.host.openDocument(path, this.root ?? undefined);
                }
                this.selected = row.entityKey ?? row.key;
                this.recent = [{ entityKey: this.selected, path, label: uiTextValue(rowTitle(row)), lifecycle, ...(lifecycle === "trash" && typeof row.revision === "string" ? { revision: row.revision } : {}) }, ...this.recent.filter(item => item.entityKey !== this.selected || item.lifecycle !== lifecycle)].slice(0, 12);
                localStorage.setItem(`weftext-query-recents:${this.libraryIdentity()}`, JSON.stringify(this.recent));
                this.renderControls();
                this.renderResults();
            }
            catch (error) {
                this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
            }
        }, "query-row-title");
        button.disabled = !row.path;
        if (row.cells.title?.state !== "value")
            setUiAttr(button, "title", message("queryView.filenameFallbackTitle"));
        if ((row.entityKey ?? row.key) === this.selected)
            button.setAttribute("aria-current", "true");
        return button;
    }
    private rowControls(row: QueryRow): HTMLElement {
        const controls = ui("div", "query-row-actions");
        const key = row.entityKey ?? row.key;
        const pinned = this.preference.pinned.includes(key);
        const pin = this.action(pinned ? message("queryView.unpin") : message("queryView.pin"), () => { this.preference.pinned = pinned ? this.preference.pinned.filter(item => item !== key) : [...this.preference.pinned, key]; this.savePreferences(false); this.renderResults(); }, "data-button compact");
        pin.setAttribute("aria-pressed", String(pinned));
        controls.append(pin);
        if (row.entityKey && row.path && queryRowLifecycle(row, this.resultPlan) === "live") {
            const target = {entityKey:row.entityKey,path:row.path};
            controls.append(this.action(message("queryShortcut.addRow"), () => {
                if (!this.shortcuts.some(item => item.entityKey === target.entityKey))
                    this.shortcuts.push({ id: crypto.randomUUID(), label: uiTextValue(rowTitle(row)), kind: "node", ...target });
                this.saveShortcuts();
                this.renderControls();
            }, "data-button compact"));
        }
        const status = row.cells.status;
        if (!row.readOnly && row.entityKey && row.path && queryRowLifecycle(row, this.resultPlan) === "live" && this.schema && !this.result?.superseded && !this.result?.externalPending && (this.result?.freshness === "current" || this.result?.snapshotMode === "admittedFrozen") && this.host.canEdit?.() !== false) {
            const edit = ui("details", "query-inline-fields");
            edit.append(ui("summary", "", message("queryEdit.inlineDisclosure")), ui("p", "data-help", message("queryEdit.inlineHelp")));
            for (const [column, cell] of Object.entries(row.cells)) {
                const field = directQueryField(this.resultPlan, this.schema, column);
                if (!field || cell.state !== "value" || !cell.value)
                    continue;
                const section = ui("section"), shape = resolveShape(field.shape, this.schema);
                if (!["scalar", "enum", "union"].includes(shape.kind) && field.key !== "tags")
                    continue;
                const key = JSON.stringify([this.root, row.entityKey, field.id]), binding = JSON.stringify([field.shape, this.schema.definitions]), draft = this.inlineDrafts.get(key) ?? { value: structuredClone(cell.value), raw: {}, binding, revision: 0 };
                section.append(ui("strong", "", fieldNames[field.key] ?? field.key));
                if (draft.binding !== binding) {
                    section.append(ui("p", "data-warning", message("queryEdit.fieldShapeChanged")), ui("pre", "", JSON.stringify(draft, null, 2)), this.action(message("queryEdit.discardOldFieldInput"), () => { this.inlineDrafts.delete(key); this.renderResults(); }));
                    edit.append(section);
                    continue;
                }
                section.append(valueEditor(field.shape, draft.value, this.schema, next => this.retainInlineDraft(key, draft, next), displayMessage("queryEdit.resultFieldLabel", { p0: uiDisplayParameter(fieldNames[field.key] ?? field.key) }), { fieldId: field.id, path: [] }, 0, this.tagReferences));
                if (field.key === "tags" && this.root)
                    section.append(this.action(message("queryEdit.readTagCatalog"), async () => { const root = this.root!, generation = this.generation; await loadTagFormReferences(this.host, root, this.tagReferences, this.element, () => root === this.root && generation === this.generation); }));
                for (const [id, control] of nativeFormControls(section))
                    if (id in draft.raw) {
                        control.value = draft.raw[id];
                        control.dispatchEvent(new Event("input", { bubbles: true }));
                    }
                const retain = () => this.retainInlineDraft(key, draft, undefined, Object.fromEntries(nativeFormControls(section).filter(([, control]) => !control.validity.valid).map(([id, control]) => [id, control.value])));
                section.addEventListener("input", retain);
                section.addEventListener("change", retain);
                section.append(this.action(message("queryEdit.applyField"), async () => {
                    const invalid = section.querySelector<HTMLInputElement>(":invalid");
                    if (invalid) {
                        invalid.reportValidity();
                        return;
                    }
                    await this.applyInlineDraft(row, field, key, draft);
                }), this.action(message("queryEdit.discardOldFieldInput"), () => { this.inlineDrafts.delete(key); this.renderResults(); }));
                edit.append(section);
            }
            if (edit.querySelector("section"))
                controls.append(edit);
        }
        if (!row.readOnly && queryRowLifecycle(row, this.resultPlan) === "live" && row.path && hasDirectStatusProjection(this.resultPlan, this.schema) && status?.state === "value" && status.value?.kind === "text" && ["open", "done", "cancelled"].includes(status.value.value)) {
            const next = status.value.value === "done" ? "open" : "done";
            controls.append(this.action(displayMessage("queryEdit.statusAction", { p0: uiDisplayParameter(next === "done" ? message("queryEdit.doneStatus") : message("queryEdit.openStatus")) }), () => this.changeTaskField(row, "status", { kind: "text", value: next }), "data-button compact"));
        }
        return controls;
    }
    private renderList(rows: QueryRow[], container: HTMLElement): void {
        const list = ui("div", "query-list");
        for (const row of rows) {
            const card = ui("article", `query-card${(row.entityKey ?? row.key) === this.selected ? " selected" : ""}`);
            card.append(this.rowOpen(row));
            if (row.cells.title?.state !== "value")
                card.append(ui("span", "query-fallback", message("queryView.filenameFallback")));
            if (this.preference.summaries) {
                const summary = row.cells.summary;
                card.append(ui("p", "query-excerpt", summary?.state === "value" ? uiTextValue(formatNativeValue(summary.value)).slice(0, 180) || message("queryView.emptyBody") : displayMessage("queryView.bodySummaryState", { p0: uiDisplayParameter(summary ? cellLabels[summary.state] : message("queryValue.unprojected")) })));
            }
            const metadata = ui("dl", "query-card-fields");
            for (const [name, cell] of Object.entries(row.cells).filter(([name]) => !["title", "summary", "types"].includes(name))) {
                metadata.append(ui("dt", "", fieldNames[name] ?? name), ui("dd", `cell-${cell.state}`, stateText(cell)));
            }
            card.append(metadata);
            if (this.preference.thumbnails) {
                const thumbnail = ui("div", "query-thumbnail-host");
                card.append(thumbnail);
                this.renderThumbnail(thumbnail, row);
            }
            card.append(this.rowControls(row));
            list.append(card);
        }
        container.append(list);
    }
    private renderThumbnail(container: HTMLElement, row: QueryRow): void {
        if (queryRowLifecycle(row, this.resultPlan) !== "live") {
            container.append(ui("p", "query-preview-state", message("queryView.thumbnailLifecycleUnavailable")));
            return;
        }
        if (!row.entityKey || !row.revision || !this.root || !this.host.requestBlob) {
            container.append(ui("p", "query-preview-state", message("queryView.thumbnailBasisUnavailable")));
            return;
        }
        const root = this.root, key = `${root}:${row.entityKey}:${row.revision}`;
        const thumbnail = mountListThumbnail(container, { viewport: this.resultsHost,
            resolve: () => {
                let pending = this.previewCache.get(key);
                if (!pending) {
                    pending = this.host.request<RowPreview>("/api/query/preview", { root, entityKey: row.entityKey, revision: row.revision });
                    this.previewCache.set(key, pending);
                }
                return pending;
            },
            read: (descriptor, revision) => this.host.requestBlob!("/api/query/preview-resource", { root, entityKey: row.entityKey, revision, resource: descriptor.resource, resourceRevision: descriptor.revision }),
        });
        this.thumbnailDisposers.push(() => thumbnail.dispose());
    }
    private renderTable(rows: QueryRow[], container: HTMLElement): void {
        const columns = [...new Set([...(this.result?.columns ?? []), ...rows.flatMap(row => Object.keys(row.cells))])].filter(column => column !== "summary");
        const table = ui("table", "query-table");
        const caption = ui("caption", "", displayMessage("queryView.tableCaption", { p0: uiDisplayParameter(this.result?.coverage === "partial" ? message("queryView.partialCaptionScope") : message("queryView.currentCaptionScope")), p1: rows.length }));
        table.append(caption);
        const head = ui("thead");
        const heading = ui("tr");
        heading.append(ui("th", "", message("queryColumn.noteOrRow")));
        columns.filter(column => column !== "title").forEach(column => { const cell = ui("th", "", fieldNames[column] ?? column); cell.scope = "col"; heading.append(cell); });
        heading.append(ui("th", "", message("queryColumn.actions")));
        head.append(heading);
        table.append(head);
        const body = ui("tbody");
        for (const row of rows) {
            const line = ui("tr");
            const title = ui("th");
            title.scope = "row";
            title.append(this.rowOpen(row));
            line.append(title);
            for (const column of columns.filter(column => column !== "title"))
                line.append(ui("td", `cell-${row.cells[column]?.state ?? "absent"}`, stateText(row.cells[column])));
            const controls = ui("td");
            controls.append(this.rowControls(row));
            line.append(controls);
            body.append(line);
        }
        table.append(body);
        const scroll = ui("div", "query-table-scroll");
        scroll.append(table);
        container.append(scroll);
    }
    private renderBoard(rows: QueryRow[]): void {
        const column = this.preference.groupField;
        const groups = new Map<string, {
            label: UiText;
            rows: QueryRow[];
        }>();
        for (const row of rows) {
            const cell = row.cells[column];
            const key = cell?.state === "value" && cell.value ? canonicalNative(cell.value) : `@${cell?.state ?? "unprojected"}`;
            const group = groups.get(key) ?? { label: stateText(cell), rows: [] };
            group.rows.push(row);
            groups.set(key, group);
        }
        this.resultsHost.append(ui("p", "data-help", displayMessage("queryView.boardHelp", { p0: uiDisplayParameter(fieldNames[column] ?? column) })));
        const board = ui("div", "query-board");
        for (const group of groups.values()) {
            const lane = ui("section", "query-board-lane");
            lane.append(ui("h3", "", () => `${uiTextValue(group.label)} · ${uiTextValue(group.rows.length)}`));
            this.renderList(group.rows, lane);
            board.append(lane);
        }
        this.resultsHost.append(board);
    }
    private renderCalendar(rows: QueryRow[]): void {
        const column = this.preference.dateField;
        const days = new Map<string, QueryRow[]>();
        const undated: QueryRow[] = [];
        for (const row of rows) {
            const cell = row.cells[column];
            const day = cell?.state === "value" ? calendarDay(cell.value) : null;
            if (!day)
                undated.push(row);
            else
                days.set(day, [...(days.get(day) ?? []), row]);
        }
        this.resultsHost.append(ui("p", "data-help", displayMessage("queryView.calendarHelp", { p0: uiDisplayParameter(fieldNames[column] ?? column) })));
        const calendar = ui("div", "query-calendar");
        for (const [day, items] of [...days].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) {
            const cell = ui("section", "query-calendar-day");
            const time = ui("time", "", day);
            time.dateTime = day;
            const heading = ui("h3");
            heading.append(time);
            cell.append(heading);
            for (const row of items)
                cell.append(this.rowOpen(row), this.rowControls(row));
            calendar.append(cell);
        }
        this.resultsHost.append(calendar);
        if (undated.length) {
            const other = ui("details", "query-undated");
            other.open = true;
            other.append(ui("summary", "", displayMessage("queryView.undatedCount", { p0: undated.length })));
            this.renderTable(undated, other);
            this.resultsHost.append(other);
        }
    }
    private renderChart(rows: QueryRow[]): void {
        const column = this.preference.chartField;
        const samples = rows.flatMap(row => {
            const cell = row.cells[column];
            if (cell?.state !== "value" || !cell.value || !["int", "decimal", "float"].includes(cell.value.kind))
                return [];
            const value = cell.value as Extract<NativeValue, {
                value: string;
            }>;
            const amount = Number(value.value);
            return Number.isFinite(amount) ? [{ row, value, amount }] : [];
        });
        this.resultsHost.append(ui("p", "data-help", displayMessage("queryView.chartHelp", { p0: uiDisplayParameter(fieldNames[column] ?? column) })));
        if (!samples.length) {
            this.resultsHost.append(ui("p", "data-empty", message("queryView.noPlottableValues")));
            return;
        }
        const max = Math.max(...samples.map(item => Math.abs(item.amount)), 1);
        const chart = ui("div", "query-chart");
        for (const sample of samples) {
            const row = ui("div", "query-chart-row");
            const label = sample.row.path ? this.rowOpen(sample.row) : ui("span", "", joinUiText(Object.entries(sample.row.cells).filter(([name]) => name !== column).map(([, cell]) => stateText(cell)), " · "));
            const track = ui("div", "query-chart-track");
            const bar = ui("div", `query-chart-bar${sample.amount < 0 ? " negative" : ""}`);
            bar.style.width = `${Math.max(0.5, Math.abs(sample.amount) / max * 100)}%`;
            track.setAttribute("aria-hidden", "true");
            track.append(bar);
            row.append(label, track, ui("span", "query-chart-value", formatNativeValue(sample.value)));
            chart.append(row);
        }
        this.resultsHost.append(chart);
        if (samples.length !== rows.length)
            this.resultsHost.append(ui("p", "data-warning", displayMessage("queryView.nonNumericRows", { p0: rows.length - samples.length })));
    }
    private async changeTaskField(row: QueryRow, key: string, value: NativeValue, expectedFieldId = this.schema?.fields.find(field => field.key === key)?.id, applied?: () => void): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.host.canEdit?.() === false) {
                this.host.notify(message("queryEdit.sessionReadOnly"), true);
                return;
            }
            if (row.readOnly || !row.path || queryRowLifecycle(row, this.resultPlan) !== "live") {
                this.host.notify(message("queryEdit.rowWriteTargetMissing"), true);
                return;
            }
            try {
                await this.host.prepareAction?.();
                const target = row.entityKey ? await this.openStrong(row.entityKey) : { path: row.path, nodeId: undefined };
                if (!row.entityKey)
                    await this.host.openDocument(target.path, this.root ?? undefined);
                const selected = this.host.getDocument();
                if (!selected || (target.nodeId ? selected.nodeId !== target.nodeId : selected.path !== target.path))
                    throw new UiError(message("queryEdit.documentIdentityMismatch"));
                const basis = { ...selected }, generation = this.generation;
                const current = () => generation === this.generation && this.host.canEdit?.() !== false && sameProjectionInput(this.host.getDocument(), basis);
                const projection = await readNoteProjection(this.host.request.bind(this.host), basis);
                if (!current()) throw new UiError(message("queryEdit.documentIdentityMismatch"));
                const projected = projection.fields.find(field => field.key === key);
                if (projection.readOnly || !projected?.writable || projected.cell.state !== "value" || !expectedFieldId || projected.fieldId !== expectedFieldId)
                    throw new UiError(message("queryEdit.fieldIdentityChanged"));
                if (!await this.confirmChange(key, projected.cell.value, value))
                    return;
                if (!current()) throw new UiError(message("queryEdit.documentIdentityMismatch"));
                const result = await this.host.request<SourcePatch | {
                    edit: SourcePatch;
                } | {
                    patches: SourcePatch[];
                    source?: string;
                }>("/api/data/edit", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, key, path: [], value });
                await this.host.applyPatch(decodeSourcePatch(result, basis.source), basis);
                applied?.();
                if (this.result) {
                    this.result.freshness = "stale";
                    this.result.superseded = true;
                }
                this.renderResults();
                this.host.notify(displayMessage("queryEdit.fieldApplied", { p0: uiDisplayParameter(fieldNames[key] ?? key) }));
            }
            catch (error) {
                this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
            }
        });
    }
    private async openTarget(path: string, entityKey?: string, lifecycle: "live" | "trash" = "live", revision?: string): Promise<void> {
        return this.closeGuard.run(async () => {
            try {
                if (entityKey)
                    await this.openStrong(entityKey, lifecycle, revision);
                else {
                    if (lifecycle === "trash")
                        throw new UiError(message("queryValidation.trashTargetIdentityMissing"));
                    await this.host.openDocument(path, this.root ?? undefined);
                }
            }
            catch (error) {
                this.host.notify(displayMessage("queryShortcut.openTargetUnavailable", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : message("error.invalidResponse")) }), true);
            }
        });
    }
    private async openStrong(entityKey: string, lifecycle: "live" | "trash" = "live", revision?: string): Promise<OpenedDocument> {
        return this.closeGuard.run(async () => {
            if (!this.host.openResolved)
                throw new UiError(message("queryValidation.stableOpenUnavailable"));
            if (lifecycle === "trash" && !revision)
                throw new UiError(message("queryValidation.trashSourceBasisMissing"));
            const opened = await this.host.request<OpenedDocument>("/api/data/open", { root: this.root, entityKey, lifecycle, ...(lifecycle === "trash" ? { revision } : {}) });
            await this.host.openResolved(opened);
            return opened;
        });
    }
    private async confirmChange(key: string, before: NativeValue | undefined, after: NativeValue): Promise<boolean> {
        return this.closeGuard.run(async () => {
            return await new Promise(resolve => { const dialog = ui("dialog", "data-confirm-dialog"); const form = ui("form"); form.method = "dialog"; form.append(ui("h2", "", displayMessage("queryEdit.confirmationTitle", { p0: uiDisplayParameter(fieldNames[key] ?? key) })), ui("p", "", () => `${uiTextValue(formatNativeValue(before))} → ${uiTextValue(formatNativeValue(after))}`), ui("p", "data-help", message("queryEdit.confirmationHelp"))); const cancel = ui("button", "data-button", message("common.cancel")); cancel.value = "cancel"; const apply = ui("button", "data-button primary", message("queryEdit.applyDraft")); apply.value = "apply"; form.append(cancel, apply); dialog.append(form); document.body.append(dialog); dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "apply"; dialog.remove(); resolve(accepted); }, { once: true }); dialog.showModal(); });
        });
    }
    private async showCapture(): Promise<void> {
        return this.closeGuard.run(async () => {
            if (!this.root || !this.workspaceId)
                return;
            if (this.captureDialog) {
                this.captureForms.get(this.root)?.updateContext();
                this.captureDialog.showModal();
                return;
            }
            if (this.scopeIdentity) {
                try {
                    await this.resolveScopeLocator(this.generation);
                }
                catch (error) {
                    this.host.notify(error instanceof Error ? errorUiText(error) : message("error.invalidResponse"), true);
                    return;
                }
            }
            const dialog = ui("dialog", "capture-dialog");
            this.closeGuard.bind(dialog);
            this.captureDialog = dialog;
            this.captureDialogs.set(this.root, dialog);
            const captureHost: PanelHost = { ...this.host, ...(this.host.openTemplates ? { openTemplates: async () => { dialog.close(); await this.host.openTemplates!(); } } : {}) };
            const capture = new QuickCapture(captureHost, { root: this.root, parent: this.directory ?? this.root });
            this.captureForms.set(this.root, capture);
            dialog.append(this.action(message("capture.closeRetained"), () => { dialog.close(); }, "data-button capture-close"), capture.element);
            document.body.append(dialog);
            dialog.showModal();
            await capture.initialize();
        });
    }
}
