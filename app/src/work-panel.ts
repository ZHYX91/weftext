import { LocaleScope, displayMessage, errorUiText, message, type UiText } from "./ui-locale";
import {
    WorkPanelPreferenceStore, clampWorkPanelHeight, defaultWorkPanelPreference, keyboardWorkPanelHeight,
    workPanelHeightBounds, workPanelTabs, workTaskNeedsAttention, workTaskProgress,
    type WorkPanelPreference, type WorkPanelStorage, type WorkPanelTab, type WorkTaskStatus,
} from "./work-panel-state";

export type { WorkPanelTab, WorkTaskStatus } from "./work-panel-state";
export interface WorkAction {
    /** Stable within the row; callers can keep a focused action across updates. */
    id?: string;
    label: UiText;
    run(): void | Promise<void>;
    disabled?: boolean;
}
interface WorkEntry {
    id: string;
    title: UiText;
    detail?: UiText;
    actions?: readonly WorkAction[];
}
export interface WorkProblem extends WorkEntry { severity: "error" | "warning" | "info"; }
export interface WorkTask extends WorkEntry { status: WorkTaskStatus; progress?: number; }
export interface WorkResult extends WorkEntry {}
type Item = WorkProblem | WorkTask | WorkResult;
interface Row {
    element: HTMLLIElement;
    scope: LocaleScope;
    item: Item;
    detail: HTMLElement;
    status: HTMLElement;
    progress: HTMLProgressElement;
    actions: HTMLElement;
    buttons: Map<string, { button: HTMLButtonElement; scope: LocaleScope; action: WorkAction }>;
}
interface TabView {
    button: HTMLButtonElement;
    panel: HTMLElement;
    list: HTMLUListElement;
    empty: HTMLElement;
    rows: Map<string, Row>;
}
export interface WorkPanelOptions {
    storage?: WorkPanelStorage | null;
    notify?: (text: UiText, error?: boolean) => void;
}
let nextPanel = 0;

/** A view of facts supplied by the existing task/author owners. It neither
 * retries a task nor owns a second persistence or close/recovery protocol.
 * Import work-panel.css in the entry point. Automatic updates never open it. */
export class WorkPanel {
    readonly element = document.createElement("section");
    readonly statusButton = document.createElement("button");
    private readonly scope = new LocaleScope();
    private readonly lifetime = new AbortController();
    private readonly tablist = document.createElement("div");
    private readonly collapse = document.createElement("button");
    private readonly resize = document.createElement("div");
    private readonly tabs = new Map<WorkPanelTab, TabView>();
    private readonly problems = new Map<string, readonly WorkProblem[]>();
    private readonly tasks = new Map<string, WorkTask>();
    private readonly results = new Map<string, WorkResult>();
    private readonly preferences: WorkPanelPreferenceStore;
    private preference = defaultWorkPanelPreference();
    private context: string | null = null;
    private problemScope: string | null = null;
    private disposed = false;
    private drag: { pointer: number; startY: number; height: number; original: WorkPanelPreference } | null = null;

    constructor(private readonly options: WorkPanelOptions = {}) {
        const id = `weftext-work-panel-${++nextPanel}`;
        this.element.id = id;
        this.element.className = "work-panel";
        this.element.setAttribute("role", "region");
        this.scope.bindAttr(this.element, "aria-label", message("workPanel.label"));
        this.statusButton.type = "button";
        this.statusButton.className = "work-panel-status-button";
        this.statusButton.setAttribute("aria-controls", id);
        this.scope.bindText(this.statusButton, () => displayMessage("workPanel.status", { p0: this.problemEntries().length, p1: this.attentionCount(), p2: this.results.size }));
        this.scope.bindAttr(this.statusButton, "title", () => message(this.preference.collapsed ? "workPanel.expand" : "workPanel.collapse"));
        this.statusButton.onclick = () => this.setCollapsed(!this.preference.collapsed);
        const header = document.createElement("header");
        header.className = "work-panel-header";
        this.tablist.className = "work-panel-tabs";
        this.tablist.setAttribute("role", "tablist");
        this.scope.bindAttr(this.tablist, "aria-label", message("workPanel.label"));
        this.collapse.type = "button";
        this.collapse.className = "work-panel-collapse";
        this.collapse.textContent = "⌄";
        this.scope.bindAttr(this.collapse, "aria-label", message("workPanel.collapse"));
        this.scope.bindAttr(this.collapse, "title", message("workPanel.collapse"));
        this.collapse.onclick = () => this.setCollapsed(true);
        header.append(this.tablist, this.collapse);
        this.resize.className = "work-panel-resizer";
        this.resize.tabIndex = 0;
        this.resize.setAttribute("role", "separator");
        this.resize.setAttribute("aria-orientation", "horizontal");
        this.resize.setAttribute("aria-controls", id);
        this.scope.bindAttr(this.resize, "aria-label", message("workPanel.resize"));
        this.scope.bindAttr(this.resize, "aria-valuetext", () => displayMessage("workPanel.height", { p0: this.actualHeight() }));
        const hint = document.createElement("span");
        hint.className = "work-panel-sr-only";
        hint.id = `${id}-resize-hint`;
        this.scope.bindText(hint, message("workPanel.resizeHint"));
        this.resize.setAttribute("aria-describedby", hint.id);
        this.element.append(this.resize, hint, header);
        for (const tab of workPanelTabs) {
            const button = document.createElement("button"), panel = document.createElement("div"), list = document.createElement("ul"), empty = document.createElement("p");
            button.type = "button";
            button.id = `${id}-tab-${tab}`;
            button.className = "work-panel-tab";
            button.dataset.tab = tab;
            button.setAttribute("role", "tab");
            button.setAttribute("aria-controls", `${id}-${tab}`);
            this.scope.bindText(button, () => displayMessage("workPanel.tabCount", { p0: message(`workPanel.${tab}`), p1: this.count(tab) }));
            button.onclick = () => this.reveal(tab);
            button.onkeydown = event => this.tabKeydown(event, tab);
            panel.id = `${id}-${tab}`;
            panel.className = "work-panel-content";
            panel.setAttribute("role", "tabpanel");
            panel.setAttribute("aria-labelledby", button.id);
            panel.tabIndex = 0;
            list.className = "work-panel-list";
            empty.className = "work-panel-empty";
            this.scope.bindText(empty, message(`workPanel.${tab}Empty`));
            panel.append(empty, list);
            this.tabs.set(tab, { button, panel, list, empty, rows: new Map() });
            this.tablist.append(button);
            this.element.append(panel);
        }
        let storage = options.storage ?? null;
        if (options.storage === undefined) {
            try { storage = document.defaultView?.localStorage ?? null; }
            catch { options.notify?.(message("workPanel.preferenceUnavailable")); }
        }
        this.preferences = new WorkPanelPreferenceStore(storage, () => options.notify?.(message("workPanel.preferenceUnavailable")));
        const window = this.element.ownerDocument.defaultView;
        this.resize.addEventListener("pointerdown", event => {
            if (event.button !== 0 || this.drag) return;
            event.preventDefault();
            this.drag = { pointer: event.pointerId, startY: event.clientY, height: this.actualHeight(), original: { ...this.preference } };
            this.resize.setPointerCapture(event.pointerId);
            this.resize.focus();
        }, { signal: this.lifetime.signal });
        this.resize.addEventListener("pointermove", event => {
            if (!this.drag || this.drag.pointer !== event.pointerId) return;
            this.preference.height = clampWorkPanelHeight(this.drag.height + this.drag.startY - event.clientY, this.viewportHeight());
            this.paint();
        }, { signal: this.lifetime.signal });
        this.resize.addEventListener("pointerup", event => {
            if (!this.drag || this.drag.pointer !== event.pointerId) return;
            this.drag = null;
            this.resize.releasePointerCapture(event.pointerId);
            this.save();
        }, { signal: this.lifetime.signal });
        const cancelResize = () => {
            if (!this.drag) return;
            const { pointer, original } = this.drag;
            this.drag = null;
            this.preference = original;
            if (this.resize.hasPointerCapture(pointer)) this.resize.releasePointerCapture(pointer);
            this.paint();
        };
        this.resize.addEventListener("pointercancel", cancelResize, { signal: this.lifetime.signal });
        this.resize.addEventListener("lostpointercapture", cancelResize, { signal: this.lifetime.signal });
        this.resize.addEventListener("keydown", event => {
            if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || event.ctrlKey || event.altKey || event.metaKey) return;
            if (event.key === "Escape" && this.drag) { event.preventDefault(); cancelResize(); return; }
            const height = keyboardWorkPanelHeight(this.actualHeight(), event.key, event.shiftKey, this.viewportHeight());
            if (height === null) return;
            event.preventDefault();
            this.preference.height = height;
            this.save(); this.paint();
        }, { signal: this.lifetime.signal });
        window?.addEventListener("resize", () => this.paint(), { signal: this.lifetime.signal });
        this.paint();
    }

    get collapsed(): boolean { return this.preference.collapsed; }
    get selected(): WorkPanelTab { return this.preference.selected; }
    setContext(context: string | null): void {
        this.assertMutable();
        if (this.context === context) return;
        this.cancelDrag();
        this.context = context;
        this.preference = this.preferences.read(context);
        const focusWasInside = this.preference.collapsed && this.element.contains(document.activeElement);
        this.paint();
        if (focusWasInside) this.statusButton.focus();
    }
    setProblems(scope: string, entries: readonly WorkProblem[]): void {
        this.assertMutable();
        this.assertEntries(entries);
        this.problems.set(scope, entries.map(entry => ({ ...entry, actions: entry.actions ? [...entry.actions] : undefined })));
        if (scope === this.problemScope) this.paint();
    }
    selectProblemScope(scope: string | null): void {
        this.assertMutable(); this.problemScope = scope; this.paint();
    }
    removeProblemScope(scope: string): void {
        this.assertMutable(); this.problems.delete(scope); if (scope === this.problemScope) this.paint();
    }
    reportTask(task: WorkTask): void {
        this.assertMutable(); this.assertEntries([task]);
        if (!["running", "waiting", "failed", "succeeded", "cancelled"].includes(task.status)) throw new Error("Unknown work task status");
        this.tasks.set(task.id, { ...task, progress: workTaskProgress(task.progress), actions: task.actions ? [...task.actions] : undefined });
        this.paint();
    }
    /** The original task identity and actions stay intact, including a waiting
     * original operation that has not yet produced a confirmed result. */
    finishTask(id: string, status: "succeeded" | "failed" | "waiting" | "cancelled", detail?: UiText): void {
        this.assertMutable();
        const task = this.tasks.get(id);
        if (!task) throw new Error("Cannot finish an unknown work task");
        this.reportTask({ ...task, status, detail: detail ?? task.detail, progress: status === "succeeded" ? 100 : undefined });
    }
    addResult(result: WorkResult): void {
        this.assertMutable(); this.assertEntries([result]);
        this.results.set(result.id, { ...result, actions: result.actions ? [...result.actions] : undefined });
        this.paint();
    }
    removeResult(id: string): void { this.assertMutable(); this.results.delete(id); this.paint(); }
    reveal(tab: WorkPanelTab, focus = false): void {
        this.assertMutable();
        if (!workPanelTabs.includes(tab)) throw new Error("Unknown work panel tab");
        const wasInside = this.element.contains(document.activeElement);
        this.preference.selected = tab;
        this.preference.collapsed = false;
        this.save(); this.paint();
        if (focus || wasInside) this.tabs.get(tab)!.button.focus();
    }
    setCollapsed(collapsed: boolean): void {
        this.assertMutable();
        const focusWasInside = collapsed && this.element.contains(document.activeElement);
        this.cancelDrag();
        this.preference.collapsed = collapsed;
        this.save(); this.paint();
        if (focusWasInside) this.statusButton.focus();
    }
    reset(): void {
        this.assertMutable(); this.cancelDrag(); this.preferences.reset(this.context);
        const focusWasInside = this.element.contains(document.activeElement);
        this.preference = defaultWorkPanelPreference(); this.paint();
        if (focusWasInside) this.statusButton.focus();
    }
    dispose(): void {
        if (this.disposed) return;
        this.disposed = true; this.cancelDrag(); this.lifetime.abort(); this.scope.dispose();
        for (const tab of this.tabs.values()) {
            tab.button.onclick = null; tab.button.onkeydown = null;
            for (const row of tab.rows.values()) this.disposeRow(row);
        }
        this.statusButton.onclick = null; this.collapse.onclick = null;
        this.element.remove(); this.statusButton.remove();
        this.problems.clear(); this.tasks.clear(); this.results.clear();
    }

    private assertMutable(): void { if (this.disposed) throw new Error("Work panel is disposed"); }
    private assertEntries(entries: readonly WorkEntry[]): void {
        const ids = new Set<string>();
        for (const entry of entries) {
            if (!entry.id || ids.has(entry.id)) throw new Error("Work entries require distinct stable identities");
            ids.add(entry.id);
            const actions = new Set<string>();
            for (const [index, action] of (entry.actions ?? []).entries()) {
                const key = action.id ?? String(index);
                if (actions.has(key)) throw new Error("Work actions require distinct row identities");
                actions.add(key);
            }
        }
    }
    private problemEntries(): readonly WorkProblem[] { return this.problemScope !== null ? this.problems.get(this.problemScope) ?? [] : []; }
    private attentionCount(): number { return [...this.tasks.values()].filter(task => workTaskNeedsAttention(task.status)).length; }
    private count(tab: WorkPanelTab): number { return tab === "problems" ? this.problemEntries().length : tab === "tasks" ? this.tasks.size : this.results.size; }
    private save(): void { this.preferences.write(this.context, this.preference); }
    private viewportHeight(): number { return this.element.ownerDocument.defaultView?.innerHeight ?? 600; }
    private actualHeight(): number { return clampWorkPanelHeight(this.preference.height, this.viewportHeight()); }
    private cancelDrag(): void {
        if (!this.drag) return;
        const { pointer, original } = this.drag;
        this.drag = null; this.preference = original;
        if (this.resize.hasPointerCapture(pointer)) this.resize.releasePointerCapture(pointer);
    }
    private paint(): void {
        if (this.disposed) return;
        this.element.hidden = this.preference.collapsed;
        this.element.inert = this.preference.collapsed;
        this.element.style.height = `${this.actualHeight()}px`;
        this.statusButton.setAttribute("aria-expanded", String(!this.preference.collapsed));
        this.statusButton.dataset.attention = String(this.problemEntries().some(problem => problem.severity === "error") || this.attentionCount() > 0);
        const bounds = workPanelHeightBounds(this.viewportHeight());
        this.resize.setAttribute("aria-valuemin", String(bounds.min));
        this.resize.setAttribute("aria-valuemax", String(bounds.max));
        this.resize.setAttribute("aria-valuenow", String(this.actualHeight()));
        for (const [tab, view] of this.tabs) {
            const selected = tab === this.preference.selected;
            view.button.setAttribute("aria-selected", String(selected)); view.button.tabIndex = selected ? 0 : -1;
            view.panel.hidden = !selected; view.panel.inert = !selected;
        }
        this.paintRows(this.tabs.get("problems")!, this.problemEntries());
        this.paintRows(this.tabs.get("tasks")!, [...this.tasks.values()]);
        this.paintRows(this.tabs.get("results")!, [...this.results.values()]);
        this.scope.refresh();
    }
    private paintRows(view: TabView, items: readonly Item[]): void {
        const ids = new Set(items.map(item => item.id));
        for (const [id, row] of view.rows) if (!ids.has(id)) { this.disposeRow(row); view.rows.delete(id); }
        for (const [index, item] of items.entries()) {
            let row = view.rows.get(item.id);
            if (!row) {
                row = this.makeRow(item); view.rows.set(item.id, row);
            }
            row.item = item;
            row.detail.hidden = item.detail === undefined;
            const state = "severity" in item ? item.severity : "status" in item ? item.status : null;
            row.status.hidden = state === null;
            row.element.dataset.state = state ?? "result";
            row.element.setAttribute("aria-busy", String("status" in item && item.status === "running"));
            const progress = "status" in item ? item.progress : undefined;
            row.progress.hidden = progress === undefined;
            if (progress !== undefined) row.progress.value = progress;
            this.paintActions(row, item.actions ?? []);
            row.scope.refresh();
            if (view.list.children[index] !== row.element) view.list.insertBefore(row.element, view.list.children[index] ?? null);
        }
        view.empty.hidden = items.length > 0;
    }
    private makeRow(item: Item): Row {
        const element = document.createElement("li"), scope = new LocaleScope(), text = document.createElement("div"), title = document.createElement("strong"), detail = document.createElement("p"), status = document.createElement("span"), progress = document.createElement("progress"), actions = document.createElement("div");
        element.className = "work-panel-item"; element.dataset.id = item.id;
        text.className = "work-panel-item-text"; detail.className = "work-panel-item-detail";
        status.className = "work-panel-item-state"; actions.className = "work-panel-item-actions"; progress.max = 100;
        const row: Row = { element, scope, item, detail, status, progress, actions, buttons: new Map() };
        scope.bindText(title, () => row.item.title);
        scope.bindText(detail, () => row.item.detail ?? "");
        scope.bindText(status, () => "severity" in row.item ? message(`workPanel.${row.item.severity}`) : "status" in row.item ? message(`workPanel.${row.item.status}`) : "");
        scope.bindAttr(progress, "aria-label", () => displayMessage("workPanel.progress", { p0: "status" in row.item ? row.item.progress ?? 0 : 0 }));
        text.append(title, detail); element.append(status, text, progress, actions);
        return row;
    }
    private paintActions(row: Row, actions: readonly WorkAction[]): void {
        const keys = new Set(actions.map((action, index) => action.id ?? String(index)));
        for (const [key, entry] of row.buttons) if (!keys.has(key)) { entry.scope.dispose(); entry.button.onclick = null; entry.button.remove(); row.buttons.delete(key); }
        for (const [index, action] of actions.entries()) {
            const key = action.id ?? String(index);
            let entry = row.buttons.get(key);
            if (!entry) {
                const button = document.createElement("button"), scope = new LocaleScope();
                button.type = "button";
                const created = { button, scope, action };
                entry = created;
                scope.bindText(button, () => created.action.label);
                button.onclick = () => {
                    const selected = created.action;
                    if (!selected.disabled) void Promise.resolve().then(() => selected.run()).catch(error => this.options.notify?.(errorUiText(error), true));
                };
                row.buttons.set(key, entry);
            }
            entry.action = action; entry.button.disabled = action.disabled ?? false; entry.scope.refresh();
            if (row.actions.children[index] !== entry.button) row.actions.insertBefore(entry.button, row.actions.children[index] ?? null);
        }
        row.actions.hidden = actions.length === 0;
    }
    private disposeRow(row: Row): void {
        row.scope.dispose();
        for (const entry of row.buttons.values()) { entry.scope.dispose(); entry.button.onclick = null; }
        row.element.remove();
    }
    private tabKeydown(event: KeyboardEvent, tab: WorkPanelTab): void {
        if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || event.altKey || event.ctrlKey || event.metaKey) return;
        const index = workPanelTabs.indexOf(tab);
        const next = event.key === "ArrowRight" ? workPanelTabs[(index + 1) % workPanelTabs.length]
            : event.key === "ArrowLeft" ? workPanelTabs[(index + workPanelTabs.length - 1) % workPanelTabs.length]
            : event.key === "Home" ? workPanelTabs[0] : event.key === "End" ? workPanelTabs.at(-1) : undefined;
        if (!next) return;
        event.preventDefault(); this.reveal(next, true);
    }
}
