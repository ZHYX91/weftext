import { uiDisplayParameter, displayMessage, message, setUiAttr, disposeUiTree, type UiText } from "./ui-locale";
import { action, ui } from "./data-panel";
import { graphCanvas, type CanvasState } from "./knowledge-graph-canvas";
import { defaultCamera, fitCamera, graphLayout } from "./relationship-graph-state";
import { DOCUMENT_ROOT, headingTree, visibleHeadings, type HeadingSnapshot, type StructureHeading } from "./heading-structure-state";
import "./heading-structure.css";

const kindLabels: Record<StructureHeading["kind"], UiText> = {
    heading: message("heading.itemKindHeading"), figure: message("heading.itemKindFigure"), table: message("heading.itemKindTable"),
    equation: message("heading.itemKindEquation"), code: message("heading.itemKindCode"), caption: message("heading.itemKindCaption"),
};

/** Read-only navigation state. The caller retains the authorized snapshot and jump guard. */
export class HeadingStructureView {
    private collapsed = new Set<string>();
    private selected = DOCUMENT_ROOT;
    private scope = DOCUMENT_ROOT;
    private search = "";
    private match = 0;
    private depth = 0;
    private mode: "list" | "map" = "list";
    private filter: "all" | "heading" | "caption" = "all";
    private canvas: CanvasState = { positions: new Map(), camera: defaultCamera() };
    private initialized = false;
    private optionsOpen = false;
    private element?: HTMLElement;
    private composing = false;
    private deferred?: { container: HTMLElement; snapshot: HeadingSnapshot; open: (heading: StructureHeading) => void };

    clear(): void {
        this.collapsed.clear(); this.selected = DOCUMENT_ROOT; this.scope = DOCUMENT_ROOT; this.search = ""; this.match = 0;
        this.initialized = false; this.canvas = { positions: new Map(), camera: defaultCamera() };
        this.composing = false; this.deferred = undefined;
        if (this.element) { disposeUiTree(this.element); this.element.remove(); this.element = undefined; }
    }

    render(container: HTMLElement, snapshot: HeadingSnapshot | undefined, open: (heading: StructureHeading) => void): void {
        if (this.composing && this.element?.isConnected && snapshot) {
            this.deferred = { container, snapshot, open };
            return;
        }
        this.composing = false;
        const previous = this.element, scroll = previous?.scrollTop ?? 0;
        const focused = previous?.contains(container.ownerDocument.activeElement) ? container.ownerDocument.activeElement as HTMLInputElement : null;
        const focusKey = focused?.dataset.outlineControl;
        const selection = focused?.tagName === "INPUT" && focused.type === "search" ? [focused.selectionStart, focused.selectionEnd] : null;
        const previousOptions = previous?.querySelector<HTMLDetailsElement>(".outline-options");
        if (previousOptions) this.optionsOpen = previousOptions.open;
        if (previous) { disposeUiTree(previous); previous.remove(); }
        const parent = ui("section", "knowledge-heading-structure outline-view");
        this.element = parent;
        container.append(parent);
        const redraw = () => this.render(container, snapshot, open);
        const control = (key: string, label: UiText, run: () => void): HTMLButtonElement => {
            const button = action(label, run);
            button.dataset.outlineControl = key;
            return button;
        };
        const restoreFocus = (): void => {
            if (!focusKey) return;
            const next = parent.querySelector<HTMLInputElement>(`[data-outline-control="${CSS.escape(focusKey)}"]`);
            next?.focus({ preventScroll: true });
            if (selection && next?.type === "search") next.setSelectionRange(selection[0], selection[1]);
        };
        const modes = ui("div", "outline-view-modes");
        modes.setAttribute("role", "group");
        setUiAttr(modes, "aria-label", message("heading.readOnlyStructure"));
        for (const [id, label] of [["list", message("heading.outlineList")], ["map", message("heading.mindMap")]] as const) {
            const button = control(id, label, () => { this.mode = id; redraw(); });
            button.setAttribute("aria-pressed", String(this.mode === id));
            modes.append(button);
        }
        parent.append(modes);
        if (!snapshot) {
            parent.append(ui("p", "knowledge-help", message("heading.refreshNativeHeadings")));
            restoreFocus();
            return;
        }
        const tree = headingTree(snapshot.items), byKey = new Map(tree.map(n => [n.key, n]));
        const choose = (key: string): void => {
            this.selected = key || DOCUMENT_ROOT;
            for (const row of parent.querySelectorAll<HTMLElement>("[data-heading-key]")) row.classList.toggle("is-selected", row.dataset.headingKey === this.selected);
            for (const row of parent.querySelectorAll<SVGElement>("[data-graph-key]")) row.classList.toggle("is-selected", row.dataset.graphKey === this.selected);
        };
        if (!byKey.has(this.scope)) this.scope = DOCUMENT_ROOT;
        const controls = ui("div", "outline-controls");
        const search = ui("input");
        search.type = "search"; search.value = this.search; search.dataset.outlineControl = "search";
        setUiAttr(search, "aria-label", message("heading.searchOutline"));
        setUiAttr(search, "placeholder", message("heading.outlineSearchPlaceholder"));
        search.oninput = event => { this.search = search.value; this.match = 0; if (!(event as InputEvent).isComposing) redraw(); };
        search.addEventListener("compositionstart", () => { this.composing = true; });
        search.addEventListener("compositionend", () => {
            this.composing = false; this.search = search.value; this.match = 0;
            const deferred = this.deferred; this.deferred = undefined;
            if (deferred) this.render(deferred.container, deferred.snapshot, deferred.open);
            else redraw();
        });
        search.onkeydown = event => {
            if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); this.search = search.value; this.match = 0; redraw(); }
        };
        controls.append(search);
        const filter = ui("select");
        filter.dataset.outlineControl = "filter";
        setUiAttr(filter, "aria-label", message("heading.headingsAndCaptions"));
        for (const [value, label] of [["all", message("heading.headingsAndCaptions")], ["heading", message("heading.headingsOnly")], ["caption", message("heading.captionsOnly")]] as const) {
            const option = ui("option", "", label); option.value = value; filter.append(option);
        }
        filter.value = this.filter;
        filter.disabled = this.mode === "map";
        filter.onchange = () => { this.filter = filter.value as typeof this.filter; redraw(); };
        controls.append(filter);
        parent.append(controls);
        const options = ui("details", "outline-options"), optionsBody = ui("div", "outline-options-body");
        options.open = this.optionsOpen;
        options.append(ui("summary", "", message("heading.displayDepth")), optionsBody);
        const depth = ui("input");
        depth.type = "number"; depth.min = "0"; depth.max = "4294967295"; depth.value = String(this.depth);
        depth.dataset.outlineControl = "depth";
        setUiAttr(depth, "aria-label", message("heading.displayDepth"));
        const applyDepth = (): void => {
            if (!depth.isConnected || depth.value === "") return;
            const value = Number(depth.value);
            if (Number.isSafeInteger(value) && value >= 0 && value <= 4294967295 && value !== this.depth) { this.depth = value; redraw(); }
        };
        depth.oninput = event => { if (!(event as InputEvent).isComposing) applyDepth(); };
        depth.onkeydown = event => { if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); applyDepth(); } };
        optionsBody.append(control("whole-document", message("heading.wholeDocument"), () => { this.scope = DOCUMENT_ROOT; redraw(); }), control("selected-subtree", message("heading.selectedSubtree"), () => {
            const selected = byKey.get(this.selected);
            if (selected) { this.scope = selected.kind === "heading" ? selected.key : selected.parent; redraw(); }
        }), depth, control("expand-all", message("heading.expandAll"), () => { this.collapsed.clear(); redraw(); }));
        parent.append(options);
        if (snapshot.coverage === "partial") parent.append(ui("p", "knowledge-warning", message("heading.unconfirmedCoverage")));
        if (snapshot.truncated) parent.append(ui("p", "knowledge-warning", message("heading.truncated")));
        const basis = this.mode === "map" ? headingTree(snapshot.items.filter(item => item.kind === "heading")) : tree;
        const visible = visibleHeadings(basis, this.scope, this.collapsed, this.search, this.depth);
        const shown = visible.items.filter(item => item.key === DOCUMENT_ROOT || this.mode === "map" || this.filter === "all" || (this.filter === "heading") === (item.kind === "heading"));
        const matches = visible.matches.filter(key => shown.some(item => item.key === key));
        const chooseMatch = (step: number) => {
            if (!matches.length) return;
            this.match = (this.match + step + matches.length) % matches.length;
            this.selected = matches[this.match]; redraw();
            this.element?.querySelector<HTMLElement>(`[data-heading-key="${CSS.escape(this.selected)}"]`)?.scrollIntoView({ block: "nearest" });
        };
        if (this.search) {
            const matchControls = ui("div", "outline-match-controls");
            matchControls.append(control("previous-match", message("heading.previousMatch"), () => chooseMatch(-1)), control("next-match", message("heading.nextMatch"), () => chooseMatch(1)), ui("span", "knowledge-help", displayMessage("heading.outlineMatchCount", { p0: matches.length })));
            parent.append(matchControls);
        }
        if (this.mode === "map") {
            const drawn = shown.slice(0, 200), ids = new Set(drawn.map(n => n.key));
            const edges = drawn.filter(n => ids.has(n.parent)).map(n => ({ key: `heading:${n.key}`, source: n.parent, target: n.key, direction: "directed" as const }));
            const positions = graphLayout(drawn, edges, "family", this.canvas.positions);
            for (const [key, point] of positions.positions) this.canvas.positions.set(key, point);
            if (!this.initialized && drawn.length) { this.initialized = true; this.canvas.camera = fitCamera([...positions.positions.values()], 700, 420); }
            if (drawn.length < shown.length) parent.append(ui("p", "knowledge-warning", displayMessage("heading.canvasLimit", { p0: drawn.length })));
            parent.append(graphCanvas(drawn.map(n => ({ key: n.key, title: n.key === DOCUMENT_ROOT ? message("heading.documentRoot") : `${n.number ? `${n.number} ` : ""}${n.text}` })), edges.map(e => ({ ...e, directed: true, label: displayMessage("heading.edgeLabel", { p0: uiDisplayParameter(e.source === DOCUMENT_ROOT ? message("heading.documentRootLabel") : byKey.get(e.source)?.text), p1: uiDisplayParameter(byKey.get(e.target)?.text) }) })), this.canvas,
                key => choose(key), key => { const item = byKey.get(key); if (item && item.key !== DOCUMENT_ROOT) open(item); }, this.selected, message("heading.layoutOnly")));
        }
        const list = ui("ul", "knowledge-heading-list outline-list");
        setUiAttr(list, "aria-label", message("heading.outlineList"));
        for (const item of shown) {
            if (item.key === DOCUMENT_ROOT) continue;
            const row = ui("li", `outline-row ${this.selected === item.key ? "is-selected" : ""}`);
            row.dataset.headingKey = item.key; row.dataset.outlineKind = item.kind;
            row.style.paddingInlineStart = `${Math.min(12, Math.max(0, item.depth - (byKey.get(this.scope)?.depth ?? 0) - 1)) * 12}px`;
            if (item.children.length) {
                const fold = control(`fold:${item.key}`, this.collapsed.has(item.key) ? "›" : "⌄", () => { this.collapsed.has(item.key) ? this.collapsed.delete(item.key) : this.collapsed.add(item.key); redraw(); });
                fold.classList.add("outline-fold"); fold.setAttribute("aria-expanded", String(!this.collapsed.has(item.key)));
                setUiAttr(fold, "aria-label", this.collapsed.has(item.key) ? message("heading.expandBranch") : message("heading.collapseBranch"));
                fold.disabled = !!this.search; row.append(fold);
            } else row.append(ui("span", "outline-fold-space"));
            const title = action(`${item.number ? `${item.number} ` : ""}${item.text}`, () => { choose(item.key); open(item); });
            title.classList.add("outline-jump"); title.dataset.outlineControl = `item:${item.key}`;
            setUiAttr(title, "aria-label", displayMessage("heading.outlineEntrySummary", { p0: uiDisplayParameter(`${item.number ? `${item.number} ` : ""}${item.text}`), p1: item.children.length, p2: item.descendants }));
            setUiAttr(title, "title", item.generated ? message("heading.generatedNoAuthorAddress") : message("heading.navigateExactNativeRange"));
            const kind = ui("span", "outline-item-kind", kindLabels[item.kind]);
            if (item.kind === "heading") kind.append(ui("span", "", String(item.level)));
            row.append(kind, title); list.append(row);
        }
        parent.append(list);
        if (!shown.some(item => item.key !== DOCUMENT_ROOT)) parent.append(ui("p", "knowledge-help", displayMessage("heading.outlineMatchCount", { p0: 0 })));
        parent.scrollTop = scroll;
        restoreFocus();
    }
}
