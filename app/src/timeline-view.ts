import { historyOwnerKey, historyTimeLabel, type HistoryContext, type HistoryList, type HistoryVersion } from "./history-state";
import { coreDescriptionUiText, decodeCoreDescription } from "./ui-diagnostics";
import { disposeUiTree, displayMessage, errorUiText, joinUiText, message, setUiAttr, setUiText, uiDisplayParameter, type UiText, UiError } from "./ui-locale";

export interface TimelineHost {
    getDocument(): HistoryContext | null;
    request<T>(path: string, body?: unknown): Promise<T>;
    /** The existing history owner retains comparison, restore and original-operation recovery. */
    openHistory(): Promise<void>;
    notify(message: UiText, error?: boolean): void;
}

const record = (value: unknown): Record<string, unknown> | undefined => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : undefined;
const text = (value: unknown): value is string => typeof value === "string" && value.length > 0;
const nullableText = (value: unknown): value is string | null => value === null || text(value);
const nullableNumber = (value: unknown): value is number | null => value === null || typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

/** Reject malformed or mis-scoped replies before they become sidebar rows. */
export function decodeTimelineList(value: unknown): HistoryList {
    const item = record(value), retention = record(item?.retention);
    if (!item || !text(item.nodeId) || !text(item.workspaceId) || !text(item.path) || !text(item.currentBaseId)
        || !nullableText(item.currentSavedVersionId) || !["saved", "external"].includes(String(item.currentness))
        || !Array.isArray(item.versions) || !Array.isArray(item.warnings) || !Array.isArray(item.protectedDrafts)
        || !retention || retention.policy !== "retainedCoreJournal" || retention.complete !== false
        || !nullableNumber(retention.knownVersions) || retention.knownVersions === null
        || !nullableNumber(retention.availableVersions) || retention.availableVersions === null
        || !nullableNumber(retention.oldestSavedAtUnixMs) || !nullableNumber(retention.newestSavedAtUnixMs))
        throw new UiError(message("error.invalidResponse"));
    const retentionDescription = decodeCoreDescription(retention.description);
    if (!retentionDescription) throw new UiError(message("error.invalidResponse"));
    const ids = new Set<string>();
    const versions = item.versions.map(value => {
        const version = record(value), description = decodeCoreDescription(version?.description);
        if (!version || !text(version.versionId) || ids.has(version.versionId) || !nullableText(version.revisionId)
            || !text(version.operationId) || !text(version.kind) || !nullableNumber(version.savedAtUnixMs)
            || !["commitClock", "unknown"].includes(String(version.timeBasis)) || !nullableNumber(version.commitSequence)
            || !["available", "missing", "unreadable", "invalid"].includes(String(version.status))
            || !nullableNumber(version.byteLength) || !nullableText(version.sha256) || !description)
            throw new UiError(message("error.invalidResponse"));
        ids.add(version.versionId);
        return { ...version, description } as unknown as HistoryVersion;
    });
    const warnings = item.warnings.map(value => {
        const warning = decodeCoreDescription(value);
        if (!warning) throw new UiError(message("error.invalidResponse"));
        return warning;
    });
    const protectedDrafts = item.protectedDrafts.map(value => {
        const draft = record(value);
        if (!draft || !text(draft.path) || typeof draft.baseSource !== "string" || typeof draft.source !== "string" || typeof draft.baseKnown !== "boolean")
            throw new UiError(message("error.invalidResponse"));
        return { path: draft.path, baseSource: draft.baseSource, source: draft.source, baseKnown: draft.baseKnown };
    });
    return { ...item, versions, warnings, protectedDrafts, retention: { ...retention, description: retentionDescription } } as unknown as HistoryList;
}

function contextKey(context: HistoryContext | null): string {
    return context?.nodeId && context.workspaceId ? JSON.stringify([context.sessionId, context.workspaceId, context.nodeId, context.path, context.baseId]) : "";
}

/** Read-only delivery gate. No Draft settlement, save, restore or source-read request is issued here. */
export class TimelineReadModel {
    context: HistoryContext | null = null;
    list: HistoryList | null = null;
    loading = false;
    error: UiText | null = null;
    private key: string | undefined;
    private generation = 0;
    private disposed = false;
    constructor(private readonly host: Pick<TimelineHost, "getDocument" | "request">, private readonly changed: () => void = () => {}) { }
    async update(force = false): Promise<void> {
        if (this.disposed) return;
        const current = this.host.getDocument(), key = contextKey(current);
        if (!force && key === this.key) return;
        const generation = ++this.generation;
        this.key = key;
        this.context = key && current ? { ...current } : null;
        this.list = null;
        this.error = null;
        this.loading = !!this.context;
        this.changed();
        if (!this.context) return;
        const captured = this.context;
        const accepts = (): boolean => !this.disposed && generation === this.generation && key === contextKey(this.host.getDocument());
        try {
            const response = await this.host.request<unknown>("/api/history/list", { sessionId: captured.sessionId });
            if (!accepts()) return;
            const list = decodeTimelineList(response);
            if (list.nodeId !== captured.nodeId || list.workspaceId !== captured.workspaceId)
                throw new UiError(message("historyState.noteIdentityMismatch"));
            // Core orders commits. Wall clocks can go backwards and never reorder them here.
            this.list = list;
        }
        catch (error) {
            if (accepts()) this.error = errorUiText(error);
        }
        finally {
            if (accepts()) {
                this.loading = false;
                this.changed();
            }
        }
    }
    dispose(): void { this.disposed = true; this.generation++; this.list = null; this.context = null; this.loading = false; this.error = null; }
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, label?: UiText): HTMLElementTagNameMap[K] {
    const value = document.createElement(tag);
    value.className = className;
    if (label !== undefined) setUiText(value, label);
    return value;
}
const availability: Record<HistoryVersion["status"], UiText> = {
    available: message("history.availableSource"), missing: message("history.missingSource"),
    unreadable: message("history.unreadableSource"), invalid: message("history.invalidRecord"),
};

/** A persistent sidebar surface; the existing HistoryPanel owns every write proposal. */
export class TimelineView {
    readonly element = element("section", "timeline-view");
    private readonly path = element("p", "timeline-path");
    private readonly status = element("p", "timeline-status");
    private readonly notices = element("div", "timeline-notices");
    private readonly rows = element("ol", "timeline-versions");
    private readonly refreshButton = element("button", "text-button", message("history.refresh"));
    private readonly detailsButton = element("button", "text-button", message("timeline.details"));
    private readonly model: TimelineReadModel;
    private readonly positions = new Map<string, { scroll: number; expanded: Set<string> }>();
    private positionKey = "";
    private disposed = false;
    constructor(private readonly host: TimelineHost) {
        setUiAttr(this.element, "aria-label", message("shell.timeline"));
        this.element.append(element("p", "timeline-scope", message("timeline.scope")), this.path);
        const toolbar = element("div", "timeline-toolbar");
        this.refreshButton.type = this.detailsButton.type = "button";
        this.refreshButton.onclick = () => { void this.refresh(); };
        this.detailsButton.onclick = () => {
            if (contextKey(this.model.context) !== contextKey(host.getDocument())) { void this.documentChanged(); return; }
            void host.openHistory().catch(error => host.notify(errorUiText(error), true));
        };
        toolbar.append(this.refreshButton, this.detailsButton);
        this.status.setAttribute("role", "status");
        this.status.setAttribute("aria-live", "polite");
        this.element.append(toolbar, this.status, this.notices, this.rows);
        this.model = new TimelineReadModel(host, () => this.draw());
        this.draw();
    }
    async documentChanged(): Promise<void> { this.capturePosition(); await this.model.update(); }
    async refresh(): Promise<void> { this.capturePosition(); await this.model.update(true); }
    private capturePosition(): void {
        if (!this.positionKey) return;
        const position = this.positions.get(this.positionKey);
        if (position) position.scroll = this.rows.scrollTop;
    }
    private draw(): void {
        if (this.disposed) return;
        const { context, list, error, loading } = this.model;
        const nextKey = context ? historyOwnerKey(context) : "";
        if (nextKey !== this.positionKey) this.positionKey = nextKey;
        if (nextKey && !this.positions.has(nextKey)) {
            // Display state is bounded and never serialized with author content.
            if (this.positions.size >= 64) this.positions.delete(this.positions.keys().next().value!);
            this.positions.set(nextKey, { scroll: 0, expanded: new Set() });
        }
        const position = this.positions.get(nextKey);
        setUiText(this.path, list?.path ?? context?.path ?? "");
        setUiAttr(this.path, "title", list?.path ?? context?.path ?? "");
        this.refreshButton.disabled = !context || loading;
        this.detailsButton.disabled = !context;
        setUiText(this.refreshButton, error ? message("timeline.retry") : message("history.refresh"));
        setUiText(this.status, !context ? message("history.admittedLocalNoteRequired") : loading ? message("history.reading") : error ?? "");
        this.status.classList.toggle("timeline-error", !!error);
        for (const child of [...this.notices.children, ...this.rows.children]) disposeUiTree(child);
        this.notices.replaceChildren();
        this.rows.replaceChildren();
        if (!list) return;
        this.notices.append(element("p", "timeline-help", coreDescriptionUiText(list.retention.description)));
        if (list.currentness === "external") this.notices.append(element("p", "timeline-warning", message("history.externalChangesHelp")));
        for (const warning of list.warnings) this.notices.append(element("p", "timeline-warning", coreDescriptionUiText(warning)));
        if (!list.versions.length) this.rows.append(element("li", "timeline-help", message("history.noRetainedRecords")));
        for (const version of list.versions) {
            const row = element("li", "timeline-version");
            const details = element("details", "timeline-version-details");
            details.dataset.historyVersion = version.versionId;
            details.open = position?.expanded.has(version.versionId) ?? false;
            details.ontoggle = () => {
                if (!position || !this.positions.has(nextKey)) return;
                details.open ? position.expanded.add(version.versionId) : position.expanded.delete(version.versionId);
            };
            const summary = element("summary", "timeline-version-summary", historyTimeLabel(version));
            const sequence = version.commitSequence === null ? message("history.commitOrderUnknown") : displayMessage("history.commitSequence", { p0: version.commitSequence });
            const metadata = joinUiText([sequence, availability[version.status], ...(version.versionId === list.currentSavedVersionId ? [message("history.currentSavedSuffix")] : [])], " · ");
            summary.append(element("span", "timeline-version-meta", metadata));
            details.append(summary, element("p", "timeline-help", coreDescriptionUiText(version.description)), element("p", "timeline-version-id", version.versionId));
            if (version.byteLength !== null) {
                details.append(element("p", "timeline-help", displayMessage("history.sourceMetadata", { p0: uiDisplayParameter(historyTimeLabel(version)), p1: uiDisplayParameter(version.byteLength), p2: version.versionId })));
            }
            row.append(details);
            this.rows.append(row);
        }
        this.rows.scrollTop = position?.scroll ?? 0;
    }
    dispose(): void { this.disposed = true; this.model.dispose(); disposeUiTree(this.element); this.element.remove(); this.positions.clear(); }
}
