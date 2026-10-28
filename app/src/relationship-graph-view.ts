import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui } from "./data-panel";
import { disposeGraphCanvasTree, graphCanvas, type CanvasState } from "./knowledge-graph-canvas";
import { disposeUiTree } from "./ui-locale";
import { defaultCamera, defaultGraphFilters, graphLayout, graphMatches, graphVisible, type GraphSnapshot, type GraphNode, type GraphEdge, type GraphFilters } from "./relationship-graph-state";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
export interface RelationshipGraphHost {
    open(node: GraphNode): void | Promise<void>;
    source(edge: GraphEdge, node: GraphNode): void | Promise<void>;
    error(error: unknown): void;
}
function select(role: string, label: UiText, choices: [
    string,
    UiText
][], value: string, changed: (value: string) => void): HTMLLabelElement {
    const wrapper = ui("label"), input = ui("select");
    input.dataset.controlRole = role;
    setUiAttr(input, "aria-label", label);
    for (const [id, title] of choices) {
        const option = ui("option", "", title);
        option.value = id;
        input.append(option);
    }
    input.value = value;
    input.onchange = () => changed(input.value);
    wrapper.append(uiTextNode(label), input);
    return wrapper;
}
/** A personal layout/filter state over one read receipt, never an author store. */
export class RelationshipGraphView {
    private filters?: GraphFilters;
    private selected = "";
    private selectedEdge = false;
    private search = "";
    private mode: "general" | "ranked" | "family" = "general";
    private canvas: CanvasState = { positions: new Map(), camera: defaultCamera() };
    private initialized = false;
    private pendingLocate?: string;
    private element?: HTMLElement;
    clear(): void { if (this.element) { disposeGraphCanvasTree(this.element); disposeUiTree(this.element); this.element.remove(); this.element = undefined; } this.filters = undefined; this.selected = ""; this.search = ""; this.initialized = false; this.pendingLocate = undefined; this.canvas = { positions: new Map(), camera: defaultCamera() }; }
    render(container: HTMLElement, snapshot: GraphSnapshot | undefined, root: string, host: RelationshipGraphHost): void {
        if (this.element) { disposeGraphCanvasTree(this.element); disposeUiTree(this.element); this.element.remove(); }
        const parent = ui("section", "knowledge-graph-view");
        this.element = parent;
        parent.append(ui("h3", "", message("graph.title")));
        container.append(parent);
        if (!snapshot || snapshot.scope !== "authorizedNativeSourcesAndTypedReferences") {
            parent.append(ui("p", "knowledge-help", message("graph.refreshAuthorizedSnapshot")));
            return;
        }
        if (!this.filters)
            this.filters = defaultGraphFilters(root);
        this.filters.root = root;
        const filter = this.filters;
        const redraw = () => this.render(container, snapshot, root, host), run = (work: () => unknown) => { void Promise.resolve().then(work).catch(host.error); };
        const controls = ui("div", "knowledge-graph-controls"), change = (work: () => void) => { work(); redraw(); };
        controls.append(select("scope", message("graph.scope"), [["local", message("graph.localScope")], ["global", message("graph.libraryScope")]], filter.scope, v => change(() => { filter.scope = v as GraphFilters["scope"]; filter.expanded = []; })), select("direction", message("graph.direction"), [["both", message("graph.bothDirections")], ["out", message("graph.outgoing")], ["in", message("graph.incoming")]], filter.direction, v => change(() => { filter.direction = v as GraphFilters["direction"]; filter.expanded = []; })), select("depth", message("graph.neighborhoodDepth"), [0, 1, 2, 3, 4, 8].map(n => [String(n), String(n)]), String(filter.depth), v => change(() => { filter.depth = Number(v); filter.expanded = []; })), select("relation", message("graph.relationshipType"), [["", message("graph.allRelationships")], ...[...new Set(snapshot.edges.map(e => e.relation))].map(id => [id, id] as [
                string,
                UiText
            ])], filter.relation, v => change(() => { filter.relation = v; })), select("directory", message("graph.directoryGroup"), [["", message("graph.allDirectories")], ...[...new Set(snapshot.nodes.map(n => n.directory).filter(Boolean))].sort().map(id => [id, id] as [
                string,
                UiText
            ])], filter.directory, v => change(() => { filter.directory = v; })), select("layout", message("graph.layout"), [["general", message("graph.generalLayout")], ["ranked", message("graph.hierarchicalLayout")], ["family", message("graph.familyLayout")]], this.mode, v => change(() => { this.mode = v as typeof this.mode; this.canvas.positions.clear(); })), action(message("graph.clearFilters"), () => change(() => { this.filters = defaultGraphFilters(root); this.search = ""; })));
        if (snapshot.tagTree) {
            try {
                const tree = new TagTree(snapshot.tagTree, true);
                controls.append(tagPicker(tree, { label: message("graph.tagCondition"), value: filter.tag, changed: reference => change(() => { filter.tag = reference ?? ""; }) }), select("tag-mode", message("graph.tagMatchMode"), [["subtree", message("graph.includeDescendants")], ["exact", message("graph.exactDirectTag")]], filter.tagMode, value => change(() => { filter.tagMode = value as GraphFilters["tagMode"]; })), action(message("graph.clearTagCondition"), () => change(() => { filter.tag = ""; })));
            }
            catch (error) {
                parent.append(ui("p", "knowledge-warning", error instanceof Error ? errorUiText(error) : message("graph.tagTreeUnavailable")));
            }
        }
        else
            parent.append(ui("p", "knowledge-help", message("graph.tagTreeUnauthorizedOrIncomplete")));
        parent.append(controls);
        if (filter.scope === "global") {
            for (const control of controls.querySelectorAll<HTMLSelectElement>('select[data-control-role="direction"],select[data-control-role="depth"]'))
                control.disabled = true;
            parent.append(ui("p", "knowledge-help", message("graph.libraryDirectionHint")));
        }
        let visible: ReturnType<typeof graphVisible>;
        try {
            visible = graphVisible(snapshot, filter);
        }
        catch (error) {
            parent.append(ui("p", "knowledge-warning", error instanceof Error ? errorUiText(error) : message("graph.tagConditionUnconfirmed")));
            return;
        }
        const nodes = new Map(snapshot.nodes.map(n => [n.key, n]));
        if (visible.tagIncomplete)
            parent.append(ui("p", "knowledge-warning", message("graph.partialTagMembers")));
        parent.append(ui("p", "knowledge-help", displayMessage("graph.scopeSummary", { p0: uiDisplayParameter(filter.scope === "local" ? displayMessage("graph.localNeighborhood", { p0: filter.depth, p1: uiDisplayParameter(filter.expanded.length ? displayMessage("graph.expansionCount", { p0: filter.expanded.length }) : "") }) : message("graph.authorizedScope")), p1: visible.nodes.length, p2: visible.edges.length })));
        if (snapshot.coverage === "partial")
            parent.append(ui("p", "knowledge-warning", message("graph.partialSourceCoverage")));
        if (snapshot.truncated)
            parent.append(ui("p", "knowledge-warning", message("graph.truncatedRead")));
        const search = ui("input");
        search.type = "search";
        search.value = this.search;
        setUiAttr(search, "aria-label", message("graph.searchAria"));
        setUiAttr(search, "placeholder", message("graph.searchPlaceholder"));
        const results = ui("div", "knowledge-graph-results");
        let paintDetail = () => { };
        const choose = (key: string, edge: boolean) => {
            this.selected = key;
            this.selectedEdge = edge;
            paintDetail();
            const neighbors = new Set<string>();
            if (key && !edge)
                for (const relation of visible.edges)
                    if (relation.source === key || relation.target === key) {
                        neighbors.add(relation.key);
                        neighbors.add(relation.source);
                        neighbors.add(relation.target);
                    }
            for (const element of parent.querySelectorAll<SVGElement>("[data-graph-key]")) {
                element.classList.toggle("is-selected", element.dataset.graphKey === key);
                element.classList.toggle("is-neighbor", neighbors.has(element.dataset.graphKey ?? ""));
            }
        }, open = (key: string) => {
            const node = nodes.get(key);
            if (node)
                run(() => host.open(node));
        };
        const paintMatches = () => {
            results.replaceChildren();
            const matches = graphMatches(visible.nodes, this.search);
            for (const node of matches)
                results.append(action(displayMessage("graph.locateNode", { p0: uiDisplayParameter(node.title) }), () => {
                    this.pendingLocate = node.key;
                    choose(node.key, false);
                    redraw();
                }));
            if (this.search && !matches.length)
                results.append(ui("p", "knowledge-help", message("graph.noMatchesInScope")));
        };
        search.oninput = () => { this.search = search.value; paintMatches(); };
        paintMatches();
        parent.append(search, results);
        const display = visible.nodes.slice(0, 200), displayIds = new Set(display.map(n => n.key)), edges = visible.edges.filter(e => displayIds.has(e.source) && displayIds.has(e.target)).slice(0, 1000), layout = graphLayout(display, edges.filter(e => !e.derived), this.mode, this.canvas.positions);
        for (const [key, point] of layout.positions)
            this.canvas.positions.set(key, point);
        const initialFit = display.length > 0 && (!this.initialized || !this.canvas.viewport);
        if (layout.cycle && this.mode !== "general")
            parent.append(ui("p", "knowledge-warning", message("graph.cycleUsesGeneralLayout")));
        if (display.length !== visible.nodes.length || edges.length !== visible.edges.length)
            parent.append(ui("p", "knowledge-warning", displayMessage("graph.canvasLimit", { p0: display.length, p1: edges.length })));
        parent.append(graphCanvas(display.map(n => ({ key: n.key, title: n.title })), edges.map(e => ({ key: e.key, source: e.source, target: e.target, directed: e.direction === "directed", label: () => `${uiTextValue(nodes.get(e.source)?.title)} ${uiTextValue(e.direction === "symmetric" ? "↔" : "→")} ${uiTextValue(nodes.get(e.target)?.title)} · ${uiTextValue(e.relation)}${uiTextValue(e.derived ? message("graph.derivedReverseSuffix") : "")}` })), this.canvas, choose, open, this.selected, message("graph.keyboardInstructions"), { fit: initialFit, revealKey: this.pendingLocate, centerReveal: true }));
        this.initialized = this.initialized || display.length > 0; this.pendingLocate = undefined;
        const detail = ui("div", "knowledge-card");
        paintDetail = () => {
            detail.replaceChildren();
            const selectedNode = !this.selectedEdge && visible.nodes.find(n => n.key === this.selected), selectedEdge = this.selectedEdge && visible.edges.find(e => e.key === this.selected);
            if (selectedNode) {
                detail.append(ui("h4", "", selectedNode.title), ui("p", "knowledge-help", selectedNode.key), action(message("graph.openNote"), () => open(selectedNode.key)), action(message("graph.expandNeighborhood"), () => change(() => {
                    if (!filter.expanded.includes(selectedNode.key))
                        filter.expanded.push(selectedNode.key);
                    filter.scope = "local";
                })));
            }
            else if (selectedEdge) {
                const edge = selectedEdge, author = nodes.get(edge.authorSource);
                detail.append(ui("h4", "", edge.relation), ui("p", "", () => `${uiTextValue(nodes.get(edge.source)?.title)} ${uiTextValue(edge.direction === "symmetric" ? message("graph.symmetricRelationship") : "→")} ${uiTextValue(nodes.get(edge.target)?.title)}`), ui("p", "knowledge-help", displayMessage("graph.sourceSummary", { p0: uiDisplayParameter(edge.derived ? message("graph.derivedReverseSource") : message("graph.originalAuthoredRelationship")), p1: uiDisplayParameter(author?.title ?? edge.authorSource), p2: uiDisplayParameter(edge.fieldId ? displayMessage("graph.fieldSource", { p0: uiDisplayParameter(edge.fieldId), p1: uiDisplayParameter(edge.itemPath.join(" / ")) }) : message("graph.nativeContent")) })));
                if (author)
                    detail.append(action(message("graph.openAuthoringSource"), () => run(() => host.source(edge, author))));
                detail.append(ui("p", "knowledge-help", message("graph.readOnlyEditingHint")));
            }
            else
                detail.append(ui("p", "knowledge-help", message("graph.selectionInstructions")));
            detail.append(action(message("graph.clearSelection"), () => choose("", false)));
        };
        paintDetail();
        parent.append(detail);
        const list = ui("details");
        list.append(ui("summary", "", message("graph.keyboardResults")));
        const nodeList = ui("ul");
        for (const node of visible.nodes) {
            const row = ui("li");
            const button = action(displayMessage("graph.selectNode", { p0: uiDisplayParameter(node.title) }), () => choose(node.key, false));
            button.ondblclick = () => open(node.key);
            button.onkeydown = event => {
                if (event.key === "Enter") {
                    event.preventDefault();
                    open(node.key);
                }
            };
            row.append(button, action(displayMessage("graph.openNode", { p0: uiDisplayParameter(node.title) }), () => open(node.key)));
            nodeList.append(row);
        }
        const edgeList = ui("ul");
        for (const edge of visible.edges) {
            const row = ui("li");
            row.append(action(() => `${uiTextValue(nodes.get(edge.source)?.title)} ${uiTextValue(edge.direction === "symmetric" ? "↔" : "→")} ${uiTextValue(nodes.get(edge.target)?.title)} · ${uiTextValue(edge.relation)}${uiTextValue(edge.derived ? message("graph.derivedReverseSuffix") : "")}`, () => choose(edge.key, true)));
            edgeList.append(row);
        }
        list.append(nodeList, edgeList);
        parent.append(list);
    }
}
