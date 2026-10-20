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
    const parent = ui("section", "knowledge-heading-structure"); parent.append(ui("h3", "", "只读标题结构")); container.append(parent);
    if (!snapshot) { parent.append(ui("p", "knowledge-help", "请刷新以取得同次原生编译的标题；不会按标题文字猜测源码位置。")); return; }
    const tree = headingTree(snapshot.items), byKey = new Map(tree.map(n => [n.key, n])), redraw = () => { parent.remove(); this.render(container, snapshot, open); };
    if (!byKey.has(this.scope)) this.scope = DOCUMENT_ROOT;
    const controls = ui("div", "knowledge-graph-controls"); controls.append(action("整篇", () => { this.scope = DOCUMENT_ROOT; redraw(); }), action("所选子树", () => { if (byKey.has(this.selected)) { this.scope = this.selected; redraw(); } }));
    const depth = ui("select"); depth.setAttribute("aria-label", "标题显示层数"); for (const [value, label] of [["0", "全部层数"], ["1", "1 层"], ["2", "2 层"], ["3", "3 层"], ["6", "6 层"]]) { const option = ui("option", "", label); option.value = value; depth.append(option); } depth.value = String(this.depth); depth.onchange = () => { this.depth = Number(depth.value); redraw(); }; controls.append(depth, action("全部展开", () => { this.collapsed.clear(); redraw(); }));
    const search = ui("input"); search.type = "search"; search.value = this.search; search.setAttribute("aria-label", "搜索真实标题"); search.placeholder = "搜索标题；清空恢复原折叠状态";
    const searchButton = action("查找", () => { this.search = search.value; this.match = 0; redraw(); }); search.onkeydown = event => { if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); this.search = search.value; this.match = 0; redraw(); } }; search.addEventListener("search", () => { if (!search.value) { this.search = ""; redraw(); } });
    const visible = visibleHeadings(tree, this.scope, this.collapsed, this.search, this.depth);
    const chooseMatch = (step: number) => { if (!visible.matches.length) return; this.match = (this.match + step + visible.matches.length) % visible.matches.length; this.selected = visible.matches[this.match]; const point = this.canvas.positions.get(this.selected); if (point) { this.canvas.camera.x = 350 - point.x * this.canvas.camera.scale; this.canvas.camera.y = 210 - point.y * this.canvas.camera.scale; } redraw(); };
    controls.append(search, searchButton, action("上一命中", () => chooseMatch(-1)), action("下一命中", () => chooseMatch(1)), ui("span", "knowledge-help", `${visible.matches.length} 个标题命中`)); parent.append(controls);
    parent.append(ui("p", "knowledge-help", `${this.scope === DOCUMENT_ROOT ? "整篇" : `子树：${byKey.get(this.scope)!.text}`} · 已展示 ${visible.items.length - (visible.items.some(n => n.key === DOCUMENT_ROOT) ? 1 : 0)} 个真实标题。文档根与同名一级标题各自存在；层级和子标题计数来自原生标题，不受搜索影响。`));
    if (snapshot.coverage === "partial") parent.append(ui("p", "knowledge-warning", "尚无完整同次原生编译，标题覆盖未确认。请刷新，不按原文猜造标题。")); if (snapshot.truncated) parent.append(ui("p", "knowledge-warning", "标题达到 5,000 项读取预算，结构已截断。"));
    const drawn = visible.items.slice(0, 200), ids = new Set(drawn.map(n => n.key)), edges = drawn.filter(n => ids.has(n.parent)).map(n => ({ key: `heading:${n.key}`, source: n.parent, target: n.key, direction: "directed" as const }));
    const positions = graphLayout(drawn, edges, "family", this.canvas.positions); for (const [key, point] of positions.positions) this.canvas.positions.set(key, point);
    if (!this.initialized && drawn.length) { this.initialized = true; this.canvas.camera = fitCamera([...positions.positions.values()], 700, 420); }
    if (drawn.length < visible.items.length) parent.append(ui("p", "knowledge-warning", `画布仅绘制 ${drawn.length} 项，下方标题列表保留本次范围全部结果。`));
    let detail: HTMLElement | undefined;
    const displaySelection = () => { if (!detail) return; detail.replaceChildren(); const node = byKey.get(this.selected); if (!node) return; detail.append(ui("h4", "", `${node.number ? `${node.number} ` : ""}${node.text}`), ui("p", "knowledge-help", node.key === DOCUMENT_ROOT ? `文档根 · ${node.descendants} 个子标题` : `真实层级 ${node.level} · ${node.children.length} 个直接子标题 · ${node.descendants} 个全部子标题`)); if (node.key !== DOCUMENT_ROOT) detail.append(action("跳转标题", () => open(node)), ui("p", "knowledge-help", node.generated ? "生成标题没有唯一作者写地址；仅按获证显示位置导航。" : "按同次原生标题的准确源码范围导航。")); };
    const choose = (key: string) => { this.selected = key || DOCUMENT_ROOT; displaySelection(); for (const element of parent.querySelectorAll<SVGElement>("[data-graph-key]")) element.classList.toggle("is-selected", element.dataset.graphKey === this.selected); };
    parent.append(graphCanvas(drawn.map(n => ({ key: n.key, title: n.key === DOCUMENT_ROOT ? "文档根（整篇）" : `${n.number ? `${n.number} ` : ""}${n.text}` })), edges.map(e => ({ ...e, directed: true, label: `${byKey.get(e.source)?.text} → ${byKey.get(e.target)?.text}（标题结构）` })), this.canvas, (key, edge) => { if (!edge) choose(key); }, key => { const node = byKey.get(key); if (node && node.key !== DOCUMENT_ROOT) open(node); }, this.selected, "只读标题结构；拖动只改变布局，不重排正文"));
    detail = ui("div", "knowledge-card"); displaySelection(); parent.append(detail);
    const list = ui("ul", "knowledge-heading-list");
    for (const heading of visible.items) { const row = ui("li"); row.dataset.headingKey = heading.key; row.style.paddingInlineStart = `${Math.min(12, heading.depth - (byKey.get(this.scope)?.depth ?? 0)) * 12}px`; const choose = action(`${heading.number ? `${heading.number} ` : ""}${heading.text} · ${heading.children.length} 个直接子标题／${heading.descendants} 个全部子标题`, () => { this.selected = heading.key; displaySelection(); }); choose.ondblclick = () => { if (heading.key !== DOCUMENT_ROOT) open(heading); }; choose.onkeydown = event => { if (event.key === "Enter" && heading.key !== DOCUMENT_ROOT) { event.preventDefault(); open(heading); } }; row.append(choose); if (heading.children.length) { const collapse = action(this.collapsed.has(heading.key) ? "展开分支" : "折叠分支", () => { if (this.collapsed.has(heading.key)) this.collapsed.delete(heading.key); else this.collapsed.add(heading.key); redraw(); }); collapse.setAttribute("aria-expanded", String(!this.collapsed.has(heading.key))); collapse.disabled = !!this.search; row.append(collapse); } if (heading.key !== DOCUMENT_ROOT) row.append(action("跳转", () => open(heading))); list.append(row); } parent.append(list);
  }
}
