import { TagTree } from "./tag-tree";
export interface TagPickerOptions { label: string; value?: string; root?: boolean; excludedRoot?: string; changed(reference: string | null): void }
/** Bounded native controls keep keyboard paths usable even for a 5,000-tag chain. */
export function tagPicker(tree: TagTree, options: TagPickerOptions): HTMLElement {
  const host = document.createElement("fieldset"), legend = document.createElement("legend"); legend.textContent = options.label; host.append(legend);
  const search = document.createElement("input"); search.type = "search"; search.className = "data-input"; search.setAttribute("aria-label", `${options.label} · 搜索名称或身份`);
  const select = document.createElement("select"); select.className = "data-input"; select.setAttribute("aria-label", options.label);
  const previous = document.createElement("button"), next = document.createElement("button"), status = document.createElement("p"), path = document.createElement("p"), full = document.createElement("button");
  for (const button of [previous, next, full]) { button.type = "button"; button.className = "data-button compact"; }
  previous.textContent = "上一段"; next.textContent = "下一段"; full.textContent = "查看完整分段路径"; let offset = 0, selected = options.value ?? "";
  const render = () => {
    const page = tree.page(search.value, offset, options.excludedRoot); offset = page.offset; select.replaceChildren();
    const add = (value: string, label: string) => { const option = document.createElement("option"); option.value = value; option.textContent = label; select.append(option); return option; };
    add("", options.root ? "根级（无父标签）" : "明确选择直接标签…");
    const selectedTag = tree.byRef.get(selected), eligible = selectedTag && (!options.excludedRoot || !tree.contains(options.excludedRoot, selectedTag.tagId));
    if (selected && !page.items.some(tag => tag.reference === selected)) add(selected, eligible ? `当前选择：${tree.label(selected)}` : "原选择当前不可用（保留身份）").disabled = !eligible;
    for (const item of page.items) add(item.reference, tree.label(item.reference)); select.value = selected;
    status.textContent = `完整标签结构 · ${page.total} 个可选项 · 当前 ${page.total ? offset + 1 : 0}–${offset + page.items.length}（每段最多 100）`;
    previous.disabled = offset === 0; next.disabled = offset + page.items.length >= page.total;
    full.disabled = !selected || !tree.byRef.has(selected); path.textContent = selected && tree.byRef.has(selected) ? tree.label(selected) : "";
  };
  search.oninput = () => { offset = 0; render(); }; previous.onclick = () => { offset -= 100; render(); }; next.onclick = () => { offset += 100; render(); };
  select.onchange = () => { selected = select.value; options.changed(selected || null); render(); }; full.onclick = () => { path.textContent = tree.label(selected, true); };
  host.append(search, select, previous, next, status, full, path); render(); return host;
}
