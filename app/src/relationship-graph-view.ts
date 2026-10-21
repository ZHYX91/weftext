import { action, ui } from "./data-panel";
import { graphCanvas, type CanvasState } from "./knowledge-graph-canvas";
import { defaultCamera, defaultGraphFilters, fitCamera, graphLayout, graphMatches, graphVisible, type GraphSnapshot, type GraphNode, type GraphEdge, type GraphFilters } from "./relationship-graph-state";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
export interface RelationshipGraphHost { open(node: GraphNode): void | Promise<void>; source(edge: GraphEdge, node: GraphNode): void | Promise<void>; error(error: unknown): void }
function select(label: string, choices: [string, string][], value: string, changed: (value: string) => void): HTMLLabelElement { const wrapper = ui("label"), input = ui("select"); input.setAttribute("aria-label", label); for (const [id, title] of choices) { const option = ui("option", "", title); option.value = id; input.append(option); } input.value = value; input.onchange = () => changed(input.value); wrapper.append(document.createTextNode(label), input); return wrapper; }
/** A personal layout/filter state over one read receipt, never an author store. */
export class RelationshipGraphView {
  private filters?: GraphFilters;
  private selected = "";
  private selectedEdge = false;
  private search = "";
  private mode: "general" | "ranked" | "family" = "general";
  private canvas: CanvasState = { positions: new Map(), camera: defaultCamera() };
  private initialized = false;
  clear(): void { this.filters = undefined; this.selected = ""; this.search = ""; this.initialized = false; this.canvas = { positions: new Map(), camera: defaultCamera() }; }
  render(container: HTMLElement, snapshot: GraphSnapshot | undefined, root: string, host: RelationshipGraphHost): void {
    const parent = ui("section", "knowledge-graph-view"); parent.append(ui("h3", "", "二维关系图")); container.append(parent);
    if (!snapshot || snapshot.scope !== "authorizedNativeSourcesAndTypedReferences") { parent.append(ui("p", "knowledge-help", "请刷新以取得当前获权关系快照。")); return; }
    if (!this.filters) this.filters = defaultGraphFilters(root);
    this.filters.root = root; const filter = this.filters;
    const redraw = () => { parent.remove(); this.render(container, snapshot, root, host); }, run = (work: () => unknown) => { void Promise.resolve().then(work).catch(host.error); };
    const controls = ui("div", "knowledge-graph-controls"), change = (work: () => void) => { work(); redraw(); };
    controls.append(
      select("范围", [["local", "当前笔记局部图"], ["global", "本次获权全库图"]], filter.scope, v => change(() => { filter.scope = v as GraphFilters["scope"]; filter.expanded = []; })),
      select("方向", [["both", "双向"], ["out", "出向"], ["in", "入向"]], filter.direction, v => change(() => { filter.direction = v as GraphFilters["direction"]; filter.expanded = []; })),
      select("邻域深度", [0, 1, 2, 3, 4, 8].map(n => [String(n), String(n)]), String(filter.depth), v => change(() => { filter.depth = Number(v); filter.expanded = []; })),
      select("关系类型", [["", "全部关系"], ...[...new Set(snapshot.edges.map(e => e.relation))].map(id => [id, id] as [string, string])], filter.relation, v => change(() => { filter.relation = v; })),
      select("目录／配置分组", [["", "全部目录"], ...[...new Set(snapshot.nodes.map(n => n.directory).filter(Boolean))].sort().map(id => [id, id] as [string, string])], filter.directory, v => change(() => { filter.directory = v; })),
      select("布局", [["general", "一般二维"], ["ranked", "层级二维"], ["family", "家族二维"]], this.mode, v => change(() => { this.mode = v as typeof this.mode; this.canvas.positions.clear(); })),
      action("清除筛选", () => change(() => { this.filters = defaultGraphFilters(root); this.search = ""; })),
    );
    if (snapshot.tagTree) {
      try { const tree = new TagTree(snapshot.tagTree, true); controls.append(tagPicker(tree, { label: "图谱标签条件", value: filter.tag, changed: reference => change(() => { filter.tag = reference ?? ""; }) }), select("标签匹配模式", [["subtree", "含子级"], ["exact", "精确直接标签"]], filter.tagMode, value => change(() => { filter.tagMode = value as GraphFilters["tagMode"]; })), action("清除标签条件", () => change(() => { filter.tag = ""; }))); } catch (error) { parent.append(ui("p", "knowledge-warning", error instanceof Error ? error.message : "标签树依据不可用。")); }
    } else parent.append(ui("p", "knowledge-help", "标签结构未获权或未完整读取；树筛选暂不可用。笔记权限与标签结构权限分别核对。"));
    parent.append(controls);
    if (filter.scope === "global") {
      for (const control of controls.querySelectorAll<HTMLSelectElement>('select[aria-label="方向"],select[aria-label="邻域深度"]')) control.disabled = true;
      parent.append(ui("p", "knowledge-help", "全库显示全部方向；入向／出向和邻域深度以当前笔记为根，在局部图中启用。"));
    }
    let visible: ReturnType<typeof graphVisible>;
    try { visible = graphVisible(snapshot, filter); } catch (error) { parent.append(ui("p", "knowledge-warning", error instanceof Error ? error.message : "标签条件当前无法确认。")); return; }
    const nodes = new Map(snapshot.nodes.map(n => [n.key, n]));
    if (visible.tagIncomplete) parent.append(ui("p", "knowledge-warning", "标签条件仅显示本次独立获权且已确认的成员；未获权字段或失效标签保持未知，不作为完整空结果。"));
    parent.append(ui("p", "knowledge-help", `${filter.scope === "local" ? `当前笔记 ${filter.depth} 层邻域${filter.expanded.length ? `，显式展开 ${filter.expanded.length} 次` : ""}` : "本次明确获权范围"} · 已展示 ${visible.nodes.length} 个节点、${visible.edges.length} 条关系。目录和标题不作为业务关系边。`));
    if (snapshot.coverage === "partial") parent.append(ui("p", "knowledge-warning", "关系来源部分覆盖：未确认的原生生成关系或字段保持未知；没有边不代表全库无关系。"));
    if (snapshot.truncated) parent.append(ui("p", "knowledge-warning", "读取达到节点或关系预算，结果已截断；未公开未读取或无权对象的数量。"));
    const search = ui("input"); search.type = "search"; search.value = this.search; search.setAttribute("aria-label", "在当前图范围搜索定位"); search.placeholder = "搜索当前范围；不会扩大图范围"; const results = ui("div", "knowledge-graph-results");
    let paintDetail = () => {};
    const choose = (key: string, edge: boolean) => { this.selected = key; this.selectedEdge = edge; paintDetail(); const neighbors = new Set<string>(); if (key && !edge) for (const relation of visible.edges) if (relation.source === key || relation.target === key) { neighbors.add(relation.key); neighbors.add(relation.source); neighbors.add(relation.target); } for (const element of parent.querySelectorAll<SVGElement>("[data-graph-key]")) { element.classList.toggle("is-selected", element.dataset.graphKey === key); element.classList.toggle("is-neighbor", neighbors.has(element.dataset.graphKey ?? "")); } }, open = (key: string) => { const node = nodes.get(key); if (node) run(() => host.open(node)); };
    const paintMatches = () => { results.replaceChildren(); const matches = graphMatches(visible.nodes, this.search); for (const node of matches) results.append(action(`定位 ${node.title}`, () => { const point = this.canvas.positions.get(node.key); if (point) { this.canvas.camera.x = 350 - point.x * this.canvas.camera.scale; this.canvas.camera.y = 210 - point.y * this.canvas.camera.scale; } choose(node.key, false); redraw(); })); if (this.search && !matches.length) results.append(ui("p", "knowledge-help", "当前范围没有匹配；需要扩大范围时请明确修改范围或筛选。")); };
    search.oninput = () => { this.search = search.value; paintMatches(); }; paintMatches(); parent.append(search, results);
    const display = visible.nodes.slice(0, 200), displayIds = new Set(display.map(n => n.key)), edges = visible.edges.filter(e => displayIds.has(e.source) && displayIds.has(e.target)).slice(0, 1000), layout = graphLayout(display, edges.filter(e => !e.derived), this.mode, this.canvas.positions); for (const [key, point] of layout.positions) this.canvas.positions.set(key, point);
    if (!this.initialized && display.length) { this.initialized = true; this.canvas.camera = fitCamera([...layout.positions.values()], 700, 420); }
    if (layout.cycle && this.mode !== "general") parent.append(ui("p", "knowledge-warning", "当前关系含循环；保留全部事实并采用一般布局。可在列表检查原关系。"));
    if (display.length !== visible.nodes.length || edges.length !== visible.edges.length) parent.append(ui("p", "knowledge-warning", `画布仅绘制 ${display.length} 个节点、${edges.length} 条关系；下方列表保留本次范围全部结果。`));
    parent.append(graphCanvas(display.map(n => ({ key: n.key, title: n.title })), edges.map(e => ({ key: e.key, source: e.source, target: e.target, directed: e.direction === "directed", label: `${nodes.get(e.source)?.title} ${e.direction === "symmetric" ? "↔" : "→"} ${nodes.get(e.target)?.title} · ${e.relation}${e.derived ? "（派生反向）" : ""}` })), this.canvas, choose, open, this.selected, "关系图；方向键平移，节点 Enter 打开，空格选择，Esc 清选择"));
    const detail = ui("div", "knowledge-card");
    paintDetail = () => { detail.replaceChildren(); const selectedNode = !this.selectedEdge && visible.nodes.find(n => n.key === this.selected), selectedEdge = this.selectedEdge && visible.edges.find(e => e.key === this.selected);
    if (selectedNode) { detail.append(ui("h4", "", selectedNode.title), ui("p", "knowledge-help", selectedNode.key), action("打开笔记", () => open(selectedNode.key)), action("展开此节点邻域", () => change(() => { if (!filter.expanded.includes(selectedNode.key)) filter.expanded.push(selectedNode.key); filter.scope = "local"; }))); }
    else if (selectedEdge) { const edge = selectedEdge, author = nodes.get(edge.authorSource); detail.append(ui("h4", "", edge.relation), ui("p", "", `${nodes.get(edge.source)?.title} ${edge.direction === "symmetric" ? "↔ 对称关系 ↔" : "→"} ${nodes.get(edge.target)?.title}`), ui("p", "knowledge-help", `${edge.derived ? "派生反向；作者事实保存在原来源，不另写反边。" : "原始作者关系。"} 来源：${author?.title ?? edge.authorSource} · ${edge.fieldId ? `字段 ${edge.fieldId} ${edge.itemPath.join(" / ")}` : "原生正文"}`)); if (author) detail.append(action("打开准确作者来源", () => run(() => host.source(edge, author)))); detail.append(ui("p", "knowledge-help", "图谱只读。新增或移除关系请在原始字段／正文入口按其独立权限编辑；拖动只调整布局。")); }
    else detail.append(ui("p", "knowledge-help", "单击／空格选择节点或边查看详情；双击／Enter 打开笔记。选择和搜索不自动移动视口。"));
    detail.append(action("清除选择", () => choose("", false))); }; paintDetail(); parent.append(detail);
    const list = ui("details"); list.append(ui("summary", "", "键盘与完整结果列表")); const nodeList = ui("ul");
    for (const node of visible.nodes) { const row = ui("li"); const button = action(`选择 ${node.title}`, () => choose(node.key, false)); button.ondblclick = () => open(node.key); button.onkeydown = event => { if (event.key === "Enter") { event.preventDefault(); open(node.key); } }; row.append(button, action(`打开 ${node.title}`, () => open(node.key))); nodeList.append(row); }
    const edgeList = ui("ul"); for (const edge of visible.edges) { const row = ui("li"); row.append(action(`${nodes.get(edge.source)?.title} ${edge.direction === "symmetric" ? "↔" : "→"} ${nodes.get(edge.target)?.title} · ${edge.relation}${edge.derived ? "（派生反向）" : ""}`, () => choose(edge.key, true))); edgeList.append(row); } list.append(nodeList, edgeList); parent.append(list);
  }
}
