import { disposeUiTree, message as uiMessage, setUiAttr, setUiText, type UiText } from "./ui-locale";
import { defaultDockState, decodeDockState, dockContextKey, hideDockView, moveDockView, resizeDockGroups, revealDockView, type DockGroup, type DockSide, type DockState, type DockViewDefinition } from "./workspace-dock-state";

export interface WorkspaceDockView {
    id: string;
    label: UiText;
    /** The controller retains ownership of this element, including its hidden state. */
    element: HTMLElement;
    side?: DockSide;
    group?: string;
    hidden?: boolean;
}
export interface WorkspaceDockOptions {
    document?: Document;
    storage?: Pick<Storage, "getItem" | "setItem" | "removeItem"> | null;
    notify?: (message: UiText) => void;
    /** Called after requested reveal or newly visible activation, never during registration, resize or paint. */
    onActivate?: (id: string) => void;
}
export interface DockMoveTarget { side: DockSide; groupId?: string; position?: "before" | "after" | "center" }
interface MountedView extends WorkspaceDockView { slot: HTMLElement }
interface MountedGroup { section: HTMLElement; header: HTMLElement; body: HTMLElement }
const sides: readonly DockSide[] = ["left", "right"];

/** Reparents presentation slots only. No view/controller or author session is rebuilt. */
export class WorkspaceDock {
    readonly left: HTMLElement;
    readonly right: HTMLElement;
    readonly controls: HTMLElement;
    private readonly document: Document;
    private readonly storage: WorkspaceDockOptions["storage"];
    private readonly views = new Map<string, MountedView>();
    private readonly groups = new Map<string, MountedGroup>();
    private readonly memory = new Map<string, DockState>();
    private readonly lifetime = new AbortController();
    private readonly empties = new Map<DockSide, HTMLElement>();
    private readonly edgeTargets = new Map<DockSide, HTMLElement>();
    private readonly parking: HTMLElement;
    private readonly compositionOwners = new Set<EventTarget>();
    private state: DockState = defaultDockState([]);
    private context: string | null = null;
    private persistenceReported = false;
    private draggedId: string | null = null;
    private menu: HTMLElement | null = null;
    private destroyed = false;
    private notifyingActivation = false;
    constructor(private readonly options: WorkspaceDockOptions = {}) {
        this.document = options.document ?? document;
        let storage = options.storage;
        if (storage === undefined) {
            try { storage = this.document.defaultView?.localStorage ?? null; }
            catch { this.reportPersistence(); storage = null; }
        }
        this.storage = storage;
        this.left = this.create("aside", "workspace-dock-side workspace-dock-left");
        this.right = this.create("aside", "workspace-dock-side workspace-dock-right");
        setUiAttr(this.left, "aria-label", uiMessage("workspaceDock.leftSidebar"));
        setUiAttr(this.right, "aria-label", uiMessage("workspaceDock.rightSidebar"));
        this.controls = this.create("div", "workspace-dock-controls");
        this.parking = this.create("div", "workspace-dock-parking"); this.parking.hidden = true; this.parking.inert = true;
        this.left.append(this.parking);
        for (const side of sides) {
            const host = this.side(side);
            const empty = this.create("p", "workspace-dock-empty");
            setUiText(empty, uiMessage("workspaceDock.emptySide"));
            this.empties.set(side, empty);
            host.append(empty);
            host.addEventListener("compositionstart", event => { if (event.target) this.compositionOwners.add(event.target); }, { signal: this.lifetime.signal });
            host.addEventListener("compositionend", event => { if (event.target) this.compositionOwners.delete(event.target); }, { signal: this.lifetime.signal });
            host.addEventListener("dragover", event => {
                if (!this.draggedId || event.defaultPrevented) return;
                event.preventDefault();
                host.dataset.drop = "center";
                if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
            }, { signal: this.lifetime.signal });
            host.addEventListener("dragleave", event => { if (!(event.relatedTarget instanceof Node) || !host.contains(event.relatedTarget)) delete host.dataset.drop; }, { signal: this.lifetime.signal });
            host.addEventListener("drop", event => {
                if (!this.draggedId || event.defaultPrevented) return;
                event.preventDefault();
                const id = this.draggedId; this.endDrag();
                this.move(id, { side });
            }, { signal: this.lifetime.signal });
        }
        this.document.addEventListener("pointerdown", event => {
            if (this.menu && event.target instanceof Node && !this.menu.contains(event.target)) this.closeMenu(false);
        }, { signal: this.lifetime.signal });
        this.render();
    }
    private create<K extends keyof HTMLElementTagNameMap>(tag: K, className: string): HTMLElementTagNameMap[K] {
        const result = this.document.createElement(tag); result.className = className; return result;
    }
    private side(side: DockSide): HTMLElement { return side === "left" ? this.left : this.right; }
    private definitions(): DockViewDefinition[] { return [...this.views.values()].map(view => ({ id: view.id, side: view.side ?? "left", group: view.group, hidden: view.hidden })); }
    private reportPersistence(): void {
        if (!this.persistenceReported) { this.persistenceReported = true; this.options.notify?.(uiMessage("navigationLayout.preferenceNotPersistent")); }
    }
    private available(): boolean {
        if (this.destroyed) return false;
        for (const owner of this.compositionOwners) if (!(owner instanceof Node) || !owner.isConnected) this.compositionOwners.delete(owner);
        if (!this.compositionOwners.size) return true;
        this.options.notify?.(uiMessage("workspaceDock.waitComposition"));
        return false;
    }
    register(view: WorkspaceDockView): void {
        if (this.views.has(view.id)) throw new Error(`Duplicate workspace view: ${view.id}`);
        if (!/^[a-zA-Z][\w-]{0,79}$/.test(view.id)) throw new Error("Invalid workspace view ID");
        const slot = this.create("section", "workspace-dock-view");
        slot.id = `workspace-view-${view.id}`;
        slot.dataset.view = view.id;
        slot.setAttribute("role", "tabpanel");
        setUiAttr(slot, "aria-label", view.label);
        slot.append(view.element);
        this.views.set(view.id, { ...view, slot });
        this.state = decodeDockState(this.state, this.definitions());
        if (this.context === null && view.side === "right" && !view.hidden) this.state.rightHidden = false;
        this.render();
    }
    /** A string identifies the library/workspace, never its authored content. */
    setContext(context: string | null): void {
        if (context === this.context || !this.available()) return;
        this.persist();
        this.context = context;
        let raw: unknown = this.memory.get(context ?? "");
        if (raw === undefined && context !== null) {
            try {
                const value = this.storage?.getItem(dockContextKey(context));
                if (value && value.length <= 32768) {
                    try { raw = JSON.parse(value); } catch { /* Presentation-only corruption uses the defaults. */ }
                }
            } catch { this.reportPersistence(); }
        }
        this.state = decodeDockState(raw, this.definitions());
        this.render();
    }
    getState(): DockState { return structuredClone(this.state); }
    private persist(): void {
        const state = decodeDockState(this.state, this.definitions());
        this.memory.set(this.context ?? "", state);
        if (this.context === null) return;
        try { this.storage?.setItem(dockContextKey(this.context), JSON.stringify(state)); }
        catch { this.reportPersistence(); }
    }
    private visibleActive(state: DockState): string[] {
        const active: string[] = [];
        for (const side of sides) {
            if (state[side === "left" ? "leftHidden" : "rightHidden"]) continue;
            const maximizedHere = state[side].some(group => group.id === state.maximized);
            for (const group of state[side]) if (!group.collapsed && (!maximizedHere || group.id === state.maximized)) active.push(group.active);
        }
        return active;
    }
    private notifyActivation(ids: string[]): void {
        if (this.notifyingActivation) return;
        this.notifyingActivation = true;
        try { for (const id of ids) if (this.visibleActive(this.state).includes(id)) this.options.onActivate?.(id); }
        finally { this.notifyingActivation = false; }
    }
    private update(state: DockState, activateChanged = false): void {
        const before = new Set(this.visibleActive(this.state));
        this.state = decodeDockState(state, this.definitions()); this.persist(); this.render();
        if (activateChanged) this.notifyActivation(this.visibleActive(this.state).filter(id => !before.has(id)));
    }
    reveal(id: string, options: { focus?: boolean; activate?: boolean } = {}): void {
        if (!this.views.has(id) || !this.available()) return;
        this.update(revealDockView(this.state, id, this.definitions()));
        if (options.activate !== false) this.notifyActivation([id]);
        if (options.focus) this.tab(id)?.focus({ preventScroll: true });
    }
    hide(id: string): void {
        if (!this.views.has(id) || !this.available()) return;
        const active = this.document.activeElement, location = this.find(id);
        const focused = this.views.get(id)!.slot.contains(active)
            || active instanceof HTMLElement && active.closest<HTMLElement>("[data-dock-tab]")?.dataset.dockTab === id
            || !!location && location.group.active === id && this.groups.get(location.group.id)?.header.contains(active) === true
            || this.menu?.dataset.dockView === id && this.menu.contains(active);
        this.update(hideDockView(this.state, id), true);
        if (focused) this.focusFirst();
    }
    move(id: string, target: DockMoveTarget): void {
        if (!this.views.has(id) || !this.available()) return;
        this.update(moveDockView(this.state, id, target), true);
        this.tab(id)?.focus({ preventScroll: true });
    }
    toggleSide(side: DockSide): void {
        if (!this.available()) return;
        const state = structuredClone(this.state), key = side === "left" ? "leftHidden" : "rightHidden";
        state[key] = !state[key];
        const focused = this.side(side).contains(this.document.activeElement);
        this.update(state, true);
        if (focused && state[key]) this.controls.querySelector<HTMLButtonElement>(`[data-toggle-side="${side}"]`)?.focus({ preventScroll: true });
    }
    reset(): void {
        if (!this.available()) return;
        this.update(defaultDockState(this.definitions()), true);
    }
    /** Recheck presentation without reloading views or a cached result. */
    refresh(): void { if (!this.destroyed) this.render(); }
    private tab(id: string): HTMLButtonElement | null { return this.side(this.find(id)?.side ?? "left").querySelector<HTMLButtonElement>(`[data-dock-tab="${id}"]`); }
    private find(id: string): { side: DockSide; group: DockGroup; index: number } | undefined {
        for (const side of sides) {
            const index = this.state[side].findIndex(group => group.views.includes(id));
            if (index >= 0) return { side, group: this.state[side][index], index };
        }
    }
    private focusFirst(): void {
        const choices = [...this.left.querySelectorAll<HTMLButtonElement>('[role="tab"][aria-selected="true"]'), ...this.right.querySelectorAll<HTMLButtonElement>('[role="tab"][aria-selected="true"]'), ...this.controls.querySelectorAll<HTMLButtonElement>("button")];
        choices.find(item => !item.closest("[hidden],[inert]") && item.getClientRects().length > 0)?.focus({ preventScroll: true });
    }
    private button(label: UiText, run: () => void, text?: string): HTMLButtonElement {
        const button = this.create("button", "workspace-dock-button"); button.type = "button";
        setUiAttr(button, "aria-label", label); setUiAttr(button, "title", label);
        if (text !== undefined) button.textContent = text; else setUiText(button, label);
        button.onclick = run; return button;
    }
    private render(): void {
        if (this.destroyed) return;
        this.closeMenu(false);
        const focused = this.document.activeElement instanceof HTMLElement ? this.document.activeElement : null;
        const focusView = focused?.closest<HTMLElement>("[data-dock-tab]")?.dataset.dockTab;
        const focusGroup = focused?.closest<HTMLElement>("[data-group]")?.dataset.group;
        const focusAction = focused?.dataset.dockAction;
        const focusSideToggle = focused?.dataset.toggleSide;
        const retained = new Set<string>();
        for (const side of sides) {
            const host = this.side(side);
            const sideHidden = side === "left" ? this.state.leftHidden : this.state.rightHidden;
            host.hidden = sideHidden;
            host.inert = host.hidden;
            host.querySelectorAll(".workspace-dock-divider").forEach(divider => { disposeUiTree(divider); divider.remove(); });
            let previous: HTMLElement | null = null;
            for (const group of this.state[side]) {
                retained.add(group.id);
                let mounted = this.groups.get(group.id);
                if (!mounted) {
                    const section = this.create("section", "workspace-dock-group"), header = this.create("header", "workspace-dock-header"), body = this.create("div", "workspace-dock-body");
                    section.dataset.group = group.id; section.append(header, body);
                    mounted = { section, header, body }; this.groups.set(group.id, mounted);
                }
                const { section, header, body } = mounted;
                section.id = `workspace-dock-group-${encodeURIComponent(group.id)}`;
                const maximized = this.state.maximized === group.id;
                const maximizedHere = this.state[side].some(item => item.id === this.state.maximized);
                section.hidden = !!this.state.maximized && maximizedHere && !maximized;
                section.inert = section.hidden;
                section.classList.toggle("is-collapsed", group.collapsed);
                section.classList.toggle("is-maximized", maximized);
                section.style.flexGrow = String(group.weight);
                disposeUiTree(header); header.replaceChildren();
                const tabs = this.create("div", "workspace-dock-tabs"); tabs.setAttribute("role", "tablist");
                setUiAttr(tabs, "aria-label", side === "left" ? uiMessage("workspaceDock.leftSidebar") : uiMessage("workspaceDock.rightSidebar"));
                for (const id of group.views) {
                    const view = this.views.get(id); if (!view) continue;
                    const tab = this.button(view.label, () => this.reveal(id, { focus: true }));
                    tab.className = "workspace-dock-tab"; setUiText(tab, view.label);
                    tab.id = `workspace-tab-${id}`;
                    tab.dataset.dockTab = id;
                    tab.draggable = true;
                    tab.setAttribute("role", "tab");
                    tab.setAttribute("aria-controls", view.slot.id);
                    tab.setAttribute("aria-selected", String(group.active === id));
                    tab.tabIndex = group.active === id ? 0 : -1;
                    view.slot.setAttribute("aria-labelledby", tab.id);
                    view.slot.hidden = group.active !== id || group.collapsed;
                    view.slot.inert = view.slot.hidden;
                    tab.onkeydown = event => {
                        if (event.isComposing) return;
                        const index = group.views.indexOf(id);
                        const next = event.key === "ArrowLeft" ? (index + group.views.length - 1) % group.views.length : event.key === "ArrowRight" ? (index + 1) % group.views.length : event.key === "Home" ? 0 : event.key === "End" ? group.views.length - 1 : -1;
                        if (next >= 0) { event.preventDefault(); this.reveal(group.views[next], { focus: true }); }
                        if (event.key === "ContextMenu" || event.shiftKey && event.key === "F10") { event.preventDefault(); this.openMenu(id, tab); }
                    };
                    tab.oncontextmenu = event => { event.preventDefault(); this.openMenu(id, tab); };
                    tab.ondragstart = event => {
                        if (!this.available() || !event.dataTransfer) { event.preventDefault(); return; }
                        this.draggedId = id; event.dataTransfer.effectAllowed = "move";
                        event.dataTransfer.setData("application/x-weftext-dock-view", id);
                        event.dataTransfer.setData("text/plain", id);
                        this.showEdgeTargets();
                    };
                    tab.ondragend = () => this.endDrag();
                    tabs.append(tab);
                    // Avoid reparenting an active input on ordinary refresh/tab selection.
                    if (view.slot.parentElement !== body) body.append(view.slot);
                }
                const actions = this.create("div", "workspace-dock-group-actions");
                const collapse = this.button(group.collapsed ? uiMessage("workspaceDock.expandGroup") : uiMessage("workspaceDock.collapseGroup"), () => {
                    if (!this.available()) return;
                    const state = structuredClone(this.state); const target = state[side].find(item => item.id === group.id)!;
                    target.collapsed = !target.collapsed; this.update(state, true);
                }, group.collapsed ? "⌄" : "⌃");
                collapse.dataset.dockAction = "collapse";
                collapse.setAttribute("aria-expanded", String(!group.collapsed));
                collapse.setAttribute("aria-controls", `workspace-group-body-${encodeURIComponent(group.id)}`);
                const maximize = this.button(maximized ? uiMessage("workspaceDock.restoreGroups") : uiMessage("workspaceDock.maximizeGroup"), () => {
                    if (!this.available()) return;
                    const state = structuredClone(this.state); state.maximized = maximized ? null : group.id;
                    state[side].find(item => item.id === group.id)!.collapsed = false; this.update(state, true);
                }, maximized ? "↙" : "↗");
                maximize.dataset.dockAction = "maximize";
                const more = this.button(uiMessage("workspaceDock.viewActions"), () => this.openMenu(group.active, more), "⋯");
                more.dataset.dockAction = "menu";
                more.setAttribute("aria-haspopup", "menu"); actions.append(collapse, maximize, more);
                header.append(tabs, actions); body.id = `workspace-group-body-${encodeURIComponent(group.id)}`;
                body.hidden = group.collapsed; body.inert = group.collapsed;
                section.ondragover = event => {
                    if (!this.draggedId) return;
                    event.preventDefault(); event.stopPropagation();
                    section.dataset.drop = this.dropPosition(section, event.clientY);
                    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
                };
                section.ondragleave = event => { if (!(event.relatedTarget instanceof Node) || !section.contains(event.relatedTarget)) delete section.dataset.drop; };
                section.ondrop = event => {
                    if (!this.draggedId) return;
                    event.preventDefault(); event.stopPropagation();
                    const id = this.draggedId, position = this.dropPosition(section, event.clientY);
                    this.endDrag(); this.move(id, { side, groupId: group.id, position });
                };
                // Existing group and view nodes stay attached whenever their position is unchanged.
                const expectedBefore: Element | null = previous ? previous.nextElementSibling : host.firstElementChild;
                if (expectedBefore !== section) host.insertBefore(section, expectedBefore ?? null);
                previous = section;
                const index = this.state[side].indexOf(group), next = this.state[side][index + 1];
                if (next && !group.collapsed && !next.collapsed && !maximizedHere) {
                    const divider = this.divider(side, index); host.insertBefore(divider, section.nextSibling); previous = divider;
                }
            }
            const empty = this.empties.get(side)!; empty.hidden = this.state[side].length > 0; host.append(empty);
        }
        for (const [id, mounted] of this.groups) if (!retained.has(id)) {
            // A view can be hidden without destroying its controller or pending input.
            for (const view of this.views.values()) if (view.slot.parentElement === mounted.body) { view.slot.hidden = true; view.slot.inert = true; this.parking.append(view.slot); }
            disposeUiTree(mounted.header); mounted.section.remove(); this.groups.delete(id);
        }
        for (const id of this.state.hidden) {
            const view = this.views.get(id); if (view) { view.slot.hidden = true; view.slot.inert = true; if (view.slot.parentElement !== this.parking) this.parking.append(view.slot); }
        }
        this.renderControls();
        if (focused?.isConnected && focused !== this.document.activeElement && !focused.closest("[hidden]")) focused.focus({ preventScroll: true });
        else if (focusView) this.tab(focusView)?.focus({ preventScroll: true });
        else if (focusGroup && focusAction) this.groups.get(focusGroup)?.header.querySelector<HTMLElement>(`[data-dock-action="${focusAction}"]`)?.focus({ preventScroll: true });
        else if (focusSideToggle) this.controls.querySelector<HTMLElement>(`[data-toggle-side="${focusSideToggle}"]`)?.focus({ preventScroll: true });
    }
    private renderControls(): void {
        disposeUiTree(this.controls); this.controls.replaceChildren();
        for (const side of sides) {
            const button = this.button(side === "left" ? uiMessage("workspaceDock.leftSidebar") : uiMessage("workspaceDock.rightSidebar"), () => this.toggleSide(side));
            button.dataset.toggleSide = side;
            button.setAttribute("aria-pressed", String(!(side === "left" ? this.state.leftHidden : this.state.rightHidden)));
            this.controls.append(button);
        }
        const views = this.create("details", "workspace-dock-view-menu"), summary = this.create("summary", "");
        setUiText(summary, uiMessage("workspaceDock.viewActions")); views.append(summary);
        const list = this.create("div", "workspace-dock-view-menu-list");
        for (const [id, view] of this.views) {
            const button = this.button(view.label, () => { this.reveal(id, { focus: true }); views.open = false; });
            button.dataset.showView = id; button.setAttribute("aria-pressed", String(!this.state.hidden.includes(id))); list.append(button);
        }
        views.append(list); this.controls.append(views, this.button(uiMessage("workspaceDock.resetLayout"), () => this.reset()));
    }
    private divider(side: DockSide, index: number): HTMLElement {
        const divider = this.create("div", "workspace-dock-divider"); divider.tabIndex = 0;
        divider.setAttribute("role", "separator"); divider.setAttribute("aria-orientation", "horizontal");
        setUiAttr(divider, "aria-label", uiMessage("workspaceDock.groupResize")); setUiAttr(divider, "title", uiMessage("workspaceDock.resizeHint"));
        const pair = this.state[side].slice(index, index + 2), proportion = pair[0].weight / (pair[0].weight + pair[1].weight);
        divider.dataset.resizeFirst = pair[0].id;
        divider.setAttribute("aria-controls", pair.map(group => `workspace-dock-group-${encodeURIComponent(group.id)}`).join(" "));
        setUiAttr(divider, "aria-description", uiMessage("workspaceDock.resizeHint"));
        divider.setAttribute("aria-valuemin", "10"); divider.setAttribute("aria-valuemax", "90"); divider.setAttribute("aria-valuenow", String(Math.round(proportion * 100)));
        divider.onkeydown = event => {
            if (event.isComposing || !["ArrowUp", "ArrowDown", "Enter"].includes(event.key)) return;
            if (!this.available()) return;
            event.preventDefault();
            const delta = event.key === "Enter" ? 0.5 - proportion : (event.key === "ArrowDown" ? 1 : -1) * (event.shiftKey ? .12 : .04);
            this.update(resizeDockGroups(this.state, side, index, delta));
            [...this.side(side).querySelectorAll<HTMLElement>(".workspace-dock-divider")].find(item => item.dataset.resizeFirst === pair[0].id)?.focus({ preventScroll: true });
        };
        divider.onpointerdown = event => {
            if (event.button !== 0 || !this.available()) return;
            event.preventDefault();
            const initial = structuredClone(this.state), startY = event.clientY;
            const first = this.groups.get(pair[0].id)!.section, second = this.groups.get(pair[1].id)!.section;
            const height = first.getBoundingClientRect().height + second.getBoundingClientRect().height;
            const controller = new AbortController();
            const pointer = event.pointerId;
            // Keep this separator attached during the gesture; rerender only on release.
            divider.setPointerCapture(pointer);
            const move = (next: PointerEvent) => {
                if (next.pointerId !== pointer) return;
                this.state = resizeDockGroups(initial, side, index, (next.clientY - startY) / Math.max(1, height));
                first.style.flexGrow = String(this.state[side][index].weight); second.style.flexGrow = String(this.state[side][index + 1].weight);
                const total = this.state[side][index].weight + this.state[side][index + 1].weight;
                divider.setAttribute("aria-valuenow", String(Math.round(this.state[side][index].weight / total * 100)));
            };
            const end = (next: PointerEvent) => {
                if (next.pointerId !== pointer) return;
                controller.abort();
                if (next.type === "pointercancel" || next.type === "lostpointercapture") this.state = initial;
                this.persist(); this.render();
            };
            divider.addEventListener("pointermove", move, { signal: controller.signal });
            divider.addEventListener("pointerup", end, { signal: controller.signal });
            divider.addEventListener("pointercancel", end, { signal: controller.signal });
            divider.addEventListener("lostpointercapture", end, { once: true, signal: controller.signal });
        };
        return divider;
    }
    private dropPosition(section: HTMLElement, clientY: number): "before" | "after" | "center" {
        const rect = section.getBoundingClientRect(), relative = clientY - rect.top;
        return relative < Math.min(48, rect.height * .22) ? "before" : relative > rect.height - Math.min(48, rect.height * .22) ? "after" : "center";
    }
    private endDrag(): void {
        this.draggedId = null;
        for (const target of this.edgeTargets.values()) { disposeUiTree(target); target.remove(); }
        this.edgeTargets.clear();
        for (const host of [this.left, this.right]) { delete host.dataset.drop; host.querySelectorAll<HTMLElement>("[data-drop]").forEach(item => { delete item.dataset.drop; }); }
    }
    private showEdgeTargets(): void {
        for (const side of sides) {
            const host = this.side(side), rect = host.getBoundingClientRect();
            if (!host.hidden && rect.width >= 40 && this.document.defaultView?.getComputedStyle(host).visibility !== "hidden") continue;
            const target = this.create("div", `workspace-dock-edge-target workspace-dock-edge-${side}`);
            target.dataset.edgeSide = side;
            setUiText(target, side === "left" ? uiMessage("workspaceDock.leftSidebar") : uiMessage("workspaceDock.rightSidebar"));
            target.setAttribute("aria-hidden", "true");
            target.ondragover = event => { if (!this.draggedId) return; event.preventDefault(); target.classList.add("is-over"); if (event.dataTransfer) event.dataTransfer.dropEffect = "move"; };
            target.ondragleave = () => target.classList.remove("is-over");
            target.ondrop = event => {
                if (!this.draggedId) return;
                event.preventDefault(); event.stopPropagation();
                const id = this.draggedId; this.endDrag(); this.move(id, { side });
            };
            this.edgeTargets.set(side, target); this.document.body.append(target);
        }
    }
    private openMenu(id: string, trigger: HTMLElement): void {
        const location = this.find(id); if (!location || !this.available()) return;
        this.closeMenu(false);
        const menu = this.create("div", "workspace-dock-menu"); menu.setAttribute("role", "menu");
        menu.dataset.dockView = id;
        setUiAttr(menu, "aria-label", uiMessage("workspaceDock.viewActions"));
        const items: { label: UiText; run: () => void }[] = [
            { label: uiMessage("workspaceDock.splitAbove"), run: () => this.move(id, { side: location.side, groupId: location.group.id, position: "before" }) },
            { label: uiMessage("workspaceDock.splitBelow"), run: () => this.move(id, { side: location.side, groupId: location.group.id, position: "after" }) },
        ];
        const previous = this.state[location.side][location.index - 1], next = this.state[location.side][location.index + 1];
        if (previous) items.push(
            { label: uiMessage("workspaceDock.previousGroup"), run: () => this.move(id, { side: location.side, groupId: previous.id, position: "before" }) },
            { label: uiMessage("workspaceDock.mergePrevious"), run: () => this.move(id, { side: location.side, groupId: previous.id, position: "center" }) },
        );
        if (next) items.push(
            { label: uiMessage("workspaceDock.nextGroup"), run: () => this.move(id, { side: location.side, groupId: next.id, position: "after" }) },
            { label: uiMessage("workspaceDock.mergeNext"), run: () => this.move(id, { side: location.side, groupId: next.id, position: "center" }) },
        );
        items.push(
            { label: uiMessage(location.side === "left" ? "workspaceDock.moveRight" : "workspaceDock.moveLeft"), run: () => this.move(id, { side: location.side === "left" ? "right" : "left" }) },
            { label: uiMessage("workspaceDock.hideView"), run: () => this.hide(id) },
        );
        for (const item of items) {
            const button = this.button(item.label, () => { item.run(); this.closeMenu(false); });
            button.setAttribute("role", "menuitem"); menu.append(button);
        }
        const buttons = () => [...menu.querySelectorAll<HTMLButtonElement>("button")];
        menu.onkeydown = event => {
            if (event.isComposing) return;
            if (event.key === "Escape") { event.preventDefault(); this.closeMenu(false); this.tab(id)?.focus({ preventScroll: true }); }
            const choices = buttons(), index = choices.indexOf(this.document.activeElement as HTMLButtonElement);
            const target = event.key === "ArrowDown" ? (index + 1) % choices.length : event.key === "ArrowUp" ? (index + choices.length - 1) % choices.length : event.key === "Home" ? 0 : event.key === "End" ? choices.length - 1 : -1;
            if (target >= 0) { event.preventDefault(); choices[target].focus(); }
            if (event.key === "Tab") this.closeMenu(false);
        };
        this.document.body.append(menu); this.menu = menu;
        const rect = trigger.getBoundingClientRect(), width = menu.getBoundingClientRect().width;
        menu.style.left = `${Math.max(4, Math.min(rect.left, (this.document.defaultView?.innerWidth ?? 1024) - width - 4))}px`;
        menu.style.top = `${Math.min(rect.bottom + 3, Math.max(4, (this.document.defaultView?.innerHeight ?? 768) - menu.getBoundingClientRect().height - 4))}px`;
        buttons()[0]?.focus();
    }
    private closeMenu(restore: boolean): void {
        if (!this.menu) return;
        disposeUiTree(this.menu); this.menu.remove(); this.menu = null;
        if (restore) this.focusFirst();
    }
    destroy(): void {
        if (this.destroyed) return;
        this.persist(); this.lifetime.abort(); this.closeMenu(false); this.endDrag(); this.destroyed = true;
        for (const group of this.groups.values()) disposeUiTree(group.header);
        disposeUiTree(this.controls);
        this.left.remove(); this.right.remove(); this.controls.remove(); this.groups.clear();
    }
}
