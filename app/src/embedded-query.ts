import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { decodeHostedQueryResult, decodeLocalQueryResult, queryDiagnostics } from "./query-result";
import type { AuthorInputSession } from "./author-input";
import { message, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
import { diagnosticText, fieldNames, formatNativeValue, type Cell, type DataSchema } from "./data-panel";
import { queryPlanControls, queryCallControls } from "./query-plan-controls";
import { initialOccurrenceView, decodeOccurrenceView, occurrenceViewControls, occurrenceProjection, renderOccurrenceCards, type OccurrenceView } from "./query-result-view";
import { parseQueryParameters, parseQueryPlan, queryResultState, queryRowLifecycle, type QueryPlan, type QueryResult, type QueryRow } from "./query-panel";
import type { Origin, RenderNode, ViewMode } from "./types";
import { textareaSourceDraft, utf8Offset } from "./writing";
import type { FrozenQuery, EmbeddedQuerySelection } from "./export-state";
import { resourceCellReferences } from "./media-selection";
export interface EmbeddedQueryBasis {
    sessionId: string;
    source: string;
    serial: number;
}
export interface EmbeddedQueryOccurrence {
    id: string;
    key: string | null;
    label: string;
    kind: "definition" | "call";
    callFrom: number;
    range: {
        from: number;
        to: number;
        utf16From: number;
        utf16To: number;
        raw: string;
        writable: boolean;
    };
    writable: boolean;
    builderSupported: boolean;
    runnable: boolean;
    plan?: QueryPlan | null;
    reason?: string | null;
    controlReason?: string | null;
    binding?: {
        schema: string;
        revision: string;
        language: string;
    };
}
export interface EmbeddedQueryDiscovery {
    serial: number;
    source?: string;
    sourceHash: string;
    syntaxValid: boolean;
    occurrences: EmbeddedQueryOccurrence[];
}
export interface EmbeddedQueryRun {
    serial: number;
    source?: string;
    sourceHash: string;
    occurrenceId: string;
    draftPreview: boolean;
    result: QueryResult;
}
export interface EmbeddedQueryEdit {
    serial?: number;
    sourceHash?: string;
    patchUnit?: string;
    source: string;
    patches: {
        from: number;
        to: number;
        text: string;
        byteFrom?: number;
        byteTo?: number;
        expected?: string;
    }[];
}
export interface EmbeddedQueryAssociation {
    label: string;
    origin: Origin | null;
    occurrence: EmbeddedQueryOccurrence | null;
    unique: boolean;
}
export function sameQueryBasis(left: EmbeddedQueryBasis | null, right: EmbeddedQueryBasis): boolean {
    return !!left && left.sessionId === right.sessionId && left.serial === right.serial && left.source === right.source;
}
/** The complete author range comes from the official AST, never displayed text. */
export function exactQueryOccurrence(source: string, occurrence: EmbeddedQueryOccurrence): boolean {
    const range = occurrence.range;
    return Number.isSafeInteger(range.utf16From) && Number.isSafeInteger(range.utf16To)
        && range.utf16From >= 0 && range.utf16To >= range.utf16From && range.utf16To <= source.length
        && sourceBoundary(source, range.utf16From) && sourceBoundary(source, range.utf16To)
        && source.slice(range.utf16From, range.utf16To) === range.raw
        && utf8Offset(source, range.utf16From) === range.from && utf8Offset(source, range.utf16To) === range.to;
}
/** Only consumed native metadata tags can host a query. Repeated output has no unique writer. */
export function associateEmbeddedQueries(tree: RenderNode, occurrences: readonly EmbeddedQueryOccurrence[]): Map<string, EmbeddedQueryAssociation> {
    const tags: RenderNode[] = [];
    const visit = (node: RenderNode): void => {
        if (node.kind === "tag" && typeof node.attrs["data-weftext-query-label"] === "string")
            tags.push(node);
        node.children.forEach(visit);
    };
    visit(tree);
    const matched = tags.map(tag => {
        const label = String(tag.attrs["data-weftext-query-label"]);
        const candidates = tag.origin?.isMain ? occurrences.filter(item => item.label === label && item.callFrom === tag.origin!.from) : [];
        return { tag, label, occurrence: candidates.length === 1 ? candidates[0] : null };
    });
    return new Map(matched.map(({ tag, label, occurrence }) => [tag.key, { label, origin: tag.origin, occurrence, unique: !!occurrence && matched.filter(item => item.occurrence?.id === occurrence.id).length === 1 }]));
}
export function embeddedQueryPolicy(mode: ViewMode, current: boolean, writable: boolean, builderSupported: boolean) {
    const edit = mode !== "read" && current && writable;
    return { sourceVisible: mode === "styled", sourcePreview: mode === "source", reveal: mode === "focus" && edit, builder: mode === "write" && edit && builderSupported, sourceDialog: mode === "write" && edit, edit };
}
/** A backend response must preserve all bytes outside the selected native carrier. */
export function validateEmbeddedEdit(basis: EmbeddedQueryBasis, occurrence: EmbeddedQueryOccurrence, result: EmbeddedQueryEdit, expectedHash?: string): SourcePatch[] {
    if (!exactQueryOccurrence(basis.source, occurrence) || !occurrence.writable || !occurrence.range.writable || result.serial !== undefined && result.serial !== basis.serial)
        throw new UiError(message("embeddedQuery.sourceBasisChanged"));
    const patches = result.patches.map(patch => ({ from: patch.from, to: patch.to, expected: basis.source.slice(patch.from, patch.to), insert: patch.text }));
    if (result.patchUnit !== undefined && result.patchUnit !== "utf16" || expectedHash !== undefined && result.sourceHash !== expectedHash || result.patches.some(patch => patch.byteFrom !== undefined && patch.byteFrom !== utf8Offset(basis.source, patch.from) || patch.byteTo !== undefined && patch.byteTo !== utf8Offset(basis.source, patch.to) || patch.expected !== undefined && patch.expected !== basis.source.slice(patch.from, patch.to)))
        throw new UiError(message("embeddedQuery.patchByteBasisMismatch"));
    if (patches.some(patch => patch.from < occurrence.range.utf16From || patch.to > occurrence.range.utf16To))
        throw new UiError(message("embeddedQuery.editOutsideDefinition"));
    if (applySourcePatches(basis.source, patches) !== result.source)
        throw new UiError(message("embeddedQuery.patchDraftMismatch"));
    return patches;
}
export function validateQueryArrangement(basis: EmbeddedQueryBasis, item: EmbeddedQueryOccurrence, occurrences: readonly EmbeddedQueryOccurrence[], command: "up" | "down" | "remove", result: EmbeddedQueryEdit, hash?: string): SourcePatch[] {
    let from = item.range.utf16From, to = item.range.utf16To, replacement = "";
    if (command !== "remove") {
        const sorted = [...occurrences].sort((a, b) => a.range.utf16From - b.range.utf16From), index = sorted.findIndex(q => q.id === item.id), other = sorted[index + (command === "up" ? -1 : 1)];
        if (!other?.writable || !exactQueryOccurrence(basis.source, other))
            throw new UiError(message("embeddedQuery.adjacentWriteTargetMissing"));
        const [first, last] = item.range.utf16From < other.range.utf16From ? [item, other] : [other, item];
        const separator = basis.source.slice(first.range.utf16To, last.range.utf16From);
        if (!/^\s*$/.test(separator))
            throw new UiError(message("embeddedQuery.moveAcrossContentRefused"));
        from = first.range.utf16From;
        to = last.range.utf16To;
        replacement = last.range.raw + separator + first.range.raw;
    }
    const expected = basis.source.slice(0, from) + replacement + basis.source.slice(to);
    if (result.source !== expected)
        throw new UiError(message("embeddedQuery.arrangementContentMismatch"));
    return validateEmbeddedEdit(basis, { ...item, range: { ...item.range, from: utf8Offset(basis.source, from), to: utf8Offset(basis.source, to), utf16From: from, utf16To: to, raw: basis.source.slice(from, to) } }, result, hash);
}
/** Source serial and request generation both matter: even Q1 → Q2 → Q1 is a new draft. */
export class EmbeddedQueryRunGate {
    private generation = 0;
    begin(basis: EmbeddedQueryBasis, occurrenceId: string, sourceHash?: string) { return { basis: { ...basis }, occurrenceId, sourceHash, generation: ++this.generation }; }
    invalidate(): void { this.generation++; }
    current(token: ReturnType<EmbeddedQueryRunGate["begin"]>, basis: EmbeddedQueryBasis | null): boolean { return token.generation === this.generation && sameQueryBasis(basis, token.basis); }
    accepts(token: ReturnType<EmbeddedQueryRunGate["begin"]>, current: EmbeddedQueryBasis | null, result: EmbeddedQueryRun): boolean {
        return this.current(token, current) && result.serial === token.basis.serial && result.occurrenceId === token.occurrenceId && (result.source === undefined || result.source === token.basis.source) && (token.sourceHash === undefined || result.sourceHash === token.sourceHash);
    }
}
interface QueryPresentation {
    view: OccurrenceView;
    gate: EmbeddedQueryRunGate;
    result?: QueryResult;
    resultBasis?: EmbeddedQueryBasis;
    resultPlan?: QueryPlan;
    loading: boolean;
    error?: UiText;
    expanded: boolean;
    builderOpen: boolean;
    parameters: string;
    authorInputs?: AuthorInputSession;
    authorSessionId?: string;
    planDraft?: QueryPlan;
    planRevision?: number;
    pendingEdit?: object;
    planBasis?: EmbeddedQueryBasis;
    autoRunBasis?: EmbeddedQueryBasis;
}
export interface EmbeddedQueryHost {
    queryResultScope: "local" | "hosted";
    authorInputs?(): AuthorInputSession | undefined;
    preferenceIdentity?(): string | null;
    catalog?(): Promise<DataSchema | null>;
    openDefinition?(owner: string, key: string): Promise<void>;
    basis(): EmbeddedQueryBasis;
    mode(): ViewMode;
    editable(): boolean;
    request<T>(path: string, body: unknown): Promise<T>;
    prepareEdit(): Promise<void>;
    apply(patches: SourcePatch[], basis: EmbeddedQueryBasis, event: string): Promise<void>;
    reveal(occurrence: EmbeddedQueryOccurrence, host: HTMLElement): boolean;
    closeReveal(): void;
    openRow(row: QueryRow, plan: QueryPlan | null): Promise<void>;
    /** Hosted targets carry independent navigation authority, without local paths. */
    canOpenRow?(row: QueryRow, plan: QueryPlan | null): boolean;
    openResource?(reference: string): Promise<void>;
    openOwner(origin: Origin): Promise<void>;
    canOpenOwner?(origin: Origin): boolean;
    output?(query: FrozenQuery, result: QueryResult): void | Promise<void>;
    notify(message: UiText, error?: boolean): void;
}
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] { const result = document.createElement(tag); result.className = className; setUiText(result, text); return result; }
function action(label: UiText, run: () => void | Promise<void>, host: EmbeddedQueryHost): HTMLButtonElement { const button = element("button", "embedded-query-button", label); button.type = "button"; button.onclick = () => { void Promise.resolve().then(run).catch(error => host.notify(error instanceof Error ? errorUiText(error) : String(error), true)); }; return button; }
function cellText(cell: Cell): UiText { return cell.state === "value" ? formatNativeValue(cell.value) : ({ absent: message("queryValue.absent"), null: "none", invalid: message("queryValue.invalid"), unavailable: message("queryValue.unavailable") } as const)[cell.state]; }
const presentationIdentity = (item: EmbeddedQueryOccurrence): string => JSON.stringify([item.key, item.label, item.kind]);
/** Session-only presentation. Native source and the shared CodeMirror history remain authoritative. */
export class EmbeddedQueryController {
    private pendingFocus: string | null = null;
    private formSchema: DataSchema | null = null;
    private discovery: EmbeddedQueryDiscovery | null = null;
    private discoveryBasis: EmbeddedQueryBasis | null = null;
    private discoveryGeneration = 0;
    private states = new Map<string, QueryPresentation>();
    private mounts = new Map<HTMLElement, EmbeddedQueryAssociation>();
    private dialog: HTMLDialogElement | null = null;
    private associations = new Map<string, EmbeddedQueryAssociation>();
    constructor(private readonly host: EmbeddedQueryHost) { }
    dispose(): void {
        for (const state of this.states.values())
            state.authorInputs?.retire();
        this.formSchema = null;
        this.invalidate();
        this.dialog?.close();
        this.dialog?.remove();
        this.dialog = null;
        this.discovery = null;
        this.discoveryBasis = null;
        this.states.clear();
        this.mounts.clear();
        this.associations.clear();
    }
    get hasUnappliedAuthorInputs(): boolean { return !!this.dialog?.open || [...this.states.values()].some(state => !!state.planDraft); }
    get hasQueries(): boolean { return !!this.discovery?.occurrences.length; }
    get occurrences(): readonly EmbeddedQueryOccurrence[] { return this.discovery?.occurrences ?? []; }
    outputSelections(): EmbeddedQuerySelection[] {
        const basis = this.host.basis();
        if (!sameQueryBasis(this.discoveryBasis, basis))
            return [];
        return this.occurrences.flatMap(item => {
            const state = this.states.get(presentationIdentity(item));
            return state?.result?.queryId && !state.loading && !state.error && !state.result.superseded && sameQueryBasis(state.resultBasis ?? null, basis)
                ? [{ occurrenceId: item.id, queryId: state.result.queryId }] : [];
        });
    }
    private viewKey(item: EmbeddedQueryOccurrence): string | null { const identity = this.host.preferenceIdentity?.(); return identity && item.key ? `weftext-occurrence-view-v1:${identity}:${item.key}` : null; }
    private state(item: EmbeddedQueryOccurrence): QueryPresentation {
        const key = presentationIdentity(item);
        let state = this.states.get(key);
        if (!state) {
            let view = initialOccurrenceView();
            const storage = this.viewKey(item);
            if (storage) {
                try {
                    view = decodeOccurrenceView(localStorage.getItem(storage));
                }
                catch {
                    view.warning = message("embeddedQuery.viewStorageUnavailable");
                }
            }
            state = { view, gate: new EmbeddedQueryRunGate(), loading: false, expanded: false, builderOpen: false, parameters: "{}" };
            this.states.set(key, state);
        }
        return state;
    }
    invalidate(): void {
        this.discoveryGeneration++;
        for (const state of this.states.values()) {
            state.gate.invalidate();
            state.loading = false;
        }
        for (const [mount, association] of this.mounts)
            this.renderStatusOnly(mount, association);
    }
    /** Host-owned execution inputs change independently of native author source. */
    runInputsChanged(): void {
        for (const state of this.states.values()) {
            state.gate.invalidate();
            state.loading = false;
            state.resultBasis = undefined;
            if (state.result)
                state.result = { ...state.result, superseded: true };
        }
        for (const [mount, association] of this.mounts) {
            if (association.occurrence)
                this.renderResultArea(mount, association, this.state(association.occurrence));
            this.renderStatusOnly(mount, association);
        }
    }
    async discover(basis: EmbeddedQueryBasis): Promise<boolean> {
        const generation = ++this.discoveryGeneration;
        const discovery = await this.host.request<EmbeddedQueryDiscovery>("/api/query/embedded/discover", basis);
        if (generation !== this.discoveryGeneration || !sameQueryBasis(this.host.basis(), basis) || discovery.serial !== basis.serial || discovery.source !== undefined && discovery.source !== basis.source)
            return false;
        if (discovery.occurrences.some(item => !exactQueryOccurrence(basis.source, item)))
            throw new UiError(message("embeddedQuery.discoveryRangeMismatch"));
        this.discovery = discovery;
        this.discoveryBasis = { ...basis };
        const identities = new Set(discovery.occurrences.map(presentationIdentity));
        for (const [key, state] of this.states)
            if (!identities.has(key)) {
                state.gate.invalidate();
                state.authorInputs?.retire();
                this.states.delete(key);
            }
        return true;
    }
    beginProjection(tree: RenderNode, basis: EmbeddedQueryBasis): void {
        this.mounts.clear();
        this.associations = associateEmbeddedQueries(tree, sameQueryBasis(this.discoveryBasis, basis) ? this.occurrences : []);
    }
    detach(): void { this.mounts.clear(); this.associations.clear(); }
    closeExpansion(): void {
        for (const state of this.states.values())
            state.expanded = false;
    }
    modeChanged(): void {
        if (this.host.mode() !== "write")
            this.dialog?.close();
        if (this.host.mode() !== "focus")
            this.closeExpansion();
    }
    forNode(node: RenderNode): HTMLElement | null {
        const association = this.associations.get(node.key);
        if (!association)
            return null;
        const mount = element("section", "embedded-query");
        mount.dataset.queryLabel = association.label;
        setUiAttr(mount, "aria-label", displayMessage("embeddedQuery.regionLabel", { p0: uiDisplayParameter(association.label) }));
        this.mounts.set(mount, association);
        this.render(mount, association);
        if (this.pendingFocus === association.label) {
            this.pendingFocus = null;
            queueMicrotask(() => {
                if (mount.isConnected)
                    mount.querySelector<HTMLElement>("summary")?.focus({ preventScroll: true });
            });
        }
        const item = association.occurrence;
        if (item && item.runnable && this.current(association)) {
            const state = this.state(item), basis = this.host.basis();
            if (!sameQueryBasis(state.autoRunBasis ?? null, basis)) {
                state.autoRunBasis = { ...basis };
                queueMicrotask(() => { void this.run(association).catch(error => this.host.notify(error instanceof Error ? errorUiText(error) : String(error), true)); });
            }
        }
        return mount;
    }
    private current(association: EmbeddedQueryAssociation): boolean { return !!association.occurrence && sameQueryBasis(this.discoveryBasis, this.host.basis()) && exactQueryOccurrence(this.host.basis().source, association.occurrence); }
    private canWrite(association: EmbeddedQueryAssociation): boolean { return this.host.editable() && association.unique && association.occurrence?.writable === true && association.occurrence.range.writable; }
    private refresh(item: EmbeddedQueryOccurrence): void {
        for (const [mount, association] of this.mounts)
            if (association.occurrence?.id === item.id)
                this.render(mount, association);
    }
    private refreshResult(item: EmbeddedQueryOccurrence): void {
        for (const [mount, association] of this.mounts)
            if (association.occurrence?.id === item.id) {
                this.renderResultArea(mount, association, this.state(item));
                this.renderStatusOnly(mount, association);
            }
    }
    private renderStatusOnly(mount: HTMLElement, association: EmbeddedQueryAssociation): void {
        const status = mount.querySelector<HTMLElement>(".embedded-query-basis");
        if (status)
            setUiText(status, this.statusText(association));
        for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-edit]"))
            button.disabled = !this.current(association) || !this.canWrite(association) || this.host.mode() === "read"
                || button.dataset.queryEdit === "builderApply" && (!association.occurrence || !!this.state(association.occurrence).pendingEdit || !sameQueryBasis(this.state(association.occurrence).planBasis ?? null, this.host.basis()));
        if (association.occurrence) {
            const item = association.occurrence, state = this.state(item);
            for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-run]"))
                button.disabled = !this.current(association) || !item.runnable || state.loading;
            for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-output]"))
                button.disabled = !this.current(association) || state.loading || !!state.result?.superseded || !sameQueryBasis(state.resultBasis ?? null, this.host.basis());
        }
        if (association.occurrence) {
            const state = this.state(association.occurrence), status = mount.querySelector<HTMLElement>(".embedded-query-result-state");
            if (status && state.result) {
                const result = state.result;
                setUiText(status, () => uiTextValue(queryResultState(result)) + uiTextValue((sameQueryBasis(state.resultBasis ?? null, this.host.basis()) ? "" : message("embeddedQuery.oldDraftSuffix"))));
            }
        }
    }
    private statusText(association: EmbeddedQueryAssociation): UiText {
        const item = association.occurrence;
        if (!item)
            return message("embeddedQuery.unassociatedSource");
        const state = this.state(item);
        if (!this.current(association))
            return message("embeddedQuery.relocatingSource");
        if (state.resultBasis && !sameQueryBasis(state.resultBasis, this.host.basis()))
            return message("embeddedQuery.currentNotRun");
        return message("embeddedQuery.draftPreviewHelp");
    }
    private render(mount: HTMLElement, association: EmbeddedQueryAssociation): void {
        mount.replaceChildren();
        const item = association.occurrence;
        const heading = element("div", "embedded-query-heading");
        heading.append(element("strong", "", association.label.replace(/^weftext-query-/, "")));
        mount.append(heading, element("p", "embedded-query-basis", this.statusText(association)));
        if (!item) {
            if (association.origin && !association.origin.isMain) {
                if (this.host.canOpenOwner?.(association.origin) !== false)
                    heading.append(action(message("embeddedQuery.viewSourceNote"), () => this.host.openOwner(association.origin!), this.host));
                else
                    mount.append(element("p", "embedded-query-help", message("embeddedQuery.sourceNavigationUnavailable")));
            }
            return;
        }
        const state = this.state(item), mode = this.host.mode();
        const policy = embeddedQueryPolicy(mode, this.current(association), this.canWrite(association), item.builderSupported && !!item.plan);
        const run = action(state.loading ? message("embeddedQuery.running") : state.result ? message("embeddedQuery.refreshResults") : message("embeddedQuery.runDraft"), () => this.run(association), this.host);
        run.dataset.queryRun = "run";
        run.disabled = state.loading || !this.current(association) || !item.runnable;
        heading.append(run);
        if (policy.sourceVisible) {
            const source = element("pre", "embedded-query-native-source", item.range.raw);
            setUiAttr(source, "aria-label", message("embeddedQuery.completeSourceLabel"));
            mount.append(source);
        }
        if (policy.sourcePreview)
            mount.append(element("p", "embedded-query-help", message("embeddedQuery.sourcePreviewHelp")));
        if (policy.reveal) {
            const toggle = action(state.expanded ? message("embeddedQuery.collapseSource") : message("embeddedQuery.expandSource"), () => {
                const expanded = !state.expanded;
                this.closeExpansion();
                this.host.closeReveal();
                state.expanded = expanded;
                for (const [other, entry] of this.mounts)
                    this.render(other, entry);
            }, this.host);
            toggle.dataset.queryEdit = "source";
            toggle.setAttribute("aria-expanded", String(state.expanded));
            heading.append(toggle);
        }
        if (policy.builder) {
            const toggle = action(state.builderOpen ? message("embeddedQuery.collapseBuilder") : message("embeddedQuery.openBuilder"), async () => {
                const basis = { ...this.host.basis() };
                if (!state.builderOpen && this.host.catalog) {
                    const schema = await this.host.catalog();
                    if (!sameQueryBasis(basis, this.host.basis()) || !this.current(association))
                        throw new UiError(message("embeddedQuery.builderCatalogSuperseded"));
                    this.formSchema = schema;
                }
                state.builderOpen = !state.builderOpen;
                this.refresh(item);
            }, this.host);
            toggle.dataset.queryEdit = "builder";
            toggle.setAttribute("aria-expanded", String(state.builderOpen));
            heading.append(toggle);
        }
        if (policy.sourceDialog) {
            const source = action(message("embeddedQuery.editSource"), () => this.editSource(association), this.host);
            source.dataset.queryEdit = "source";
            heading.append(source);
        }
        if (mode === "write" && !item.builderSupported)
            mount.append(element("p", "embedded-query-warning", item.controlReason ?? item.reason ?? message("embeddedQuery.builderUnsupported")));
        if (!item.runnable)
            mount.append(element("p", "embedded-query-warning", displayMessage("embeddedQuery.previewUnavailable", { p0: uiDisplayParameter(item.reason ?? item.controlReason ?? message("embeddedQuery.executionEligibilityUnavailable")) })));
        if (!association.unique || !item.writable)
            mount.append(element("p", "embedded-query-warning", item.reason ?? message("embeddedQuery.occurrenceReadOnly")));
        if (mode === "read")
            mount.append(element("p", "embedded-query-help", message("embeddedQuery.readModeHelp")));
        if (policy.edit) {
            const more = element("details", "embedded-query-more");
            more.append(element("summary", "", message("embeddedQuery.moreDisclosure")));
            more.append(action(message("embeddedQuery.moveUp"), () => this.arrange(association, "up"), this.host), action(message("embeddedQuery.moveDown"), () => this.arrange(association, "down"), this.host), action(message("embeddedQuery.removeHere"), () => this.confirmRemoval(association), this.host));
            heading.append(more);
        }
        if (item.plan?.advanced?.call) {
            const call = item.plan.advanced.call;
            mount.append(element("p", "embedded-query-help", displayMessage("embeddedQuery.callOwnerHelp", { p0: uiDisplayParameter(call.selection.mode === "pinned" ? displayMessage("embeddedQuery.pinnedCallDefinition", { p0: uiDisplayParameter(call.selection.revision) }) : message("embeddedQuery.currentCallDefinition")), p1: uiDisplayParameter(call.owner) })));
            if (this.host.openDefinition)
                mount.append(action(message("embeddedQuery.openDefinitionOwner"), () => this.host.openDefinition!(call.owner, call.selection.key), this.host));
        }
        if (mode !== "read") {
            const parameters = element("details", "embedded-query-parameters");
            parameters.append(element("summary", "", message("embeddedQuery.parametersDisclosure")));
            const input = element("textarea", "embedded-query-input");
            input.value = state.parameters;
            input.rows = 3;
            setUiAttr(input, "aria-label", displayMessage("embeddedQuery.parametersLabel", { p0: uiDisplayParameter(association.label) }));
            input.oninput = () => {
                state.parameters = input.value;
                state.gate.invalidate();
                state.loading = false;
                if (state.result) {
                    state.resultBasis = undefined;
                    state.result = { ...state.result, superseded: true };
                }
                this.refreshResult(item);
            };
            parameters.append(input, element("p", "embedded-query-help", message("embeddedQuery.previewParametersHelp")));
            mount.append(parameters);
        }
        mount.append(element("div", "embedded-query-results"));
        this.renderResultArea(mount, association, state);
        if (policy.builder && state.builderOpen)
            this.renderBuilder(mount, association, state);
        if (policy.reveal && state.expanded) {
            const source = element("div", "embedded-query-source-host");
            mount.append(source);
            if (!this.host.reveal(item, source)) {
                state.expanded = false;
                source.replaceWith(element("p", "embedded-query-warning", message("embeddedQuery.sourceExpansionUnavailable")));
            }
        }
    }
    /** Result arrivals update only their derived area, leaving focused controls and source selection intact. */
    private renderResultArea(mount: HTMLElement, association: EmbeddedQueryAssociation, state: QueryPresentation): void {
        const area = mount.querySelector<HTMLElement>(".embedded-query-results"), item = association.occurrence;
        if (!area || !item)
            return;
        const active = document.activeElement instanceof HTMLElement && area.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.controlKey : null, viewOpen = area.querySelector<HTMLDetailsElement>(".embedded-query-view")?.open;
        area.replaceChildren();
        const run = mount.querySelector<HTMLButtonElement>("[data-query-run]");
        if (run) {
            setUiText(run, state.loading ? message("embeddedQuery.running") : state.result ? message("embeddedQuery.refreshResults") : message("embeddedQuery.runDraft"));
            run.disabled = state.loading || !item.runnable || !this.current(association);
        }
        if (state.error)
            area.append(element("p", "embedded-query-warning", state.error));
        if (state.result)
            this.renderResult(area, state.result, state.resultPlan ?? null, sameQueryBasis(state.resultBasis ?? null, this.host.basis()), state.view, () => {
                state.view.projection = occurrenceProjection(state.resultPlan ?? null, state.result?.schemaRevision);
                delete state.view.warning;
                const key = this.viewKey(item);
                if (key) {
                    try {
                        localStorage.setItem(key, JSON.stringify(state.view));
                    }
                    catch {
                        state.view.warning = message("embeddedQuery.viewPersistenceFailed");
                    }
                }
                this.refreshResult(item);
            });
        else
            area.append(element("p", "embedded-query-help", state.loading ? message("embeddedQuery.computing") : message("embeddedQuery.noResultYet")));
        if (state.result?.queryId && this.host.output) {
            const result = state.result;
            const output = action(message("queryPanel.outputResults"), () => {
                if (!sameQueryBasis(state.resultBasis ?? null, this.host.basis()) || result.superseded)
                    throw new UiError(message("embeddedQuery.outputRequiresCurrentRun"));
                return this.host.output!({ queryId: result.queryId!, label: item.label, rowCount: result.rows.length, coverage: result.coverage, freshness: result.freshness, limited: result.limited }, result);
            }, this.host);
            output.dataset.queryOutput = "output";
            output.disabled = state.loading || !!result.superseded || !sameQueryBasis(state.resultBasis ?? null, this.host.basis());
            area.append(output);
        }
        const view = area.querySelector<HTMLDetailsElement>(".embedded-query-view");
        if (view && viewOpen !== undefined)
            view.open = viewOpen;
        if (active)
            area.querySelector<HTMLElement>(`[data-control-key="${CSS.escape(active)}"]`)?.focus({ preventScroll: true });
    }
    private async run(association: EmbeddedQueryAssociation): Promise<void> {
        const item = association.occurrence;
        if (!item || !this.current(association) || !item.runnable)
            return;
        const basis = { ...this.host.basis() }, state = this.state(item), parameters = parseQueryParameters(state.parameters), token = state.gate.begin(basis, item.id, this.discovery?.sourceHash);
        state.loading = true;
        state.error = undefined;
        this.refreshResult(item);
        try {
            const result = await this.host.request<EmbeddedQueryRun>("/api/query/embedded/run", { ...basis, occurrenceId: item.id, parameters });
            if (!state.gate.accepts(token, this.host.basis(), result))
                return;
            state.result = this.host.queryResultScope === "hosted" ? decodeHostedQueryResult(result.result,true) : decodeLocalQueryResult(result.result);
            state.resultBasis = basis;
            state.resultPlan = item.plan ?? undefined;
            state.error = undefined;
        }
        catch (error) {
            if (state.gate.current(token, this.host.basis())) {
                state.error = error instanceof Error ? errorUiText(error) : String(error);
                if (state.result)
                    state.result = { ...state.result, superseded: true };
            }
        }
        finally {
            if (state.gate.current(token, this.host.basis())) {
                state.loading = false;
                this.refreshResult(item);
            }
        }
    }
    private renderResult(mount: HTMLElement, result: QueryResult, plan: QueryPlan | null, definitionCurrent: boolean, view: OccurrenceView, update: () => void): void {
        const status = element("p", `embedded-query-result-state ${result.coverage} ${result.freshness}`, () => uiTextValue(queryResultState(result)) + uiTextValue((definitionCurrent ? "" : message("embeddedQuery.oldInputSuffix"))));
        status.setAttribute("role", "status");
        mount.append(status);
        // Keep these distinct even when the existing summary chooses one primary state.
        mount.append(element("p", "embedded-query-help", displayMessage("embeddedQuery.resultBasisSummary", { p0: uiDisplayParameter(result.coverage === "complete" ? message("queryCoverage.complete") : message("queryCoverage.partial")), p1: uiDisplayParameter(result.freshness), p2: uiDisplayParameter(result.limited ? message("queryStatus.displayLimitSuffix") : ""), p3: uiDisplayParameter(result.externalPending ? message("queryStatus.externalPendingSuffix") : "") })));
        if (result.cutId)
            mount.append(element("p", "embedded-query-help", displayMessage("queryStatus.readRevision", { p0: uiDisplayParameter(result.cutId) })));
        if (result.aggregateExact === false)
            mount.append(element("p", "embedded-query-warning", message("embeddedQuery.aggregateKnownInputHelp")));
        for (const diagnostic of queryDiagnostics(result))
            mount.append(element("p", "embedded-query-warning", diagnostic));
        if (!result.rows.length)
            mount.append(element("p", "embedded-query-empty", definitionCurrent && result.coverage === "complete" && !result.superseded && result.snapshotMode === "admittedFrozen" ? message("embeddedQuery.frozenCompleteEmpty") : definitionCurrent && result.coverage === "complete" && result.freshness === "current" && !result.superseded ? message("embeddedQuery.statedScopeCompleteEmpty") : message("embeddedQuery.knownScopeEmpty")));
        const allColumns = result.columns ?? [...new Set(result.rows.flatMap(row => Object.keys(row.cells)))];
        mount.append(occurrenceViewControls(view, allColumns, update));
        if (view.warning)
            mount.append(element("p", "embedded-query-warning", view.warning));
        const projection = occurrenceProjection(plan, result.schemaRevision);
        if (view.projection && view.projection !== projection) {
            mount.append(element("p", "embedded-query-warning", message("embeddedQuery.projectionChanged")));
            view = initialOccurrenceView();
        }
        const openRow = (row: QueryRow): HTMLElement => {
            const title = row.cells.title?.state === "value" ? formatNativeValue(row.cells.title.value) : row.path?.replaceAll("\\", "/").split("/").at(-1) ?? (row as {
                resource?: string;
            }).resource ?? message("queryValue.generatedRowFallback");
            return (this.host.canOpenRow ? this.host.canOpenRow(row, plan) : row.path && queryRowLifecycle(row, plan)) ? action(title, () => this.host.openRow(row, plan), this.host) : element("span", "", title);
        };
        if (view.kind !== "table") {
            renderOccurrenceCards(mount, result, view, openRow);
            return;
        }
        const tableHost = element("div", "embedded-query-table-host"), table = element("table", "embedded-query-table"), header = element("tr");
        const columns = allColumns.filter(column => !view.columns.length || view.columns.includes(column));
        header.append(element("th", "", message("queryColumn.note")));
        for (const column of columns)
            header.append(element("th", "", fieldNames[column] ?? column));
        const thead = element("thead");
        thead.append(header);
        table.append(thead);
        const tbody = element("tbody");
        for (const row of result.rows) {
            const tr = element("tr"), target = element("td");
            const title = row.cells.title?.state === "value" ? formatNativeValue(row.cells.title.value) : row.path?.replaceAll("\\", "/").split("/").at(-1) ?? (row as {
                resource?: string;
            }).resource ?? message("queryValue.generatedRowFallback");
            if (this.host.canOpenRow ? this.host.canOpenRow(row, plan) : row.path && queryRowLifecycle(row, plan))
                target.append(action(title, () => this.host.openRow(row, plan), this.host));
            else
                setUiText(target, title);
            tr.append(target);
            for (const column of columns) {
                const cell = row.cells[column], td = element("td", `cell-${cell?.state ?? "unprojected"}`, cell ? cellText(cell) : message("queryValue.unprojected"));
                if (cell && this.host.openResource)
                    for (const reference of resourceCellReferences(cell)) {
                        const open = action(message("queryPanel.openAuthorizedResource"), () => this.host.openResource!(reference), this.host);
                        open.disabled = !definitionCurrent || result.superseded === true;
                        td.append(open);
                    }
                tr.append(td);
            }
            tbody.append(tr);
        }
        table.append(tbody);
        tableHost.append(table);
        mount.append(tableHost);
    }
    private renderBuilder(mount: HTMLElement, association: EmbeddedQueryAssociation, state: QueryPresentation): void {
        const item = association.occurrence!, plan = state.planDraft ?? structuredClone(item.plan!);
        state.planDraft = plan;
        state.planBasis ??= { ...this.host.basis() };
        const builder = element("form", "embedded-query-builder");
        state.planRevision ??= 0;
        const changed = () => { if (state.planDraft === plan) state.planRevision = (state.planRevision ?? 0) + 1; };
        // Incomplete raw controls are proposals too, even when they cannot update the plan yet.
        builder.addEventListener("input", changed);
        builder.addEventListener("change", changed);
        builder.append(element("strong", "", message("embeddedQuery.builderTitle")));
        const currentBasis = this.host.basis();
        if (state.authorSessionId !== currentBasis.sessionId) {
            state.authorInputs?.retire();
            state.authorInputs = undefined;
            state.authorSessionId = currentBasis.sessionId;
        }
        const inputs = state.authorInputs ??= this.host.authorInputs?.();
        builder.append(plan.advanced?.call ? queryCallControls(plan, inputs) : queryPlanControls(plan, this.formSchema, changed, undefined, inputs));
        const add = (role: string, label: UiText, value: string, update: (value: string) => void, multiline = false): HTMLInputElement | HTMLTextAreaElement => { const field = element("label", "embedded-query-field", label), input = multiline ? element("textarea", "embedded-query-input") : element("input", "embedded-query-input"); input.value = value; inputs?.bind(input, `query-author:${role}`); input.dataset.controlKey = JSON.stringify(["embedded-query-builder", item.id, role]); setUiAttr(input, "aria-label", () => `${uiTextValue(item.label)} ${uiTextValue(label)}`); input.oninput = () => update(input.value); field.append(input); builder.append(field); return input; };
        if (!plan.advanced?.call) {
            add("types", message("embeddedQuery.typesLabel"), plan.types.join(", "), value => { plan.types = value.split(",").map(item => item.trim()).filter(Boolean); });
            add("where", message("embeddedQuery.conditionLabel"), plan.where ?? "", value => {
                if (value.trim())
                    plan.where = value;
                else
                    delete plan.where;
            }, true);
            const take = add("take", message("embeddedQuery.takeLabel"), plan.take == null ? "" : String(plan.take), value => {
                if (!value.trim())
                    delete plan.take;
                else
                    plan.take = Number(value);
            });
            if (take instanceof HTMLInputElement) {
                take.type = "number";
                take.min = "1";
                take.step = "1";
            }
            const descendants = element("input");
            descendants.type = "checkbox";
            descendants.checked = plan.scope.descendants;
            descendants.onchange = () => { plan.scope.descendants = descendants.checked; };
            const scope = element("label", "embedded-query-field", message("queryPanel.includeDescendants"));
            scope.append(descendants);
            builder.append(scope);
        }
        builder.append(element("p", "embedded-query-help", message("embeddedQuery.builderApplyHelp")));
        const apply = action(message("embeddedQuery.applyBuilder"), async () => {
            if (!apply.isConnected || state.pendingEdit || state.planDraft !== plan || !sameQueryBasis(state.planBasis ?? null, this.host.basis()))
                throw new UiError(message("embeddedQuery.editBasisChanged"));
            const serialized = JSON.stringify(plan), proposal = parseQueryPlan(serialized), revision = state.planRevision, submittedInputs = state.authorInputs;
            const basis = { ...state.planBasis! }, token = {};
            state.pendingEdit = token;
            apply.disabled = true;
            try {
                await this.edit(association, { plan: proposal }, "input.query.builder", basis);
                // A completed original operation cannot settle a newer proposal or later raw input.
                if (state.planDraft === plan && state.planRevision === revision && JSON.stringify(plan) === serialized && state.authorInputs === submittedInputs) {
                    submittedInputs?.retire();
                    state.authorInputs = undefined; state.planDraft = undefined; state.planBasis = undefined; state.builderOpen = false;
                }
            } finally {
                if (state.pendingEdit === token) state.pendingEdit = undefined;
                for (const [currentMount, currentAssociation] of this.mounts)
                    if (currentAssociation.occurrence?.id === item.id) this.renderStatusOnly(currentMount, currentAssociation);
            }
        }, this.host);
        apply.dataset.queryEdit = "builderApply";
        apply.disabled = !!state.pendingEdit;
        if (!sameQueryBasis(state.planBasis, currentBasis)) {
            apply.disabled = true;
            builder.append(element("p", "embedded-query-help", message("embeddedQuery.builderSessionChanged")));
            // A fresh grant/session never silently adopts an old proposal. A
            // deliberate recheck is possible only against the exact same bytes
            // and retained source generation, after fresh discovery qualified it.
            if (state.planBasis?.source === currentBasis.source && state.planBasis.serial === currentBasis.serial) {
                const recheck = action(message("embeddedQuery.recheckBuilder"), () => {
                    const fresh = this.host.basis();
                    if (!this.current(association) || !this.canWrite(association) || !state.planBasis || state.planBasis.source !== fresh.source || state.planBasis.serial !== fresh.serial)
                        throw new UiError(message("embeddedQuery.editBasisChanged"));
                    state.planBasis = { ...fresh };
                    this.refresh(item);
                }, this.host);
                builder.append(recheck);
            }
        }
        builder.onsubmit = event => { event.preventDefault(); apply.click(); };
        builder.append(apply, action(message("embeddedQuery.discardBuilder"), () => { state.authorInputs?.retire(); state.authorInputs = undefined; state.planDraft = undefined; state.planBasis = undefined; state.builderOpen = false; this.refresh(item); }, this.host));
        mount.append(builder);
    }
    private confirmRemoval(association: EmbeddedQueryAssociation): void {
        const item = association.occurrence;
        if (!item)
            return;
        const prior = document.activeElement instanceof HTMLElement ? document.activeElement : null, dialog = element("dialog", "embedded-query-dialog");
        setUiAttr(dialog, "aria-label", message("embeddedQuery.removeHere"));
        dialog.append(element("h2", "", message("embeddedQuery.removeHere")), element("p", "embedded-query-help", item.kind === "call" ? message("embeddedQuery.removeCallHelp") : message("embeddedQuery.removeDefinitionHelp")), action(message("common.cancel"), () => dialog.close(), this.host), action(message("embeddedQuery.confirmRemoval"), async () => { await this.arrange(association, "remove"); dialog.close(); }, this.host));
        dialog.addEventListener("close", () => {
            dialog.remove();
            if (prior?.isConnected)
                prior.focus({ preventScroll: true });
            if (this.dialog === dialog)
                this.dialog = null;
        }, { once: true });
        this.dialog?.close();
        this.dialog = dialog;
        document.body.append(dialog);
        dialog.showModal();
    }
    private async arrange(association: EmbeddedQueryAssociation, command: "up" | "down" | "remove"): Promise<void> {
        const item = association.occurrence;
        if (!item || !this.current(association) || !this.canWrite(association) || this.host.mode() === "read")
            throw new UiError(message("embeddedQuery.arrangementWriteUnavailable"));
        const basis = { ...this.host.basis() }, occurrences = [...this.occurrences], hash = this.discovery?.sourceHash;
        await this.host.prepareEdit();
        if (!sameQueryBasis(basis, this.host.basis()))
            throw new UiError(message("embeddedQuery.arrangementBasisChanged"));
        const result = await this.host.request<EmbeddedQueryEdit>("/api/query/embedded/edit", { ...basis, occurrenceId: item.id, action: command });
        if (!sameQueryBasis(basis, this.host.basis()) || !this.canWrite(association) || this.host.mode() === "read")
            throw new UiError(message("embeddedQuery.arrangementEligibilityChanged"));
        const patches = validateQueryArrangement(basis, item, occurrences, command, result, hash);
        this.pendingFocus = command === "remove" ? occurrences.find(q => q.id !== item.id)?.label ?? null : item.label;
        await this.host.apply(patches, basis, `input.query.${command}`);
    }
    private async edit(association: EmbeddedQueryAssociation, proposal: {
        plan?: QueryPlan;
        replacement?: string;
    }, event: string, expectedBasis?: EmbeddedQueryBasis): Promise<void> {
        const item = association.occurrence;
        if (!item || !this.current(association) || !this.canWrite(association) || this.host.mode() === "read")
            throw new UiError(message("embeddedQuery.editSourceUnavailable"));
        const basis = expectedBasis ?? { ...this.host.basis() };
        await this.host.prepareEdit();
        if (!sameQueryBasis(this.host.basis(), basis))
            throw new UiError(message("embeddedQuery.editBasisChanged"));
        const result = await this.host.request<EmbeddedQueryEdit>("/api/query/embedded/edit", { ...basis, occurrenceId: item.id, ...proposal });
        if (!sameQueryBasis(this.host.basis(), basis) || this.host.mode() === "read" || !this.canWrite(association))
            throw new UiError(message("embeddedQuery.editEligibilityChanged"));
        const patches = validateEmbeddedEdit(basis, item, result, this.discovery?.sourceHash);
        if (patches.length)
            await this.host.apply(patches, basis, event);
    }
    private async editSource(association: EmbeddedQueryAssociation): Promise<void> {
        const item = association.occurrence;
        if (!item || !this.current(association) || !this.canWrite(association) || this.host.mode() !== "write")
            return;
        const basis = { ...this.host.basis() };
        this.dialog?.close();
        const dialog = element("dialog", "embedded-query-dialog"), form = element("form"), input = element("textarea", "embedded-query-input embedded-query-source-proposal"), feedback = element("p", "embedded-query-warning");
        input.value = item.range.raw;
        const inputs = this.host.authorInputs?.();
        inputs?.bind(input, `query-source:${item.id}`);
        input.rows = 14;
        input.spellcheck = false;
        setUiAttr(input, "aria-label", displayMessage("embeddedQuery.sourceProposalLabel", { p0: uiDisplayParameter(item.label) }));
        form.append(element("h2", "", displayMessage("embeddedQuery.sourceDialogTitle", { p0: uiDisplayParameter(item.label) })), element("p", "embedded-query-help", message("embeddedQuery.sourceDialogHelp")), input, feedback);
        const buttons = element("div", "embedded-query-heading"), confirm = action(message("embeddedQuery.confirmSourceProposal"), async () => {
            confirm.disabled = true;
            try {
                const replacement = textareaSourceDraft(item.range.raw, input.value);
                if (replacement !== item.range.raw)
                    await this.edit(association, { replacement }, "input.query.source", basis);
                dialog.close();
            }
            catch (error) {
                setUiText(feedback, error instanceof Error ? errorUiText(error) : String(error));
            }
            finally {
                confirm.disabled = false;
            }
        }, this.host);
        confirm.dataset.queryEdit = "source";
        buttons.append(action(message("embeddedQuery.cancelSourceProposal"), () => dialog.close(), this.host), confirm);
        form.append(buttons);
        form.onsubmit = event => { event.preventDefault(); confirm.click(); };
        dialog.append(form);
        document.body.append(dialog);
        this.dialog = dialog;
        dialog.addEventListener("close", () => {
            inputs?.retire();
            dialog.remove();
            if (this.dialog === dialog)
                this.dialog = null;
        }, { once: true });
        dialog.showModal();
        input.focus();
    }
}
