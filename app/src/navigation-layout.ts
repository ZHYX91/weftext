import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
export interface NavigationContext {
    root: string;
    workspaceId?: string;
    scope?: {
        kind: "library" | "directory" | "tag" | "query";
        key: string;
    };
}
export interface NavigationPreference {
    sidebarWidth: number;
    inspectorWidth: number;
    inspectorWideWidth: number;
    sidebarCollapsed: boolean;
    sidebarScroll: number;
}
export interface NavigationStorage {
    getItem(key: string): string | null;
    setItem(key: string, value: string): void;
    removeItem(key: string): void;
}
export const defaultNavigationPreference = (): NavigationPreference => ({ sidebarWidth: 264, inspectorWidth: 380, inspectorWideWidth: 680, sidebarCollapsed: false, sidebarScroll: 0 });
const numberWithin = (value: unknown, min: number, max: number, fallback: number): number => typeof value === "number" && Number.isFinite(value) && value >= min && value <= max ? Math.round(value) : fallback;
/** This projection stores geometry only: no result rows, author data, or writers. */
export function decodeNavigationPreference(value: unknown): NavigationPreference {
    const defaults = defaultNavigationPreference();
    if (!value || typeof value !== "object" || Array.isArray(value))
        return defaults;
    const item = value as Record<string, unknown>;
    return {
        sidebarWidth: numberWithin(item.sidebarWidth, 184, 480, defaults.sidebarWidth),
        inspectorWidth: numberWithin(item.inspectorWidth, 260, 960, defaults.inspectorWidth),
        inspectorWideWidth: numberWithin(item.inspectorWideWidth, 260, 960, defaults.inspectorWideWidth),
        sidebarCollapsed: typeof item.sidebarCollapsed === "boolean" ? item.sidebarCollapsed : false,
        sidebarScroll: numberWithin(item.sidebarScroll, 0, 10000000, 0),
    };
}
export function navigationContextKey(context: NavigationContext): string {
    const library = context.workspaceId ? ["workspace", context.workspaceId] : ["ordinary", context.root.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase()];
    return `weftext-navigation-layout:v1:${encodeURIComponent(JSON.stringify([library, context.scope?.kind ?? "library", context.scope?.key ?? ""]))}`;
}
export class NavigationPreferenceStore {
    private readonly memory = new Map<string, NavigationPreference>();
    private reported = false;
    constructor(private readonly storage: NavigationStorage | null, private readonly onUnavailable?: () => void) { }
    private unavailable(): void {
        if (!this.reported) {
            this.reported = true;
            this.onUnavailable?.();
        }
    }
    read(context: NavigationContext | null): NavigationPreference {
        if (!context)
            return defaultNavigationPreference();
        const key = navigationContextKey(context);
        const cached = this.memory.get(key);
        if (cached)
            return { ...cached };
        let result = defaultNavigationPreference();
        let raw: string | null | undefined;
        try {
            raw = this.storage?.getItem(key);
        }
        catch {
            this.unavailable();
        }
        if (raw && raw.length <= 4096) {
            try {
                result = decodeNavigationPreference(JSON.parse(raw));
            }
            catch { /* Corrupt display settings reset without changing authored content. */ }
        }
        this.memory.set(key, result);
        return { ...result };
    }
    write(context: NavigationContext | null, preference: NavigationPreference): void {
        if (!context)
            return;
        const key = navigationContextKey(context);
        const clean = decodeNavigationPreference(preference);
        this.memory.set(key, clean);
        try {
            this.storage?.setItem(key, JSON.stringify(clean));
        }
        catch {
            this.unavailable();
        }
    }
    reset(context: NavigationContext | null): void {
        if (!context)
            return;
        const key = navigationContextKey(context);
        this.memory.set(key, defaultNavigationPreference());
        try {
            this.storage?.removeItem(key);
        }
        catch {
            this.unavailable();
        }
    }
}
export interface ResizeBounds {
    min: number;
    max: number;
}
export function sidebarResizeBounds(viewportWidth: number, inspectorDocked: boolean): ResizeBounds {
    return { min: 184, max: Math.max(184, Math.min(480, viewportWidth - 380 - (inspectorDocked ? 260 : 0))) };
}
export function inspectorResizeBounds(viewportWidth: number, workspaceWidth: number): ResizeBounds {
    const max = Math.max(260, Math.min(960, viewportWidth * .62, workspaceWidth - 320));
    return { min: 260, max: Math.round(max) };
}
export const clampPaneWidth = (width: number, bounds: ResizeBounds): number => Math.round(Math.min(bounds.max, Math.max(bounds.min, width)));
export function keyboardPaneWidth(width: number, pane: "sidebar" | "inspector", key: string, shift: boolean, bounds: ResizeBounds, defaultWidth: number): number | null {
    if (key === "Home")
        return bounds.min;
    if (key === "End")
        return bounds.max;
    if (key === "Enter")
        return clampPaneWidth(defaultWidth, bounds);
    if (key !== "ArrowLeft" && key !== "ArrowRight")
        return null;
    const direction = (key === "ArrowRight" ? 1 : -1) * (pane === "sidebar" ? 1 : -1);
    return clampPaneWidth(width + direction * (shift ? 48 : 12), bounds);
}
export interface NavigationLayoutOptions {
    shell: HTMLElement;
    sidebar: HTMLElement;
    workspaceBody: HTMLElement;
    inspector: HTMLElement;
    toolbar?: HTMLElement;
    storage?: NavigationStorage | null;
    notify?: (message: UiText) => void;
}
export interface NavigationLayoutController {
    readonly controls: HTMLElement;
    setContext(context: NavigationContext | null): void;
    /** Call immediately before rebuilding the sidebar DOM. */
    captureSidebarState(): void;
    /** Call after rebuilding the sidebar DOM; never reloads a cached result. */
    restoreSidebarState(): void;
    toggleSidebar(): void;
    reset(): void;
    refresh(): void;
    destroy(): void;
}
/** Import navigation-layout.css once in the host; installation touches layout only. */
export function installNavigationLayout(options: NavigationLayoutOptions): NavigationLayoutController {
    const { shell, sidebar, workspaceBody, inspector } = options;
    const doc = shell.ownerDocument;
    const hostWindow = doc.defaultView;
    if (!hostWindow)
        throw new UiError("Navigation layout requires a document window");
    const view: Window = hostWindow;
    let storage = options.storage ?? null;
    if (options.storage === undefined) {
        try {
            storage = view.localStorage;
        }
        catch {
            options.notify?.(uiMessage("navigationLayout.preferenceNotPersistent"));
        }
    }
    const store = new NavigationPreferenceStore(storage, () => options.notify?.(uiMessage("navigationLayout.preferenceNotPersistent")));
    let context: NavigationContext | null = null;
    let preference = defaultNavigationPreference();
    let destroyed = false;
    let previousFocus: {
        id: string;
        owned: boolean;
    } | null = null;
    let drag: {
        pane: "sidebar" | "inspector";
        pointer: number;
        startX: number;
        startWidth: number;
        original: NavigationPreference;
    } | null = null;
    const lifetime = new AbortController();
    const priorSidebarId = sidebar.id, priorInspectorId = inspector.id;
    const priorInert = sidebar.hasAttribute("inert");
    const priorSidebarWidth = shell.style.getPropertyValue("--navigation-sidebar-width");
    const priorInspectorWidth = shell.style.getPropertyValue("--navigation-inspector-width");
    if (!sidebar.id)
        sidebar.id = "weftext-navigation-pane";
    if (!inspector.id)
        inspector.id = "weftext-inspector-pane";
    shell.classList.add("navigation-layout");
    workspaceBody.classList.add("navigation-workspace-body");
    const controls = doc.createElement("div");
    controls.className = "navigation-layout-controls";
    const toggle = doc.createElement("button");
    toggle.type = "button";
    toggle.className = "text-button navigation-layout-toggle";
    toggle.setAttribute("aria-controls", sidebar.id);
    const resetButton = doc.createElement("button");
    resetButton.type = "button";
    resetButton.className = "text-button";
    setUiText(resetButton, uiMessage("navigationLayout.restorePanelLayout"));
    controls.append(toggle, resetButton);
    options.toolbar?.append(controls);
    if (!options.toolbar) {
        controls.classList.add("navigation-layout-floating");
        shell.append(controls);
    }
    const hint = doc.createElement("span");
    hint.className = "navigation-layout-sr-only";
    hint.id = `${sidebar.id}-resize-instructions`;
    setUiText(hint, uiMessage("navigationLayout.resizeKeyboardHint"));
    controls.append(hint);
    function makeSeparator(label: UiText, controlled: string, className: string): HTMLElement {
        const handle = doc.createElement("div");
        handle.className = `navigation-resize-handle ${className}`;
        handle.tabIndex = 0;
        handle.setAttribute("role", "separator");
        handle.setAttribute("aria-orientation", "vertical");
        setUiAttr(handle, "aria-label", label);
        handle.setAttribute("aria-controls", controlled);
        handle.setAttribute("aria-describedby", hint.id);
        return handle;
    }
    const sidebarHandle = makeSeparator(uiMessage("navigationLayout.resizeLibraryNavigation"), sidebar.id, "navigation-sidebar-resizer");
    const inspectorHandle = makeSeparator(uiMessage("navigationLayout.resizeInspector"), inspector.id, "navigation-inspector-resizer");
    shell.insertBefore(sidebarHandle, sidebar.parentElement === shell ? sidebar.nextSibling : null);
    workspaceBody.insertBefore(inspectorHandle, inspector.parentElement === workspaceBody ? inspector : null);
    const wideSidebar = (): boolean => view.innerWidth > 780;
    const dockedInspector = (): boolean => view.innerWidth > 1000 && !inspector.hidden;
    function sidebarBounds(): ResizeBounds { return sidebarResizeBounds(shell.clientWidth || view.innerWidth, dockedInspector()); }
    function inspectorBounds(): ResizeBounds { return inspectorResizeBounds(shell.clientWidth || view.innerWidth, workspaceBody.clientWidth); }
    function inspectorPreferredWidth(): number { return inspector.classList.contains("wide") ? preference.inspectorWideWidth : preference.inspectorWidth; }
    function setPreferredWidth(pane: "sidebar" | "inspector", width: number): void {
        if (pane === "sidebar")
            preference.sidebarWidth = width;
        else if (inspector.classList.contains("wide"))
            preference.inspectorWideWidth = width;
        else
            preference.inspectorWidth = width;
    }
    function setAccessibleRange(handle: HTMLElement, width: number, bounds: ResizeBounds): void {
        handle.setAttribute("aria-valuemin", String(bounds.min));
        handle.setAttribute("aria-valuemax", String(bounds.max));
        handle.setAttribute("aria-valuenow", String(width));
        setUiAttr(handle, "aria-valuetext", displayMessage("navigationLayout.pixelWidth", { p0: uiDisplayParameter(width) }));
    }
    function refresh(): void {
        if (destroyed)
            return;
        const wide = wideSidebar();
        const collapsed = wide && preference.sidebarCollapsed;
        if (shell.classList.contains("navigation-sidebar-collapsed") !== collapsed)
            shell.classList.toggle("navigation-sidebar-collapsed", collapsed);
        sidebar.toggleAttribute("inert", priorInert || collapsed);
        const sidebarWidth = clampPaneWidth(preference.sidebarWidth, sidebarBounds());
        shell.style.setProperty("--navigation-sidebar-width", `${sidebarWidth}px`);
        const inspectorWidth = clampPaneWidth(inspectorPreferredWidth(), inspectorBounds());
        shell.style.setProperty("--navigation-inspector-width", `${inspectorWidth}px`);
        sidebarHandle.hidden = !wide || collapsed;
        inspectorHandle.hidden = !dockedInspector();
        setAccessibleRange(sidebarHandle, sidebarWidth, sidebarBounds());
        setAccessibleRange(inspectorHandle, inspectorWidth, inspectorBounds());
        const opened = wide ? !collapsed : shell.classList.contains("sidebar-open");
        setUiText(toggle, opened ? uiMessage("navigationLayout.collapseNavigation") : uiMessage("navigationLayout.showNavigation"));
        toggle.setAttribute("aria-expanded", String(opened));
    }
    function captureSidebarState(): void {
        const tree = sidebar.querySelector<HTMLElement>(".file-tree");
        if (tree)
            preference.sidebarScroll = tree.scrollTop;
        const focused = doc.activeElement as HTMLElement | null;
        previousFocus = focused && sidebar.contains(focused) ? { id: focused.id, owned: true } : null;
        store.write(context, preference);
    }
    function restoreSidebarState(): void {
        const tree = sidebar.querySelector<HTMLElement>(".file-tree");
        if (tree)
            tree.scrollTop = preference.sidebarScroll;
        // Only an explicit, surviving DOM ID can restore focus. A remembered title,
        // path, or list index never selects another object with the same name.
        if (previousFocus?.owned && previousFocus.id && (!doc.activeElement || doc.activeElement === doc.body)) {
            const target = doc.getElementById(previousFocus.id);
            if (target && sidebar.contains(target))
                target.focus({ preventScroll: true });
        }
        previousFocus = null;
    }
    function cancelDrag(): void {
        if (!drag)
            return;
        preference = drag.original;
        drag = null;
        doc.body.classList.remove("navigation-resizing");
        refresh();
    }
    function toggleSidebar(): void {
        const ownedFocus = sidebar.contains(doc.activeElement) || doc.activeElement === sidebarHandle;
        if (wideSidebar()) {
            preference.sidebarCollapsed = !preference.sidebarCollapsed;
            store.write(context, preference);
        }
        else
            shell.classList.toggle("sidebar-open");
        refresh();
        if (ownedFocus && (preference.sidebarCollapsed || !shell.classList.contains("sidebar-open")))
            toggle.focus({ preventScroll: true });
    }
    function reset(): void { cancelDrag(); store.reset(context); preference = defaultNavigationPreference(); refresh(); restoreSidebarState(); }
    function keyboardResize(pane: "sidebar" | "inspector", event: KeyboardEvent): void {
        if (event.isComposing || event.ctrlKey || event.metaKey || event.altKey)
            return;
        if (event.key === "Escape" && drag) {
            event.preventDefault();
            cancelDrag();
            return;
        }
        const bounds = pane === "sidebar" ? sidebarBounds() : inspectorBounds();
        const current = clampPaneWidth(pane === "sidebar" ? preference.sidebarWidth : inspectorPreferredWidth(), bounds);
        const defaults = defaultNavigationPreference();
        const next = keyboardPaneWidth(current, pane, event.key, event.shiftKey, bounds, pane === "sidebar" ? defaults.sidebarWidth : inspector.classList.contains("wide") ? defaults.inspectorWideWidth : defaults.inspectorWidth);
        if (next == null)
            return;
        event.preventDefault();
        setPreferredWidth(pane, next);
        store.write(context, preference);
        refresh();
    }
    function beginDrag(pane: "sidebar" | "inspector", event: PointerEvent): void {
        if (event.button !== 0 || !event.isPrimary)
            return;
        event.preventDefault();
        cancelDrag();
        const bounds = pane === "sidebar" ? sidebarBounds() : inspectorBounds();
        drag = { pane, pointer: event.pointerId, startX: event.clientX, startWidth: clampPaneWidth(pane === "sidebar" ? preference.sidebarWidth : inspectorPreferredWidth(), bounds), original: { ...preference } };
        doc.body.classList.add("navigation-resizing");
        try {
            (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
        }
        catch { /* Window listeners still complete or cancel the drag. */ }
    }
    sidebarHandle.addEventListener("keydown", event => keyboardResize("sidebar", event), { signal: lifetime.signal });
    inspectorHandle.addEventListener("keydown", event => keyboardResize("inspector", event), { signal: lifetime.signal });
    sidebarHandle.addEventListener("pointerdown", event => beginDrag("sidebar", event), { signal: lifetime.signal });
    inspectorHandle.addEventListener("pointerdown", event => beginDrag("inspector", event), { signal: lifetime.signal });
    view.addEventListener("pointermove", event => {
        if (!drag || event.pointerId !== drag.pointer)
            return;
        const bounds = drag.pane === "sidebar" ? sidebarBounds() : inspectorBounds();
        const direction = drag.pane === "sidebar" ? 1 : -1;
        setPreferredWidth(drag.pane, clampPaneWidth(drag.startWidth + (event.clientX - drag.startX) * direction, bounds));
        refresh();
    }, { signal: lifetime.signal });
    view.addEventListener("pointerup", event => {
        if (drag && event.pointerId === drag.pointer) {
            drag = null;
            doc.body.classList.remove("navigation-resizing");
            store.write(context, preference);
            refresh();
        }
    }, { signal: lifetime.signal });
    view.addEventListener("pointercancel", event => {
        if (drag && event.pointerId === drag.pointer)
            cancelDrag();
    }, { signal: lifetime.signal });
    view.addEventListener("keydown", event => {
        if (drag && event.key === "Escape") {
            event.preventDefault();
            cancelDrag();
        }
    }, { signal: lifetime.signal });
    view.addEventListener("blur", cancelDrag, { signal: lifetime.signal });
    view.addEventListener("resize", refresh, { signal: lifetime.signal });
    toggle.addEventListener("click", toggleSidebar, { signal: lifetime.signal });
    resetButton.addEventListener("click", reset, { signal: lifetime.signal });
    const observer = new MutationObserver(refresh);
    observer.observe(inspector, { attributes: true, attributeFilter: ["hidden", "class"] });
    observer.observe(shell, { attributes: true, attributeFilter: ["class"] });
    refresh();
    return {
        controls, refresh, captureSidebarState, restoreSidebarState, toggleSidebar, reset,
        setContext(next): void {
            if ((context && next && navigationContextKey(context) === navigationContextKey(next)) || (!context && !next)) {
                refresh();
                return;
            }
            cancelDrag();
            captureSidebarState();
            context = next;
            preference = store.read(context);
            previousFocus = null;
            refresh();
        },
        destroy(): void {
            if (destroyed)
                return;
            captureSidebarState();
            cancelDrag();
            destroyed = true;
            observer.disconnect();
            lifetime.abort();
            controls.remove();
            sidebarHandle.remove();
            inspectorHandle.remove();
            shell.classList.remove("navigation-layout", "navigation-sidebar-collapsed");
            workspaceBody.classList.remove("navigation-workspace-body");
            sidebar.toggleAttribute("inert", priorInert);
            if (!priorSidebarId)
                sidebar.removeAttribute("id");
            if (!priorInspectorId)
                inspector.removeAttribute("id");
            if (priorSidebarWidth)
                shell.style.setProperty("--navigation-sidebar-width", priorSidebarWidth);
            else
                shell.style.removeProperty("--navigation-sidebar-width");
            if (priorInspectorWidth)
                shell.style.setProperty("--navigation-inspector-width", priorInspectorWidth);
            else
                shell.style.removeProperty("--navigation-inspector-width");
        },
    };
}
