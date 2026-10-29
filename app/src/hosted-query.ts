import { schemaFieldLabel } from "./builtin-property-labels";
import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, diagnosticText, formatNativeValue, ui, type Cell, type DataSchema } from "./data-panel";
import { queryPlanControls } from "./query-plan-controls";
import { initialOccurrenceView, occurrenceViewControls, renderOccurrenceCards } from "./query-result-view";
import { directQueryField, parseQueryJson, parseQueryParameters, parseQueryPlan, queryResultState, type QueryPlan } from "./query-panel";
import { applyHostedScope, decodeHostedQuery, exactHostedDependencies, exactHostedScope, hostedDependencyMap, hostedScopeDraft, HostedQueryGate, type HostedProviderDescriptor, type HostedQueryDependencies, type HostedQueryResponse, type HostedQueryResult, type HostedQueryRow, type HostedSeriesPolicyDescriptor, type HostedScopeCatalog } from "./hosted-query-state";
import { hostedCachePrefix } from "./hosted-state";
import { HostedQueryDefinitions, type HostedDefinitionHost } from "./hosted-query-definitions";
import { mediaCellReferences } from "./hosted-media-state";
import { TagTree, tagPredicate, type TagMatchMode } from "./tag-tree";
import { tagPicker } from "./tag-picker";
import type { RegistryCatalog } from "./hosted-navigation-registry-state";
type Product = "title" | "body" | "headings" | "types" | "self";
type Resource = {
    resource: string;
    title: string;
    fields: string[];
    products: Record<Product, boolean>;
    relations: string[];
    definitions: boolean;
    definitionHistory: boolean;
};
type Definition = {
    resource?: string;
    key: string;
    label: string;
    revision?: string;
    plan?: QueryPlan;
    binding?: unknown;
};
type Catalog = HostedScopeCatalog & {
    formSchema?: DataSchema | null;
    resources: Resource[];
    definitions: Definition[];
    providers?: HostedProviderDescriptor[];
    seriesPolicies?: HostedSeriesPolicyDescriptor[];
    coverage?: "complete" | "partial";
    providerCoverage?: "complete" | "partial";
    unavailableDefinitions?: {
        resource: string;
        status: "unavailable";
    }[];
    unavailableProviders?: {
        id: string;
        status: "unavailable";
    }[];
    unavailableSeriesPolicies?: {
        id: string;
        status: "unavailable";
    }[];
};
type Draft = HostedQueryDependencies & {
    resources: string[];
    fields: string[];
    products: Product[];
    relations: string[];
    plan: string;
    parameters: string;
    owner: string;
    definition: string;
    selection: "current" | "pinned";
    revision: string;
    scopeDraft?: QueryPlan["scope"];
    visual?: {
        where: string;
        columns: string;
        order: string;
    };
    tagFilter?: {
        reference: string;
        mode: TagMatchMode;
    };
};
type Request = <T>(path: string, body?: unknown) => Promise<T>;
const productNames: Record<Product, UiText> = { title: message("queryColumn.title"), body: message("hostedQuery.productBody"), headings: message("hostedQuery.productHeadings"), types: message("queryPlan.noteTypes"), self: message("hostedQuery.productSelf") };
const basePlan = (): QueryPlan => ({ scope: { descendants: true, lifecycle: "live" }, types: [], select: [], order: [] });
const initial = (): Draft => ({ resources: [], fields: [], products: [], relations: [], providers: {}, seriesPolicies: {}, plan: JSON.stringify(basePlan(), null, 2), parameters: "{}", owner: "", definition: "", selection: "current", revision: "" });
function select(role: string, label: UiText, options: [
    string,
    UiText
][], value: string) {
    const control = ui("select", "data-input");
    control.dataset.controlKey = JSON.stringify(["hosted-query", role]);
    setUiAttr(control, "aria-label", label);
    for (const [key, text] of options) {
        const option = ui("option", "", text);
        option.value = key;
        control.append(option);
    }
    control.value = value;
    return control;
}
function input(role: string, label: UiText, value: string, multiline = false) {
    const control = multiline ? ui("textarea", "data-input") : ui("input", "data-input");
    control.dataset.controlKey = JSON.stringify(["hosted-query", role]);
    setUiAttr(control, "aria-label", label);
    control.value = value;
    if (control instanceof HTMLTextAreaElement) {
        control.rows = 10;
        control.spellcheck = false;
    }
    const host = ui("label", "data-label", label);
    host.append(control);
    return { host, control };
}
const text = (cell?: Cell) => !cell ? message("queryValue.unprojected") : cell.state === "value" ? formatNativeValue(cell.value) : ({ absent: message("queryValue.absent"), null: "none", invalid: message("hostedQuery.invalidCell"), unavailable: message("queryValue.unavailable") } as const)[cell.state];
/** Temporary controls reuse the native Query plan. Only the author buffer saves definitions. */
export class HostedQuery {
    private view = initialOccurrenceView();
    private epoch = 0;
    private key = "";
    private inputRevision = 0;
    private draft = initial();
    private catalog: Catalog = { resources: [], definitions: [] };
    private gate = new HostedQueryGate();
    private busy = false;
    private result?: HostedQueryResult;
    private resultInput?: string;
    private unreadable?: string;
    private message = ui("p", "hint");
    private controls = ui("section");
    private results = ui("section");
    private scopeControls = ui("section");
    private tagTree?: TagTree;
    private tagReadSerial = 0;
    private definitions?: HostedQueryDefinitions;
    constructor(private host: HTMLElement, private request: Request, private open: (resource: string) => Promise<void>, private output: (queryId: string, metadata: {
        columns: string[];
        rowCount: number;
        coverage: string;
    }) => void, author?: HostedDefinitionHost, private dependenciesChanged?: () => void, private openMedia?: (reference: string) => Promise<void>, private fieldActions?: {
        canEdit(resource: string, fieldId: string): boolean;
        open(resource: string, fieldId: string): Promise<void>;
    }) {
        if (author)
            this.definitions = new HostedQueryDefinitions(request, author, () => this.draft.plan, resource => this.catalog.resources.some(item => item.resource === resource && item.definitions));
        this.clear(false);
    }
    dependencies(): HostedQueryDependencies { return exactHostedDependencies(this.draft, this.catalog); }
    get version() { return this.inputRevision; }
    refuseResult(message: UiText) { this.gate.clear(); this.result = undefined; this.resultInput = undefined; setUiText(this.message, message); this.renderResults(); }
    environmentChanged(description: UiText = message("hostedQuery.environmentChanged")) {
        this.tagReadSerial++;
        this.tagTree = undefined;
        this.inputRevision++;
        this.gate.changed();
        if (this.result)
            this.result = { ...this.result, superseded: true };
        setUiText(this.message, description);
        this.renderResults();
    }
    private dependencyBasis() { return JSON.stringify([this.draft.providers, this.draft.seriesPolicies, ...(["providers", "seriesPolicies"] as const).map(kind => Object.keys(this.draft[kind]).map(id => [id, this.catalog[kind]?.find(item => item.id === id)?.fingerprint ?? null]))]); }
    authorChanged(force = false) { void this.definitions?.authorChanged(force); }
    async refreshCatalog(): Promise<void> {
        const epoch = this.epoch;
        try {
            const catalog = await this.request<Catalog>("query/catalog");
            if (epoch !== this.epoch)
                return;
            const previous = this.dependencyBasis(), previousScope = JSON.stringify([this.catalog.directories, this.catalog.trash]);
            this.catalog = catalog;
            if (previous !== this.dependencyBasis() || previousScope !== JSON.stringify([catalog.directories, catalog.trash])) {
                this.changed();
                this.dependenciesChanged?.();
            }
            this.renderControls();
        }
        catch (error) {
            if (epoch === this.epoch)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : message("hostedQuery.catalogRefreshFailed"));
        }
    }
    clear(removeCache = true) {
        this.view = initialOccurrenceView();
        this.tagReadSerial++;
        this.tagTree = undefined;
        this.epoch++;
        this.inputRevision++;
        this.gate.clear();
        this.definitions?.clear();
        this.busy = false;
        this.result = undefined;
        this.resultInput = undefined;
        this.draft = initial();
        this.catalog = { resources: [], definitions: [] };
        this.unreadable = undefined;
        this.host.replaceChildren();
        this.host.hidden = true;
        this.controls.replaceChildren();
        this.results.replaceChildren();
        this.scopeControls.replaceChildren();
        setUiText(this.message, "");
        this.key = "";
        this.dependenciesChanged?.();
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-query:"))
                    sessionStorage.removeItem(key);
        this.key = "";
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch;
        this.key = hostedCachePrefix("query", deployment, actor, generation) + "controls";
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw) {
                const parsed = JSON.parse(raw) as Draft;
                if (!Array.isArray(parsed.resources) || !Array.isArray(parsed.fields) || !Array.isArray(parsed.products) || !Array.isArray(parsed.relations) || typeof parsed.plan !== "string" || typeof parsed.parameters !== "string" || !["current", "pinned"].includes(parsed.selection) || parsed.tagFilter !== undefined && (!parsed.tagFilter || typeof parsed.tagFilter.reference !== "string" || !["exact", "subtree"].includes(parsed.tagFilter.mode)))
                    throw new Error();
                this.draft = { ...parsed, providers: hostedDependencyMap(parsed.providers), seriesPolicies: hostedDependencyMap(parsed.seriesPolicies), scopeDraft: hostedScopeDraft(parsed.scopeDraft) };
            }
        }
        catch {
            this.unreadable = raw ?? "";
        }
        this.host.hidden = false;
        this.host.replaceChildren(ui("h2", "", message("hostedQuery.title")), ui("p", "hint", message("hostedQuery.authorizationHelp")), this.message, this.controls, this.results);
        this.renderControls();
        try {
            const catalog = await this.request<Catalog>("query/catalog");
            if (epoch !== this.epoch)
                return;
            this.catalog = catalog;
            if (!raw) {
                const first = catalog.resources.find(item => item.fields.length) ?? catalog.resources[0];
                this.draft.resources = first ? [first.resource] : [];
                this.draft.fields = first?.fields ?? [];
                const plan = basePlan();
                plan.select = this.draft.fields.map(id => ({ name: id, expr: `field(${JSON.stringify(id)})` }));
                this.draft.plan = JSON.stringify(plan, null, 2);
            }
            if (!this.draft.owner)
                this.draft.owner = catalog.definitions[0]?.resource ?? catalog.resources.find(item => item.definitions)?.resource ?? "";
            this.renderControls();
        }
        catch (error) {
            if (epoch === this.epoch)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : message("hostedQuery.catalogUnavailable"));
        }
        if (epoch === this.epoch)
            this.dependenciesChanged?.();
        if (epoch === this.epoch && this.definitions) {
            this.host.append(this.definitions.element);
            await this.definitions.connect(deployment, actor, generation);
        }
    }
    private fingerprint() { return JSON.stringify(this.draft); }
    private persist() {
        if (!this.key || this.unreadable !== undefined)
            return;
        try {
            sessionStorage.setItem(this.key, JSON.stringify(this.draft));
        }
        catch {
            setUiText(this.message, message("hostedQuery.draftPersistenceFailed"));
        }
    }
    private changed() {
        this.inputRevision++;
        this.gate.changed();
        if (this.result)
            this.result = { ...this.result, superseded: true };
        setUiText(this.message, this.busy ? message("hostedQuery.changedWhileBusy") : message("hostedQuery.changedNotRun"));
        this.persist();
        this.renderResults();
    }
    private writePlan(plan: QueryPlan) { this.draft.plan = JSON.stringify(plan, null, 2); this.draft.visual = undefined; this.changed(); this.renderControls(); }
    private checkboxes<T extends string>(kind: "resources" | "fields" | "products" | "relations", legend: UiText, values: T[], chosen: T[], update: (items: T[]) => void) {
        const group = ui("fieldset");
        group.append(ui("legend", "", legend));
        for (const value of values) {
            const label = ui("label", "data-label", kind === "products" ? productNames[value as Product] : value);
            const control = ui("input");
            control.dataset.controlKey = JSON.stringify(["hosted-query", kind, value]);
            control.type = "checkbox";
            control.checked = chosen.includes(value);
            setUiAttr(control, "aria-label", () => `${uiTextValue(legend)} ${uiTextValue(value)}`);
            control.onchange = () => {
                const next = new Set(chosen);
                if (control.checked)
                    next.add(value);
                else
                    next.delete(value);
                update([...next]);
            };
            label.prepend(control);
            group.append(label);
        }
        for (const value of chosen.filter(value => !values.includes(value)))
            group.append(ui("p", "hint", displayMessage("hostedQuery.originalChoiceUnavailable", { p0: uiDisplayParameter(value) })));
        return group;
    }
    private dependencyControls(kind: "providers" | "seriesPolicies") {
        const title = kind === "providers" ? message("hostedQuery.providersTitle") : message("hostedQuery.seriesPoliciesTitle"), group = ui("fieldset");
        group.append(ui("legend", "", title));
        const descriptors = this.catalog[kind] ?? [], selection = this.draft[kind];
        const update = (id: string, fingerprint?: string) => { this.draft[kind] = Object.fromEntries([...Object.entries(selection).filter(([key]) => key !== id), ...(fingerprint ? [[id, fingerprint]] : [])]); this.changed(); this.dependenciesChanged?.(); this.renderControls(); };
        for (const item of descriptors) {
            const label = ui("label", "data-label", item.id), control = ui("input");
            control.dataset.controlKey = JSON.stringify(["hosted-query", kind, item.id, item.fingerprint]);
            control.type = "checkbox";
            control.checked = selection[item.id] === item.fingerprint;
            setUiAttr(control, "aria-label", () => `${uiTextValue(title)} ${uiTextValue(item.id)}`);
            control.onchange = () => update(item.id, control.checked ? item.fingerprint : undefined);
            label.prepend(control);
            group.append(label);
            const detail = kind === "providers" ? (() => { const provider = item as HostedProviderDescriptor; return `${provider.implementation.id} · ABI ${provider.implementation.abi} · ${provider.data.kind}${provider.data.key ? ` ${provider.data.key}` : ""}${provider.data.version ? ` / ${provider.data.version}` : ""} · ${(provider.queryNames ?? []).join("、")}`; })() : (() => { const policy = item as HostedSeriesPolicyDescriptor; return displayMessage("hostedQuery.seriesPolicySummary", { p0: uiDisplayParameter(policy.key), p1: uiDisplayParameter(policy.kinds.join("、")), p2: uiDisplayParameter(policy.rule), p3: uiDisplayParameter(policy.calendar ?? message("hostedQuery.dependencyUnavailable")), p4: uiDisplayParameter(policy.zone ?? message("hostedQuery.dependencyUnavailable")), p5: uiDisplayParameter(policy.status === "unavailable" ? message("hostedQuery.policyUnavailableSuffix") : "") }); })();
            group.append(ui("p", "hint", detail));
        }
        for (const [id, fingerprint] of Object.entries(selection)) {
            const item = descriptors.find(item => item.id === id);
            if (!item || item.fingerprint !== fingerprint)
                group.append(ui("p", "hint", displayMessage("hostedQuery.originalDependencyChanged", { p0: uiDisplayParameter(title), p1: uiDisplayParameter(id), p2: uiDisplayParameter(fingerprint) })), action(displayMessage("hostedQuery.removeOriginalDependency", { p0: uiDisplayParameter(title), p1: uiDisplayParameter(id) }), () => update(id)));
        }
        if (!descriptors.length)
            group.append(ui("p", "hint", displayMessage("hostedQuery.noDependencies", { p0: uiDisplayParameter(title) })));
        return group;
    }
    private renderScopes() {
        this.scopeControls.replaceChildren();
        let parsed: QueryPlan | undefined;
        try {
            parsed = parseQueryPlan(this.draft.plan);
        }
        catch { /* Preserve incomplete JSON verbatim. */ }
        const current = this.draft.scopeDraft ?? parsed?.scope ?? basePlan().scope, directories = this.catalog.directories ?? [], group = ui("fieldset");
        group.append(ui("legend", "", message("hostedQuery.scopeLegend")));
        if (this.catalog.scopeCoverage === "partial")
            group.append(ui("p", "hint", message("hostedQuery.partialScopeCatalog")));
        const options: [
            string,
            UiText
        ][] = [["", message("hostedQuery.allAuthorizedObjects")], ...directories.map(item => [item.reference, () => `${uiTextValue(item.title || item.id)} · ${uiTextValue(item.kind === "node" ? message("hostedQuery.noteDirectoryKind") : message("hostedQuery.managedDirectoryKind"))} · ${uiTextValue(item.lifecycle ?? "live")}${uiTextValue(item.status === "unavailable" ? message("hostedQuery.unavailableSuffix") : "")}`] as [
                string,
                UiText
            ])];
        if (current.directory && !options.some(([value]) => value === current.directory))
            options.push([current.directory, displayMessage("hostedQuery.retainedScopeChoice", { p0: uiDisplayParameter(current.directory) })]);
        const directory = select("scope-directory", message("hostedQuery.directoryScopeLabel"), options, current.directory ?? ""), descendants = ui("input"), lifecycle = select("scope-lifecycle", message("hostedQuery.lifecycleLabel"), [["live", message("queryPanel.liveNotes")], ["trash", message("queryPanel.readOnlyTrash")]], current.lifecycle);
        descendants.dataset.controlKey = JSON.stringify(["hosted-query", "scope-descendants"]);
        descendants.type = "checkbox";
        descendants.checked = current.descendants;
        setUiAttr(descendants, "aria-label", message("hostedQuery.descendantsLabel"));
        for (const option of directory.options) {
            const item = directories.find(item => item.reference === option.value);
            if (item)
                option.disabled = item.status !== "available" || item.lifecycle === "trash" && this.catalog.trash !== true;
        }
        lifecycle.options[1].disabled = this.catalog.trash !== true;
        const update = () => { this.draft.scopeDraft = { ...(directory.value ? { directory: directory.value } : {}), descendants: descendants.checked, lifecycle: lifecycle.value as "live" | "trash" }; this.changed(); this.renderScopes(); };
        directory.onchange = update;
        descendants.onchange = update;
        lifecycle.onchange = update;
        const label = ui("label", "data-label", message("queryPanel.includeDescendants"));
        label.prepend(descendants);
        group.append(directory, label, lifecycle);
        try {
            exactHostedScope(current, this.catalog);
        }
        catch (error) {
            group.append(ui("p", "hint", error instanceof Error ? errorUiText(error) : String(error)));
        }
        if (this.draft.scopeDraft)
            group.append(ui("p", "hint", message("hostedQuery.unappliedScopeHelp")));
        if (!parsed)
            group.append(ui("p", "hint", message("hostedQuery.invalidPlanScopeHelp")));
        if (parsed?.advanced?.call || parsed?.advanced?.unionAll)
            group.append(ui("p", "hint", message("hostedQuery.outerScopeHelp")));
        const apply = action(message("hostedQuery.applyScope"), () => {
            try {
                exactHostedScope(current, this.catalog);
                const plan = applyHostedScope(this.draft.plan, current);
                this.draft.plan = plan;
                delete this.draft.scopeDraft;
                this.changed();
                this.renderControls();
            }
            catch (error) {
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
        });
        const cancel = action(message("hostedQuery.discardScope"), () => { delete this.draft.scopeDraft; this.changed(); this.renderControls(); });
        cancel.disabled = !this.draft.scopeDraft;
        group.append(apply, cancel, ui("p", "hint", message("hostedQuery.scopeAuthorizationHelp")));
        this.scopeControls.append(group);
    }
    private renderControls() {
        const disclosures = new Map([...this.controls.querySelectorAll<HTMLDetailsElement>("details[data-query-disclosure]")].map(item => [item.dataset.queryDisclosure, item.open]));
        const active = document.activeElement, focus = active instanceof HTMLElement && this.controls.contains(active) ? active.dataset.controlKey : null;
        const selection = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement && ["text", "search", "password", "email", "url", "tel"].includes(active.type) ? { from: active.selectionStart, to: active.selectionEnd } : undefined;
        this.controls.replaceChildren();
        if (this.unreadable !== undefined) {
            this.controls.append(ui("p", "hint", message("hostedQuery.unreadableRecovery")), ui("pre", "", this.unreadable), action(message("hostedQuery.discardRecovery"), () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.renderControls(); }));
            return;
        }
        const draft = this.draft, resources = this.catalog.resources;
        this.controls.append(ui("p", "hint", this.catalog.coverage === "complete" ? message("hostedQuery.completeCatalog") : this.catalog.coverage === "partial" ? message("hostedQuery.partialCatalog") : message("hostedQuery.unknownCatalogCoverage")));
        if (this.catalog.providerCoverage === "partial")
            this.controls.append(ui("p", "hint", message("hostedQuery.partialDependencyCatalog")));
        for (const missing of this.catalog.unavailableDefinitions ?? [])
            this.controls.append(ui("p", "hint", displayMessage("hostedQuery.definitionSourceUnavailable", { p0: uiDisplayParameter(missing.resource) })));
        for (const missing of this.catalog.unavailableProviders ?? [])
            this.controls.append(ui("p", "hint", displayMessage("hostedQuery.providerUnavailable", { p0: uiDisplayParameter(missing.id) })));
        for (const missing of this.catalog.unavailableSeriesPolicies ?? [])
            this.controls.append(ui("p", "hint", displayMessage("hostedQuery.seriesPolicyUnavailable", { p0: uiDisplayParameter(missing.id) })));
        this.controls.append(this.checkboxes("resources", message("hostedQuery.resourcesLegend"), resources.map(item => item.resource), draft.resources, items => { draft.resources = items; this.changed(); this.renderControls(); }));
        const selected = resources.filter(item => draft.resources.includes(item.resource));
        this.controls.append(this.checkboxes("fields", message("hostedQuery.fieldsLegend"), [...new Set(selected.flatMap(item => item.fields))], draft.fields, items => { draft.fields = items; this.changed(); this.renderControls(); }));
        this.controls.append(this.checkboxes("products", message("hostedQuery.productsLegend"), (Object.keys(productNames) as Product[]).filter(product => selected.some(item => item.products[product])), draft.products, items => { draft.products = items; this.changed(); this.renderControls(); }));
        this.controls.append(this.checkboxes("relations", message("hostedQuery.relationsLegend"), [...new Set(selected.flatMap(item => item.relations))], draft.relations, items => { draft.relations = items; this.changed(); this.renderControls(); }));
        this.controls.append(this.dependencyControls("providers"), this.dependencyControls("seriesPolicies"), ui("p", "hint", message("hostedQuery.explicitDependencyHelp")));
        const refresh = action(message("hostedQuery.refreshCatalog"), () => this.refreshCatalog());
        refresh.disabled = this.busy;
        this.controls.append(refresh);
        const plan = input("plan", message("hostedQuery.planLabel"), draft.plan, true), parameters = input("parameters", message("hostedQuery.parametersLabel"), draft.parameters, true);
        plan.control.oninput = () => { draft.plan = plan.control.value; this.changed(); this.renderScopes(); };
        parameters.control.oninput = () => { draft.parameters = parameters.control.value; this.changed(); };
        this.controls.append(plan.host, parameters.host);
        this.renderScopes();
        this.controls.append(this.scopeControls);
        const visual = ui("details");
        visual.dataset.queryDisclosure = "visual";
        visual.append(ui("summary", "", message("hostedQuery.visualDisclosure")));
        let parsed: QueryPlan | undefined;
        try {
            parsed = parseQueryPlan(draft.plan);
        }
        catch { /* Invalid source is kept verbatim. */ }
        if (parsed && !parsed.advanced && !parsed.unnest) {
            const proposal = structuredClone(parsed), opening = draft.plan, controls = ui("details");
            controls.append(ui("summary", "", message("hostedQuery.planControlsDisclosure")), queryPlanControls(proposal, this.catalog.formSchema ?? null, () => { }), action(message("hostedQuery.adoptControlProposal"), () => {
                if (draft.plan !== opening) {
                    setUiText(this.message, message("hostedQuery.controlProposalSuperseded"));
                    return;
                }
                this.writePlan(parseQueryPlan(JSON.stringify(proposal)));
            }));
            visual.append(controls);
        }
        const where = input("visual-where", message("hostedQuery.visualWhereLabel"), draft.visual?.where ?? parsed?.where ?? ""), columns = input("visual-columns", message("hostedQuery.visualColumnsLabel"), draft.visual?.columns ?? JSON.stringify(parsed?.select ?? [], null, 2), true), order = input("visual-order", message("hostedQuery.visualOrderLabel"), draft.visual?.order ?? JSON.stringify(parsed?.order ?? [], null, 2), true);
        for (const control of [where.control, columns.control, order.control])
            control.oninput = () => { draft.visual = { where: where.control.value, columns: columns.control.value, order: order.control.value }; this.changed(); };
        visual.append(where.host, columns.host, order.host, action(message("hostedQuery.applyVisual"), () => {
            try {
                const current = parseQueryPlan(draft.plan), selection = parseQueryJson(columns.control.value), sorting = parseQueryJson(order.control.value);
                this.writePlan(parseQueryPlan(JSON.stringify({ ...current, where: where.control.value || undefined, select: selection, order: sorting })));
            }
            catch (error) {
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
        }));
        const combination = select("combination", message("hostedQuery.combinationLabel"), [["union", message("hostedQuery.unionExample")], ["distinct", message("hostedQuery.distinctExample")], ["parameters", message("hostedQuery.parameterExample")], ["related", message("hostedQuery.relatedExample")]], "union");
        visual.append(combination, action(message("hostedQuery.addCombination"), () => {
            try {
                const current = parseQueryPlan(draft.plan), advanced = { ...current.advanced };
                if (combination.value === "union")
                    advanced.unionAll = [structuredClone(current), structuredClone(current)];
                else if (combination.value === "distinct")
                    advanced.distinct = structuredClone(current.select);
                else if (combination.value === "parameters") {
                    advanced.parameters = [{ name: "wanted", shape: { kind: "scalar", semantic: "text" }, default: { kind: "text", value: "open" } }];
                    current.where = `(${current.where || "true"}) && status == wanted`;
                }
                else {
                    const relation = draft.relations[0];
                    if (!relation)
                        throw new UiError(message("hostedQuery.relationRequired"));
                    advanced.related = { relation, from: structuredClone(current), direction: "forward" };
                }
                this.writePlan({ ...current, advanced });
            }
            catch (error) {
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
        }), ui("p", "hint", message("hostedQuery.nativeExampleHelp")));
        this.controls.append(visual);
        this.renderTags();
        this.renderDefinitions();
        const run = action(this.busy ? message("hostedQuery.running") : message("hostedQuery.run"), () => this.run());
        run.disabled = this.busy || !draft.resources.length;
        this.controls.append(run);
        for (const item of this.controls.querySelectorAll<HTMLDetailsElement>("details[data-query-disclosure]"))
            item.open = disclosures.get(item.dataset.queryDisclosure) ?? false;
        if (focus) {
            const restored = this.controls.querySelector<HTMLElement>(`[data-control-key="${CSS.escape(focus)}"]`);
            restored?.focus({ preventScroll: true });
            if (selection && selection.from !== null && selection.to !== null && (restored instanceof HTMLInputElement || restored instanceof HTMLTextAreaElement))
                restored.setSelectionRange(selection.from, selection.to);
        }
    }
    private renderTags(): void {
        const section = ui("details");
        section.dataset.queryDisclosure = "tags";
        section.append(ui("summary", "", message("hostedQuery.tagsDisclosure")));
        section.append(action(message("hostedQuery.readTags"), async () => {
            const epoch = this.epoch, serial = ++this.tagReadSerial;
            this.tagTree = undefined;
            try {
                const registry = await this.request<RegistryCatalog>("navigation/registry");
                if (epoch !== this.epoch || serial !== this.tagReadSerial)
                    return;
                this.tagTree = new TagTree(registry.tags.items, registry.tags.read && registry.tags.coverage === "complete");
                this.renderControls();
            }
            catch (error) {
                if (epoch === this.epoch && serial === this.tagReadSerial)
                    setUiText(this.message, String(error));
            }
        }));
        if (this.tagTree) {
            const chosen = this.draft.tagFilter ??= { reference: "", mode: "subtree" };
            const mode = select("tag-mode", message("hostedQuery.tagModeLabel"), [["subtree", message("tagNavigation.subtreeMode")], ["exact", message("tagNavigation.exactEntry")]], chosen.mode);
            mode.onchange = () => { chosen.mode = mode.value as TagMatchMode; this.persist(); };
            section.append(tagPicker(this.tagTree, { label: message("hostedQuery.tagPickerLabel"), controlKey: "hosted-query-tags", value: chosen.reference, changed: reference => { chosen.reference = reference ?? ""; this.persist(); } }), mode, action(message("hostedQuery.addTagCondition"), () => {
                try {
                    if (!chosen.reference || !this.tagTree?.byRef.has(chosen.reference))
                        throw new UiError(message("hostedQuery.tagRequired"));
                    const plan = parseQueryPlan(this.draft.plan);
                    if (plan.advanced?.call || plan.advanced?.unionAll)
                        throw new UiError(message("hostedQuery.nestedTagConditionRefused"));
                    const field = this.catalog.formSchema?.fields.find(field => field.key === "tags");
                    if (!field)
                        throw new UiError(message("hostedQuery.tagsFieldUnavailable"));
                    const predicate = tagPredicate(chosen.reference, chosen.mode, `field(${JSON.stringify(field.id)})`);
                    plan.where = plan.where ? `(${plan.where}) && (${predicate})` : predicate;
                    this.writePlan(plan);
                }
                catch (error) {
                    setUiText(this.message, String(error));
                }
            }), ui("p", "hint", message("hostedQuery.tagScopeHelp")));
        }
        this.controls.append(section);
    }
    private renderDefinitions() {
        const draft = this.draft, section = ui("details");
        section.dataset.queryDisclosure = "saved";
        section.append(ui("summary", "", message("hostedQuery.savedDefinitionsDisclosure")));
        const owners = this.catalog.resources.filter(item => item.definitions), owner = select("definition-owner", message("hostedQuery.definitionOwnerLabel"), owners.map(item => [item.resource, item.title || item.resource]), draft.owner);
        owner.onchange = () => { draft.owner = owner.value; draft.definition = ""; this.changed(); this.renderControls(); };
        section.append(owner);
        const definitions = this.catalog.definitions.filter(item => item.resource === draft.owner), definition = select("definition", message("hostedQuery.definitionLabel"), [["", message("hostedQuery.selectDefinitionPlaceholder")], ...definitions.map(item => [item.key, item.label] as [
                string,
                UiText
            ])], draft.definition);
        definition.onchange = () => {
            draft.definition = definition.value;
            const selected = definitions.find(item => item.key === definition.value);
            if (selected?.revision)
                draft.revision = selected.revision;
            this.changed();
        };
        section.append(definition);
        const selection = select("definition-selection", message("hostedQuery.definitionModeLabel"), [["current", message("hostedQuery.currentDefinitionChoice")], ["pinned", message("hostedQuery.pinnedDefinitionChoice")]], draft.selection);
        selection.onchange = () => { draft.selection = selection.value as Draft["selection"]; this.changed(); };
        const revision = input("definition-revision", message("hostedQuery.definitionRevisionLabel"), draft.revision);
        revision.control.oninput = () => { draft.revision = revision.control.value; this.changed(); };
        section.append(selection, revision.host);
        const read = action(message("hostedQuery.readDefinition"), () => this.readDefinitions());
        read.disabled = this.busy || !draft.owner;
        const adopt = action(message("hostedQuery.createCall"), () => {
            try {
                if (!draft.owner || !draft.definition)
                    throw new UiError(message("hostedQuery.definitionRequired"));
                if (draft.selection === "pinned" && (!draft.revision || !owners.find(item => item.resource === draft.owner)?.definitionHistory))
                    throw new UiError(message("hostedQuery.exactHistoricalDefinitionRequired"));
                const plan = basePlan();
                plan.advanced = { call: { owner: draft.owner, selection: draft.selection === "pinned" ? { mode: "pinned", key: draft.definition, revision: draft.revision } : { mode: "current", key: draft.definition } } };
                this.writePlan(plan);
            }
            catch (error) {
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
        });
        section.append(read, adopt, ui("p", "hint", message("hostedQuery.definitionEditingHelp")));
        this.controls.append(section);
    }
    private async readDefinitions() {
        if (this.busy)
            return;
        const epoch = this.epoch, fingerprint = this.fingerprint(), draft = structuredClone(this.draft);
        this.busy = true;
        this.renderControls();
        try {
            if (draft.selection === "pinned" && (!draft.definition || !draft.revision || !this.catalog.resources.find(item => item.resource === draft.owner)?.definitionHistory))
                throw new UiError(message("hostedQuery.pinnedDefinitionUnavailable"));
            const result = await this.request<{
                resource: string;
                ownerRevision: string | null;
                definitions: Definition[];
                coverage?: "complete" | "partial";
                unavailable?: {
                    key: string;
                    label: string;
                    status: "unavailable";
                }[];
            }>("query/definitions", { resource: draft.owner, ...(draft.definition ? { selection: { mode: draft.selection, key: draft.definition, ...(draft.selection === "pinned" ? { revision: draft.revision } : {}) } } : {}) });
            if (epoch !== this.epoch || this.fingerprint() !== fingerprint)
                return;
            if (result.resource !== draft.owner || !Array.isArray(result.definitions))
                throw new UiError(message("hostedQuery.definitionReceiptMismatch"));
            const incoming = result.definitions.map(item => ({ ...item, resource: result.resource, revision: result.ownerRevision ?? undefined }));
            if (draft.definition && (incoming.length !== 1 || incoming[0].key !== draft.definition))
                throw new UiError(message("hostedQuery.definitionKeyMismatch"));
            // A historical read must not replace the catalog's current definition.
            if (draft.selection === "current")
                this.catalog.definitions = [...this.catalog.definitions.filter(item => item.resource !== result.resource || !!draft.definition && draft.definition !== item.key), ...incoming];
            setUiText(this.message, displayMessage("hostedQuery.definitionReadSummary", { p0: uiDisplayParameter(draft.selection === "pinned" ? message("hostedQuery.exactPinnedDefinition") : message("queryView.currentCaptionScope")), p1: uiDisplayParameter(result.ownerRevision ?? message("hostedQuery.definitionRevisionUnavailable")), p2: incoming.length, p3: uiDisplayParameter(result.coverage === "partial" ? displayMessage("hostedQuery.partialDefinitionSuffix", { p0: uiDisplayParameter((result.unavailable ?? []).map(item => `${item.label} (${item.key})`).join("、") || message("hostedQuery.partialDefinitionFallback")) }) : "") }));
        }
        catch (error) {
            if (epoch === this.epoch && this.fingerprint() === fingerprint)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.renderControls();
            }
        }
    }
    private async run() {
        if (this.busy)
            return;
        const epoch = this.epoch, fingerprint = this.fingerprint(), draft = structuredClone(this.draft), token = this.gate.begin(fingerprint);
        this.busy = true;
        setUiText(this.message, message("hostedQuery.runningInputHelp"));
        this.renderControls();
        try {
            if (draft.scopeDraft)
                throw new UiError(message("hostedQuery.scopeDecisionRequired"));
            if (draft.resources.some(resource => !this.catalog.resources.some(item => item.resource === resource)))
                throw new UiError(message("hostedQuery.objectAuthorizationChanged"));
            for (const resource of draft.resources) {
                const scope = this.catalog.resources.find(item => item.resource === resource)!;
                if (draft.fields.some(field => !scope.fields.includes(field)) || draft.products.some(product => !scope.products[product]) || draft.relations.some(relation => !scope.relations.includes(relation)))
                    throw new UiError(displayMessage("hostedQuery.objectPermissionIncomplete", { p0: uiDisplayParameter(resource) }));
            }
            const dependencies = exactHostedDependencies(draft, this.catalog);
            const plan = parseQueryPlan(draft.plan);
            exactHostedScope(plan.scope, this.catalog);
            const response = await this.request<HostedQueryResponse>("query", { resources: draft.resources, fields: draft.fields, products: draft.products, relations: draft.relations, ...dependencies, parameters: parseQueryParameters(draft.parameters), plan });
            if (!this.gate.current(token, this.fingerprint()))
                return;
            this.result = decodeHostedQuery(response);
            this.resultInput = fingerprint;
            setUiText(this.message, message("hostedQuery.frozenResultsObtained"));
        }
        catch (error) {
            if (this.gate.current(token, this.fingerprint())) {
                if (this.result)
                    this.result = { ...this.result, superseded: true };
                setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
            }
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.renderControls();
                this.renderResults();
            }
        }
    }
    private renderResults() {
        this.results.replaceChildren();
        const result = this.result;
        if (!result)
            return;
        const current = this.resultInput === this.fingerprint() && !result.superseded;
        const status = ui("p", "hint", queryResultState(result));
        status.setAttribute("role", "status");
        this.results.append(status, ui("p", "hint", displayMessage("hostedQuery.resultBasisSummary", { p0: uiDisplayParameter(result.coverage), p1: uiDisplayParameter(result.freshness), p2: uiDisplayParameter(result.limited ? message("queryStatus.displayLimitSuffix") : ""), p3: uiDisplayParameter(result.externalPending ? message("queryStatus.externalPendingSuffix") : ""), p4: uiDisplayParameter(current ? "" : message("hostedQuery.previousInputSuffix")) })));
        if (result.aggregateExact === false)
            this.results.append(ui("p", "hint", message("hostedQuery.aggregateKnownInputHelp")));
        for (const diagnostic of queryDiagnostics(result))
            this.results.append(ui("p", "hint", diagnostic));
        this.results.append(occurrenceViewControls(this.view, result.columns, () => this.renderResults()));
        const table = ui("table");
        setUiAttr(table, "aria-label", message("hostedQuery.resultTableLabel"));
        const head = ui("tr");
        for (const column of result.columns)
            head.append(ui("th", "", column));
        head.append(ui("th", "", message("hostedQuery.navigationColumn")));
        table.append(head);
        for (const row of result.rows) {
            const line = ui("tr");
            for (const column of result.columns) {
                const cell = row.cells[column], value = ui("td", `cell-${cell?.state ?? "unprojected"}`, text(cell));
                if (this.openMedia && cell)
                    for (const reference of mediaCellReferences(cell)) {
                        const open = action(message("queryPanel.openAuthorizedResource"), async () => {
                            try {
                                await this.openMedia!(reference);
                            }
                            catch (error) {
                                setUiText(this.message, error instanceof Error ? errorUiText(error) : message("hostedQuery.resourceUnavailable"));
                            }
                        });
                        open.disabled = !current;
                        value.append(open);
                    }
                line.append(value);
            }
            const target = ui("td");
            if (row.canOpen === true && row.resource) {
                const open = action(message("hostedQuery.openBody"), () => this.openRow(row));
                open.disabled = !current;
                target.append(open);
            }
            else
                setUiText(target, message("hostedQuery.noBodyRead"));
            line.append(target);
            table.append(line);
        }
        if (this.view.kind === "table")
            this.results.append(table);
        else
            renderOccurrenceCards(this.results, result, this.view, original => {
                const row = original as HostedQueryRow;
                if (row.canOpen && row.resource) {
                    const open = action(message("hostedQuery.openBody"), () => this.openRow(row));
                    open.disabled = !current;
                    return open;
                }
                return ui("span", "", message("hostedQuery.noBodyRead"));
            });
        if (this.view.kind === "table" && this.view.columns.length) {
            [...table.querySelectorAll("tr")].forEach(line => {
                [...line.children].forEach((cell, index) => {
                    if (index < result.columns.length && !this.view.columns.includes(result.columns[index]))
                        (cell as HTMLElement).hidden = true;
                });
            });
        }
        if (current && result.freshness === "current" && this.fieldActions) {
            let plan: QueryPlan | null = null;
            try {
                plan = parseQueryPlan(this.draft.plan);
            }
            catch { }
            for (const row of result.rows) {
                if (row.readOnly || !row.resource || row.lifecycle === "trash")
                    continue;
                for (const column of result.columns) {
                    const field = directQueryField(plan, this.catalog.formSchema ?? null, column);
                    if (field && this.fieldActions.canEdit(row.resource, field.id))
                        this.results.append(action(displayMessage("hostedQuery.editField", { p0: uiDisplayParameter(row.resource), p1: uiDisplayParameter(schemaFieldLabel(this.catalog.formSchema!, field)) }), () => this.fieldActions!.open(row.resource!, field.id)));
                }
            }
        }
        if (result.rows.some(row => row.lifecycle === "trash"))
            this.results.append(ui("p", "hint", message("hostedQuery.trashReadOnlyHelp")));
        if (!result.rows.length)
            this.results.append(ui("p", "hint", current && result.coverage === "complete" && result.freshness === "current" && !result.limited ? message("hostedQuery.completeEmpty") : message("hostedQuery.partialEmpty")));
        const output = action(message("hostedQuery.outputOffice"), () => this.output(result.queryId, { columns: result.columns, rowCount: result.rows.length, coverage: result.coverage }));
        output.disabled = !current || this.busy;
        this.results.append(output);
    }
    private async openRow(row: HostedQueryRow) {
        if (row.canOpen !== true || !row.resource || !this.result || this.result.superseded || this.resultInput !== this.fingerprint())
            return;
        try {
            await this.open(row.resource);
        }
        catch (error) {
            setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
        }
    }
}
import { queryDiagnostics } from "./query-result";
