import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui } from "./data-panel";
import { graphCanvas, type CanvasState } from "./knowledge-graph-canvas";
import { defaultCamera, fitCamera, graphLayout } from "./relationship-graph-state";
import { DOCUMENT_ROOT, headingTree, visibleHeadings, type HeadingSnapshot, type StructureHeading } from "./heading-structure-state";
export class HeadingStructureView {
    private collapsed = new Set<string>();
    private selected = DOCUMENT_ROOT;
    private scope = DOCUMENT_ROOT;
    private search = "";
    private match = 0;
    private depth = 0;
    private canvas: CanvasState = { positions: new Map(), camera: defaultCamera() };
    private initialized = false;
    clear(): void { this.collapsed.clear(); this.selected = DOCUMENT_ROOT; this.scope = DOCUMENT_ROOT; this.search = ""; this.match = 0; this.initialized = false; this.canvas = { positions: new Map(), camera: defaultCamera() }; }
    render(container: HTMLElement, snapshot: HeadingSnapshot | undefined, open: (heading: StructureHeading) => void): void {
        const parent = ui("section", "knowledge-heading-structure");
        parent.append(ui("h3", "", message("heading.readOnlyStructure")));
        container.append(parent);
        if (!snapshot) {
            parent.append(ui("p", "knowledge-help", message("heading.refreshNativeHeadings")));
            return;
        }
        const tree = headingTree(snapshot.items), byKey = new Map(tree.map(n => [n.key, n])), redraw = () => { parent.remove(); this.render(container, snapshot, open); };
        if (!byKey.has(this.scope))
            this.scope = DOCUMENT_ROOT;
        const controls = ui("div", "knowledge-graph-controls");
        controls.append(action(message("heading.wholeDocument"), () => { this.scope = DOCUMENT_ROOT; redraw(); }), action(message("heading.selectedSubtree"), () => {
            if (byKey.has(this.selected)) {
                this.scope = this.selected;
                redraw();
            }
        }));
        const depth = ui("select");
        setUiAttr(depth, "aria-label", message("heading.displayDepth"));
        for (const [value, label] of [["0", message("heading.allLevels")], ["1", message("heading.oneLevel")], ["2", message("heading.twoLevels")], ["3", message("heading.threeLevels")], ["6", message("heading.sixLevels")]] as const) {
            const option = ui("option", "", label);
            option.value = value;
            depth.append(option);
        }
        depth.value = String(this.depth);
        depth.onchange = () => { this.depth = Number(depth.value); redraw(); };
        controls.append(depth, action(message("heading.expandAll"), () => { this.collapsed.clear(); redraw(); }));
        const search = ui("input");
        search.type = "search";
        search.value = this.search;
        setUiAttr(search, "aria-label", message("heading.searchHeadings"));
        setUiAttr(search, "placeholder", message("heading.searchPlaceholder"));
        const searchButton = action(message("heading.find"), () => { this.search = search.value; this.match = 0; redraw(); });
        search.onkeydown = event => {
            if (event.key === "Enter" && !event.isComposing) {
                event.preventDefault();
                this.search = search.value;
                this.match = 0;
                redraw();
            }
        };
        search.addEventListener("search", () => {
            if (!search.value) {
                this.search = "";
                redraw();
            }
        });
        const visible = visibleHeadings(tree, this.scope, this.collapsed, this.search, this.depth);
        const chooseMatch = (step: number) => {
            if (!visible.matches.length)
                return;
            this.match = (this.match + step + visible.matches.length) % visible.matches.length;
            this.selected = visible.matches[this.match];
            const point = this.canvas.positions.get(this.selected);
            if (point) {
                this.canvas.camera.x = 350 - point.x * this.canvas.camera.scale;
                this.canvas.camera.y = 210 - point.y * this.canvas.camera.scale;
            }
            redraw();
        };
        controls.append(search, searchButton, action(message("heading.previousMatch"), () => chooseMatch(-1)), action(message("heading.nextMatch"), () => chooseMatch(1)), ui("span", "knowledge-help", displayMessage("heading.matchCount", { p0: visible.matches.length })));
        parent.append(controls);
        parent.append(ui("p", "knowledge-help", displayMessage("heading.scopeSummary", { p0: uiDisplayParameter(this.scope === DOCUMENT_ROOT ? message("heading.wholeDocument") : displayMessage("heading.subtree", { p0: uiDisplayParameter(byKey.get(this.scope)!.text) })), p1: visible.items.length - (visible.items.some(n => n.key === DOCUMENT_ROOT) ? 1 : 0) })));
        if (snapshot.coverage === "partial")
            parent.append(ui("p", "knowledge-warning", message("heading.unconfirmedCoverage")));
        if (snapshot.truncated)
            parent.append(ui("p", "knowledge-warning", message("heading.truncated")));
        const drawn = visible.items.slice(0, 200), ids = new Set(drawn.map(n => n.key)), edges = drawn.filter(n => ids.has(n.parent)).map(n => ({ key: `heading:${n.key}`, source: n.parent, target: n.key, direction: "directed" as const }));
        const positions = graphLayout(drawn, edges, "family", this.canvas.positions);
        for (const [key, point] of positions.positions)
            this.canvas.positions.set(key, point);
        if (!this.initialized && drawn.length) {
            this.initialized = true;
            this.canvas.camera = fitCamera([...positions.positions.values()], 700, 420);
        }
        if (drawn.length < visible.items.length)
            parent.append(ui("p", "knowledge-warning", displayMessage("heading.canvasLimit", { p0: drawn.length })));
        let detail: HTMLElement | undefined;
        const displaySelection = () => {
            if (!detail)
                return;
            detail.replaceChildren();
            const node = byKey.get(this.selected);
            if (!node)
                return;
            detail.append(ui("h4", "", node.key === DOCUMENT_ROOT ? message("heading.documentRootLabel") : `${node.number ? `${node.number} ` : ""}${node.text}`), ui("p", "knowledge-help", node.key === DOCUMENT_ROOT ? displayMessage("heading.rootDescendants", { p0: node.descendants }) : displayMessage("heading.levelAndDescendants", { p0: node.level, p1: node.children.length, p2: node.descendants })));
            if (node.key !== DOCUMENT_ROOT)
                detail.append(action(message("heading.goToHeading"), () => open(node)), ui("p", "knowledge-help", node.generated ? message("heading.generatedNoAuthorAddress") : message("heading.navigateExactNativeRange")));
        };
        const choose = (key: string) => {
            this.selected = key || DOCUMENT_ROOT;
            displaySelection();
            for (const element of parent.querySelectorAll<SVGElement>("[data-graph-key]"))
                element.classList.toggle("is-selected", element.dataset.graphKey === this.selected);
        };
        parent.append(graphCanvas(drawn.map(n => ({ key: n.key, title: n.key === DOCUMENT_ROOT ? message("heading.documentRoot") : `${n.number ? `${n.number} ` : ""}${n.text}` })), edges.map(e => ({ ...e, directed: true, label: displayMessage("heading.edgeLabel", { p0: uiDisplayParameter(e.source === DOCUMENT_ROOT ? message("heading.documentRootLabel") : byKey.get(e.source)?.text), p1: uiDisplayParameter(e.target === DOCUMENT_ROOT ? message("heading.documentRootLabel") : byKey.get(e.target)?.text) }) })), this.canvas, (key, edge) => {
            if (!edge)
                choose(key);
        }, key => {
            const node = byKey.get(key);
            if (node && node.key !== DOCUMENT_ROOT)
                open(node);
        }, this.selected, message("heading.layoutOnly")));
        detail = ui("div", "knowledge-card");
        displaySelection();
        parent.append(detail);
        const list = ui("ul", "knowledge-heading-list");
        for (const heading of visible.items) {
            const row = ui("li");
            row.dataset.headingKey = heading.key;
            row.style.paddingInlineStart = `${Math.min(12, heading.depth - (byKey.get(this.scope)?.depth ?? 0)) * 12}px`;
            const choose = action(displayMessage("heading.itemSummary", { p0: uiDisplayParameter(heading.number ? `${heading.number} ` : ""), p1: uiDisplayParameter(heading.key === DOCUMENT_ROOT ? message("heading.documentRootLabel") : heading.text), p2: heading.children.length, p3: heading.descendants }), () => { this.selected = heading.key; displaySelection(); });
            choose.ondblclick = () => {
                if (heading.key !== DOCUMENT_ROOT)
                    open(heading);
            };
            choose.onkeydown = event => {
                if (event.key === "Enter" && heading.key !== DOCUMENT_ROOT) {
                    event.preventDefault();
                    open(heading);
                }
            };
            row.append(choose);
            if (heading.children.length) {
                const collapse = action(this.collapsed.has(heading.key) ? message("heading.expandBranch") : message("heading.collapseBranch"), () => {
                    if (this.collapsed.has(heading.key))
                        this.collapsed.delete(heading.key);
                    else
                        this.collapsed.add(heading.key);
                    redraw();
                });
                collapse.setAttribute("aria-expanded", String(!this.collapsed.has(heading.key)));
                collapse.disabled = !!this.search;
                row.append(collapse);
            }
            if (heading.key !== DOCUMENT_ROOT)
                row.append(action(message("heading.goTo"), () => open(heading)));
            list.append(row);
        }
        parent.append(list);
    }
}
