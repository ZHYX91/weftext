import { type WritingController, type WritingStructure, type WritingTable, type WritingTableCell, type WritingSnapshot, type TableAction } from "./writing";
import { delimitedTable, flatClipboard, parseHTMLTable, writeDelimited, writeHTMLTable, type ClipboardTable } from "./table-clipboard";
import { ChangeSet } from "@codemirror/state";

export interface TableGridOptions { onError?(error: Error): void; revealSource?(range: WritingStructure["range"]): void; renderedTable?: Element }
interface Position { row: number; column: number }
interface Rectangle { top: number; left: number; bottom: number; right: number }
/** Personal selection only, keyed by a freshly verified native source hash. */
const selections = new WeakMap<WritingController, Map<string, { anchor: Position; head: Position }>>();
function node<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: string): HTMLElementTagNameMap[K] { const result = document.createElement(tag); result.className = className; if (text != null) result.textContent = text; return result; }
export function logicalTableCells(table: WritingTable): WritingTableCell[] {
  return table.logicalCells ?? table.cells.map((body, index) => ({ row: Math.floor(index / table.columns), column: index % table.columns, rowspan: 1, colspan: 1, body, range: body, header: false }));
}
export function tableCellAt(table: WritingTable, at: Position): WritingTableCell | undefined {
  return logicalTableCells(table).find(cell => cell.row <= at.row && cell.row + cell.rowspan > at.row && cell.column <= at.column && cell.column + cell.colspan > at.column);
}
/** Span closure always selects complete logical author slots. */
export function tableRectangle(table: WritingTable, anchor: Position, head: Position): Rectangle {
  const range = { top: Math.min(anchor.row, head.row), left: Math.min(anchor.column, head.column), bottom: Math.max(anchor.row, head.row), right: Math.max(anchor.column, head.column) };
  let changed = true;
  while (changed) {
    changed = false;
    for (const cell of logicalTableCells(table)) if (cell.row <= range.bottom && cell.row + cell.rowspan > range.top && cell.column <= range.right && cell.column + cell.colspan > range.left) {
      const next = { top: Math.min(range.top, cell.row), left: Math.min(range.left, cell.column), bottom: Math.max(range.bottom, cell.row + cell.rowspan - 1), right: Math.max(range.right, cell.column + cell.colspan - 1) };
      changed ||= next.top !== range.top || next.left !== range.left || next.bottom !== range.bottom || next.right !== range.right; Object.assign(range, next);
    }
  }
  return range;
}
export function adjacentTableCell(table: WritingTable, current: Position, direction: "previous" | "next" | "up" | "down" | "left" | "right"): Position | null {
  const cell = tableCellAt(table, current); if (!cell) return null;
  const cells = logicalTableCells(table).sort((a, b) => a.row - b.row || a.column - b.column);
  if (direction === "previous" || direction === "next") {
    const index = cells.findIndex(item => item.row === cell.row && item.column === cell.column), target = cells[index + (direction === "next" ? 1 : -1)]; return target ? { row: target.row, column: target.column } : null;
  }
  const at = { row: direction === "up" ? cell.row - 1 : direction === "down" ? cell.row + cell.rowspan : current.row, column: direction === "left" ? cell.column - 1 : direction === "right" ? cell.column + cell.colspan : current.column };
  const target = tableCellAt(table, at); return target ? { row: target.row, column: target.column } : null;
}

/** Native source remains the only table model; this state stores disposable selection. */
export function createTableGrid(controller: WritingController, initial: WritingStructure, options: TableGridOptions = {}): HTMLElement {
  const surface = node("section", "writing-table-surface"), tools = node("div", "writing-table-tools"), gridHost = node("div", "writing-table-scroll"), status = node("p", "writing-help"), notice = node("p", "writing-help"), error = node("p", "writing-error");
  surface.setAttribute("aria-label", "正文表格编辑"); tools.setAttribute("role", "toolbar"); tools.setAttribute("aria-label", "表格行列与范围操作"); error.setAttribute("role", "alert"); error.hidden = true;
  status.setAttribute("aria-live", "polite"); notice.setAttribute("role", "status");
  const nativeSnapshot = controller.snapshot;
  let current = initial, snapshot = controller.snapshot, rendered: WritingSnapshot | null = null, anchor: Position = { row: 0, column: 0 }, head = { ...anchor }, editing = false, updating = false, scheduled = false, localBusy = false, stale = false, dragging: { axis: "row" | "column"; from: number } | null = null;
  let grid: HTMLTableElement, inputs = new Map<string, HTMLTextAreaElement>(), bindings: (() => void)[] = [];
  let selectionVersion = 0, pointerEngaged = false;
  const selectionKey = () => `${controller.snapshot.sessionId}:${controller.map?.sourceHash}:${current.range.utf16From}:${current.range.utf16To}`;
  const remembered = selections.get(controller)?.get(selectionKey()), sourceSelection = controller.sourceSelection;
  const sourceCell = logicalTableCells(initial.table!).find(cell => cell.body.utf16From <= sourceSelection.from && cell.body.utf16To >= sourceSelection.to);
  if (remembered) { anchor = { ...remembered.anchor }; head = { ...remembered.head }; }
  else if (sourceCell) { anchor = { row: sourceCell.row, column: sourceCell.column }; head = { ...anchor }; }
  const rememberSelection = () => {
    let saved = selections.get(controller); if (!saved) { saved = new Map(); selections.set(controller, saved); }
    const key = selectionKey(); saved.delete(key); saved.set(key, { anchor: { ...anchor }, head: { ...head } });
    if (saved.size > 128) saved.delete(saved.keys().next().value!);
  };
  // Mousedown can blur a focused grid before its later click starts editing.
  // Keep this exact mounted target through that gesture, including cross-table clicks.
  surface.addEventListener("pointerdown", event => {
    if (controller.state.composing && event.target !== document.activeElement) { event.preventDefault(); return; }
    pointerEngaged = true;
    const release = () => { document.removeEventListener("pointerup", release, true); document.removeEventListener("pointercancel", release, true); setTimeout(() => { pointerEngaged = false; controller.surfaceStateChanged(); }, 0); };
    document.addEventListener("pointerup", release, { capture: true, once: true }); document.addEventListener("pointercancel", release, { capture: true, once: true });
  }, { capture: true });
  surface.addEventListener("click", () => queueMicrotask(() => { pointerEngaged = false; controller.surfaceStateChanged(); }));
  surface.addEventListener("pointercancel", () => { pointerEngaged = false; controller.surfaceStateChanged(); });
  const key = (at: Position) => `${at.row}:${at.column}`;
  const table = () => current.table!;
  const available = () => table().topologyWritable ?? table().gridWritable;
  const report = (failure: unknown) => { const problem = failure instanceof Error ? failure : new Error(String(failure)); error.hidden = false; error.textContent = problem.message; options.onError?.(problem); };
  const requireCurrent = () => { if (!surface.isConnected || stale || !controller.isSnapshotCurrent(snapshot)) throw new Error("表格来源或视图已变化，请重新选择当前表格；输入仍保留在共享草稿。"); };
  const literal = (cell: WritingTableCell) => {
    const matches = controller.map?.literalRegions.filter(region => region.range.utf16From === cell.body.utf16From && region.range.utf16To === cell.body.utf16To && region.range.raw === cell.body.raw) ?? [];
    return matches.length === 1 ? matches[0] : cell.body.raw === "" && cell.body.writable ? { range: cell.body, text: "", role: "tableCell" } : null;
  };
  const refreshCurrent = () => {
    if (!controller.map) return false;
    const matches = controller.map.structures.filter(item => item.kind === "table" && item.range.utf16From === current.range.utf16From);
    if (matches.length !== 1 || !matches[0].table) { stale = true; return false; }
    current = matches[0]; snapshot = controller.snapshot;
    head = { row: Math.min(head.row, table().rows - 1), column: Math.min(head.column, table().columns - 1) };
    anchor = { row: Math.min(anchor.row, table().rows - 1), column: Math.min(anchor.column, table().columns - 1) }; return true;
  };
  const selectedRange = () => tableRectangle(table(), anchor, head);
  const paintSelection = () => {
    if (!grid) return; const range = selectedRange();
    for (const element of Array.from(grid.querySelectorAll<HTMLElement>("[data-cell-row]"))) {
      const row = Number(element.dataset.cellRow), column = Number(element.dataset.cellColumn), selected = row >= range.top && row <= range.bottom && column >= range.left && column <= range.right;
      element.setAttribute("aria-selected", String(selected)); element.classList.toggle("table-selected", selected); element.classList.toggle("table-current", row === head.row && column === head.column);
    }
    const active = tableCellAt(table(), head); grid.setAttribute("aria-activedescendant", active ? `${grid.id}-${key(active)}` : ""); rememberSelection();
    status.textContent = `${table().rows} 行 × ${table().columns} 列；已选第 ${range.top + 1}–${range.bottom + 1} 行、第 ${range.left + 1}–${range.right + 1} 列。${editing ? "格内方向键选择文字；Shift+Enter 换行，Enter 下一行，Esc 保留输入并返回网格。" : "方向键移动，Shift 扩选，Enter/F2 编辑，Delete 只清内容。"}${available() ? "末行 Enter 可追加一行。" : `结构只读：${table().unavailableReason || "来源或拓扑不能证明"}`}`;
  };
  const select = (position: Position, extend = false, focus = true) => {
    const cell = tableCellAt(table(), position); if (!cell) return;
    head = { row: cell.row, column: cell.column }; if (!extend) anchor = { ...head }; selectionVersion++; editing = false;
    controller.selectRange(cell.body); paintSelection(); if (focus) grid.focus({ preventScroll: true });
    grid.querySelector<HTMLElement>(`[data-cell-row="${head.row}"][data-cell-column="${head.column}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  };
  const updateButtons = () => {
    tools.setAttribute("aria-busy", String(localBusy));
    const unavailable = controller.state.composing || stale || !controller.map;
    for (const button of Array.from(tools.querySelectorAll<HTMLButtonElement>("button"))) { button.disabled = unavailable; button.setAttribute("aria-disabled", String(unavailable || localBusy)); }
  };
  const run = (operation: () => Promise<void> | void) => { if (localBusy) return; localBusy = true; updateButtons(); error.hidden = true; notice.textContent = ""; Promise.resolve().then(operation).catch(report).finally(() => { localBusy = false; updateButtons(); controller.surfaceStateChanged(); }); };
  const settle = async () => { requireCurrent(); if (controller.state.composing) throw new Error("请先完成输入法组合；当前文字和位置已保留。"); await controller.flush(); if (!refreshCurrent()) throw new Error("当前表格映射不可用，文字已保留。"); };
  const confirmCurrent = async (title: string, message: string, label: string): Promise<boolean> => {
    await settle(); const basis = controller.snapshot, selection = selectionVersion;
    const accepted = await confirmProposal(title, message, label);
    if (!accepted) return false;
    if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection) throw new Error("表格提案已过期，未将旧选区应用到当前稿。"); return true;
  };
  const command = async (change: TableAction, next: Position = head) => {
    await settle(); if (!available()) throw new Error(table().unavailableReason || "当前表格不能安全变更结构，原稿保留。");
    const beforeRange = selectedRange(), rowSelected = beforeRange.top === beforeRange.bottom && beforeRange.left === 0 && beforeRange.right === table().columns - 1, columnSelected = beforeRange.left === beforeRange.right && beforeRange.top === 0 && beforeRange.bottom === table().rows - 1;
    editing = false; grid.focus({ preventScroll: true });
    await controller.execute(() => { requireCurrent(); return { kind: "tableEdit", range: current.range, action: change }; });
    if (!refreshCurrent()) throw new Error("变更已进入草稿，等待当前表格预览。"); anchor = { ...next }; head = { ...next }; selectionVersion++; render(true); select(head);
    if (change.kind === "moveRow" && rowSelected) { anchor = { row: next.row, column: 0 }; head = { row: next.row, column: table().columns - 1 }; paintSelection(); }
    else if (change.kind === "moveColumn" && columnSelected) { anchor = { row: 0, column: next.column }; head = { row: table().rows - 1, column: next.column }; paintSelection(); }
  };
  const edit = async (position = head, selectWhole = false) => {
    await settle(); render(); const cell = tableCellAt(table(), position); if (!cell) return;
    const input = inputs.get(key(cell)); if (!input) { select(cell); throw new Error("此格包含格式、公式或生成内容；保留原生内容，可使用编辑此表源码。"); }
    anchor = { row: cell.row, column: cell.column }; head = { ...anchor }; selectionVersion++; editing = true; input.hidden = false; input.parentElement!.classList.add("table-editing"); input.focus({ preventScroll: true }); input.setSelectionRange(selectWhole ? 0 : input.value.length, input.value.length); paintSelection();
    if (selectWhole) status.textContent += " 点击字符位置尚未映射，已选择本格全部文字；可在输入框内继续放置光标。";
  };
  const finishText = async () => { await settle(); editing = false; grid.focus({ preventScroll: true }); render(true); paintSelection(); };
  const go = async (direction: "previous" | "next" | "up" | "down", enter = false) => {
    await finishText(); const target = adjacentTableCell(table(), head, direction);
    if (target) { select(target); if (enter) await edit(); return; }
    if (enter && direction === "down") {
      const next = { row: table().rows, column: head.column }; await command({ kind: "insertRow", index: table().rows }, next); await edit(next);
    } else if (direction === "next" || direction === "previous") {
      // The native Tab boundary stays reachable outside the table.
      const candidates = Array.from(document.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]')).filter(item => item.getClientRects().length && !surface.contains(item));
      const candidate = direction === "next" ? candidates.find(item => !!(surface.compareDocumentPosition(item) & Node.DOCUMENT_POSITION_FOLLOWING)) : candidates.filter(item => !!(surface.compareDocumentPosition(item) & Node.DOCUMENT_POSITION_PRECEDING)).at(-1);
      candidate?.focus();
    }
  };
  function render(force = false): void {
    if (updating || stale || !controller.map) return;
    if (!force && rendered && controller.isSnapshotCurrent(rendered)) { paintSelection(); return; }
    if (editing && document.activeElement instanceof HTMLTextAreaElement && surface.contains(document.activeElement)) return;
    updating = true; bindings.splice(0).forEach(dispose => dispose()); inputs = new Map();
    const previousFocus = grid && document.activeElement === grid, previousScroll = { top: gridHost.scrollTop, left: gridHost.scrollLeft };
    grid = node("table", "writing-table-grid"); grid.id = `writing-grid-${Math.random().toString(16).slice(2)}`; grid.tabIndex = 0; grid.setAttribute("role", "grid"); grid.setAttribute("aria-label", "正文表格单元格选择"); grid.setAttribute("aria-multiselectable", "true");
    grid.setAttribute("aria-rowcount", String(table().rows)); grid.setAttribute("aria-colcount", String(table().columns));
    const nativeCells = Array.from(options.renderedTable?.querySelectorAll("td,th") ?? []), cells = logicalTableCells(table()), tbody = node("tbody");
    const header = node("tr"); header.append(node("th"));
    for (let column = 0; column < table().columns; column++) {
      const th = node("th", "table-handle"), button = node("button", "writing-button", `${column + 1}`); button.type = "button"; button.setAttribute("aria-label", `选择第 ${column + 1} 列`);
      button.onclick = () => { void finishText().then(() => { anchor = { row: 0, column }; head = { row: table().rows - 1, column }; selectionVersion++; const cell = tableCellAt(table(), head); if (cell) controller.selectRange(cell.body); paintSelection(); grid.focus(); }).catch(report); }; setupDrag(button, "column", column); th.append(button); header.append(th);
    }
    tbody.append(header);
    for (let row = 0; row < table().rows; row++) {
      const tr = node("tr"), handle = node("th", "table-handle"), button = node("button", "writing-button", `${row + 1}`); button.type = "button"; button.setAttribute("aria-label", `选择第 ${row + 1} 行`);
      button.onclick = () => { void finishText().then(() => { anchor = { row, column: 0 }; head = { row, column: table().columns - 1 }; selectionVersion++; const cell = tableCellAt(table(), head); if (cell) controller.selectRange(cell.body); paintSelection(); grid.focus(); }).catch(report); }; setupDrag(button, "row", row); handle.append(button); tr.append(handle);
      for (const cell of cells.filter(item => item.row === row).sort((a, b) => a.column - b.column)) {
        const td = node(cell.header ? "th" : "td", "writing-table-cell"); td.rowSpan = cell.rowspan; td.colSpan = cell.colspan; td.dataset.cellRow = String(cell.row); td.dataset.cellColumn = String(cell.column); td.id = `${grid.id}-${key(cell)}`; td.setAttribute("role", "gridcell"); td.setAttribute("aria-rowindex", String(cell.row + 1)); td.setAttribute("aria-colindex", String(cell.column + 1));
        const region = literal(cell), display = node("div", "table-cell-display");
        if (region?.range.writable) {
          display.textContent = region.text || "\u00a0"; const input = node("textarea", "literal-input table-cell-input"); input.rows = 1; input.hidden = true; input.setAttribute("aria-label", `第 ${cell.row + 1} 行第 ${cell.column + 1} 列文字`); bindings.push(controller.bindLiteral(input, region)); inputs.set(key(cell), input);
          input.addEventListener("keydown", event => {
            if (event.isComposing || event.keyCode === 229 || controller.state.composing) return;
            if (event.key === "Tab" || event.key === "Escape" || event.key === "Enter" && !event.shiftKey && !(event.ctrlKey || event.metaKey || event.altKey)) {
              event.preventDefault(); event.stopImmediatePropagation(); run(() => event.key === "Escape" ? finishText() : go(event.key === "Tab" ? event.shiftKey ? "previous" : "next" : "down", true));
            }
          }, { capture: true }); td.append(display, input);
        } else {
          const native = controller.isSnapshotCurrent(nativeSnapshot) ? nativeCells[cells.indexOf(cell)] : null; if (region) display.textContent = region.text || "\u00a0"; else if (native) {
            const clone = native.cloneNode(true) as HTMLElement, originalLinks = Array.from(native.querySelectorAll<HTMLAnchorElement>("a"));
            clone.querySelectorAll<HTMLTextAreaElement>("textarea").forEach(input => input.replaceWith(document.createTextNode(input.value)));
            clone.querySelectorAll("button,input,[contenteditable]").forEach(element => element.remove());
            clone.querySelectorAll<HTMLAnchorElement>("a").forEach((link, index) => { link.onclick = event => { event.preventDefault(); event.stopPropagation(); if (!controller.isSnapshotCurrent(nativeSnapshot)) { report(new Error("表格链接投影已过期，请等待当前预览。")); return; } originalLinks[index]?.click(); }; });
            while (clone.firstChild) display.append(clone.firstChild);
          }
          else display.textContent = "原生内容（文字只读）"; td.title = "此格没有唯一的文字写入来源；可选择、复制或编辑此表源码。"; td.append(display);
        }
        td.addEventListener("click", event => {
          if (event.target instanceof HTMLTextAreaElement || controller.state.composing) return;
          if (event.shiftKey) { run(async () => { await finishText(); select(cell, true); }); }
          else run(async () => { await finishText(); select(cell); if (region?.range.writable) await edit(cell, true); });
        });
        td.addEventListener("pointerdown", event => {
          if (!(event.ctrlKey || event.metaKey) || controller.state.composing) return; event.preventDefault(); run(async () => { await finishText(); select(cell, event.shiftKey); });
        });
        tr.append(td);
      }
      tbody.append(tr);
    }
    grid.append(tbody); gridHost.replaceChildren(grid); gridHost.scrollTop = previousScroll.top; gridHost.scrollLeft = previousScroll.left; rendered = controller.snapshot; updating = false; paintSelection(); if (previousFocus) grid.focus({ preventScroll: true });
    grid.onkeydown = event => {
      if (event.target !== grid || event.isComposing || event.keyCode === 229 || controller.state.composing) return;
      const directions = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" } as const;
      if (event.key in directions) { event.preventDefault(); const target = adjacentTableCell(table(), head, directions[event.key as keyof typeof directions]); if (target) select(target, event.shiftKey); }
      else if (["Enter", "F2"].includes(event.key)) { event.preventDefault(); run(() => edit()); }
      else if (event.key === "Tab") { event.preventDefault(); run(() => go(event.shiftKey ? "previous" : "next")); }
      else if (["Delete", "Backspace"].includes(event.key)) { event.preventDefault(); run(() => command({ kind: "clearRange", ...selectedRange() })); }
      else if (event.key === "Escape") { dragging = null; surface.querySelectorAll(".table-drop-target").forEach(item => item.classList.remove("table-drop-target")); }
      else if ((event.ctrlKey || event.metaKey) && ["z", "y"].includes(event.key.toLowerCase())) { event.preventDefault(); run(() => event.key.toLowerCase() === "y" || event.shiftKey ? controller.redo() : controller.undo()); }
    };
    grid.oncopy = event => { if (event.target !== grid) return; try { requireCurrent(); const data = exportSelection(false); event.clipboardData?.setData("text/plain", writeDelimited(flatClipboard(data), "\t")); event.clipboardData?.setData("text/html", writeHTMLTable(data)); event.preventDefault(); } catch (failure) { report(failure); } };
    // ClipboardEvent cannot prove a platform write succeeded, so cut uses the
    // explicit asynchronous clipboard command, which clears only after success.
    grid.oncut = event => { if (event.target !== grid) return; event.preventDefault(); report(new Error("请使用“剪切范围”：成功写入剪贴板后才清空，失败保留原稿。")); };
    grid.onpaste = event => { if (event.target !== grid) return; event.preventDefault(); const html = event.clipboardData?.getData("text/html"), text = event.clipboardData?.getData("text/plain"); run(() => pastePreview(html ? parseHTMLTable(html) : delimitedTable(text ?? "", "\t"))); };
  }
  function setupDrag(button: HTMLButtonElement, axis: "row" | "column", index: number): void {
    button.draggable = true;
    button.ondragstart = event => { if (controller.state.composing || !available()) { event.preventDefault(); return; } dragging = { axis, from: index }; event.dataTransfer?.setData("text/plain", `${axis}:${index}`); if (event.dataTransfer) event.dataTransfer.effectAllowed = "move"; };
    button.ondragover = event => { if (dragging?.axis !== axis) return; event.preventDefault(); button.classList.add("table-drop-target"); };
    button.ondragleave = () => button.classList.remove("table-drop-target");
    button.ondrop = event => { event.preventDefault(); button.classList.remove("table-drop-target"); const origin = dragging; dragging = null; if (!origin || origin.axis !== axis || origin.from === index) return; run(() => command({ kind: axis === "row" ? "moveRow" : "moveColumn", from: origin.from, to: index }, axis === "row" ? { row: index, column: head.column } : { row: head.row, column: index })); };
    button.ondragend = () => { dragging = null; surface.querySelectorAll(".table-drop-target").forEach(item => item.classList.remove("table-drop-target")); };
  }
  function exportSelection(whole: boolean): ClipboardTable {
    const range = whole ? { top: 0, left: 0, bottom: table().rows - 1, right: table().columns - 1 } : selectedRange();
    const cells = logicalTableCells(table()).filter(cell => cell.row >= range.top && cell.row <= range.bottom && cell.column >= range.left && cell.column <= range.right).map(cell => {
      const region = literal(cell); if (!region) throw new Error("所选范围含公式、链接或格式内容，尚无准确的文字导出映射；原稿保留，请使用文档导出。");
      return { row: cell.row - range.top, column: cell.column - range.left, rowspan: cell.rowspan, colspan: cell.colspan, text: region.text, header: cell.header };
    });
    return { rows: range.bottom - range.top + 1, columns: range.right - range.left + 1, cells, losses: [] };
  }
  async function copy(format: "html" | "tsv" | "csv", cut = false, whole = false): Promise<void> {
    await settle(); const basis = controller.snapshot, selection = selectionVersion, range = selectedRange(), data = exportSelection(whole), flat = flatClipboard(data);
    if (format !== "html" && data.cells.some(cell => cell.rowspan > 1 || cell.colspan > 1) && !await confirmProposal("合并格复制为平面格式", "合并格文字放在左上原点，覆盖槽为空。表头和格式不会保留。", "复制")) return;
    if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection) throw new Error("确认期间表格或选区已变化，未复制旧范围。");
    if (format === "html") {
      if (!navigator.clipboard?.write || typeof ClipboardItem === "undefined") throw new Error("当前环境不能写 HTML 剪贴板。请选择明确的 TSV/CSV 纯文本输出；尚未改稿。");
      await navigator.clipboard.write([new ClipboardItem({ "text/html": new Blob([writeHTMLTable(data)], { type: "text/html" }), "text/plain": new Blob([writeDelimited(flat, "\t")], { type: "text/plain" }) })]);
    } else { if (!navigator.clipboard?.writeText) throw new Error("当前环境没有剪贴板写权限；原稿未变。"); await navigator.clipboard.writeText(writeDelimited(flat, format === "tsv" ? "\t" : ",")); }
    if (cut) { if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection) throw new Error("已复制原选区，但草稿或选区已变化；未清空任何内容。"); await command({ kind: "clearRange", ...range }); }
    else notice.textContent = `已复制${whole ? "整表" : "所选范围"}为 ${format.toUpperCase()}，${data.rows} 行 × ${data.columns} 列。`;
  }
  async function pastePreview(data: ClipboardTable): Promise<void> {
    await settle(); const basis = controller.snapshot, selection = selectionVersion, range = selectedRange(), at = { row: range.top, column: range.left }, extendsTable = at.row + data.rows > table().rows || at.column + data.columns > table().columns;
    if ((range.bottom > range.top || range.right > range.left) && (data.rows !== range.bottom - range.top + 1 || data.columns !== range.right - range.left + 1)) throw new Error(`剪贴板 ${data.rows} 行 × ${data.columns} 列与选区尺寸不一致；请选择单格作为粘贴起点。`);
    const message = `从第 ${at.row + 1} 行第 ${at.column + 1} 列粘贴 ${data.rows} 行 × ${data.columns} 列。${extendsTable ? "需要扩展当前表格；确认后一次扩展并粘贴。" : "现有范围内一次替换，未选内容保持。"}\n${data.losses.join("\n")}\n合并格按完整拓扑写入，第一行不会自动成为表头。`;
    if (!await confirmProposal("粘贴表格范围", message, extendsTable ? "扩展并粘贴" : "粘贴", data)) return;
    if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection) throw new Error("粘贴预览已过期，保留剪贴板输入与原表。");
    await command({ kind: "pasteCells", ...at, rows: data.rows, columns: data.columns, cells: data.cells, expand: extendsTable });
  }
  async function readClipboard(): Promise<void> {
    await settle(); const basis = controller.snapshot, selection = selectionVersion;
    const check = () => { if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection) throw new Error("读取剪贴板期间目标已变化，旧内容未贴入新位置。"); };
    if (navigator.clipboard?.read) {
      const items = await navigator.clipboard.read(); let text: string | null = null;
      for (const item of items) { if (item.types.includes("text/html")) { const html = await (await item.getType("text/html")).text(); check(); await pastePreview(parseHTMLTable(html)); return; } if (item.types.includes("text/plain")) text = await (await item.getType("text/plain")).text(); }
      if (text != null) { check(); await pastePreview(delimitedTable(text, "\t")); return; }
      throw new Error("剪贴板没有受支持的 HTML 或 TSV 表格，请保留原输入并选择其它格式。");
    }
    if (!navigator.clipboard?.readText) throw new Error("当前环境不能读取剪贴板。请聚焦网格后使用系统粘贴；原稿未变。");
    const text = await navigator.clipboard.readText(); check(); await pastePreview(delimitedTable(text, "\t"));
  }
  const tool = (label: string, operation: () => Promise<void> | void) => { const button = node("button", "writing-button", label); button.type = "button"; button.onclick = () => run(operation); tools.append(button); return button; };
  tool("上方加行", () => command({ kind: "insertRow", index: selectedRange().top }));
  tool("下方加行", () => command({ kind: "insertRow", index: selectedRange().bottom + 1 }));
  tool("左侧加列", () => command({ kind: "insertColumn", index: selectedRange().left }));
  tool("右侧加列", () => command({ kind: "insertColumn", index: selectedRange().right + 1 }));
  tool("删除所选行", async () => { const range = selectedRange(); if (range.top !== range.bottom) throw new Error("请用行把手选择一行，再执行删除；未拆分为多项操作。"); await command({ kind: "removeRow", index: range.top }, { row: Math.max(0, range.top - 1), column: head.column }); });
  tool("删除所选列", async () => { const range = selectedRange(); if (range.left !== range.right) throw new Error("请用列把手选择一列，再执行删除；未拆分为多项操作。"); await command({ kind: "removeColumn", index: range.left }, { row: head.row, column: Math.max(0, range.left - 1) }); });
  for (const [label, axis, delta] of [["行上移", "row", -1], ["行下移", "row", 1], ["列左移", "column", -1], ["列右移", "column", 1]] as const) tool(label, async () => {
    const from = axis === "row" ? head.row : head.column, to = from + delta, bound = axis === "row" ? table().rows : table().columns; if (to < 0 || to >= bound) throw new Error("已在表格边界，未移动内容。");
    await command({ kind: axis === "row" ? "moveRow" : "moveColumn", from, to }, axis === "row" ? { row: to, column: head.column } : { row: head.row, column: to });
  });
  tool("合并所选格", async () => { const range = selectedRange(); if (!await confirmCurrent("合并所选单元格", "所选内容按逻辑行序全部保存在左上格，以段落分隔。原生公式、链接及格内格式保留；不兼容格式会整次拒绝。", "合并")) return; await command({ kind: "merge", ...range }, { row: range.top, column: range.left }); });
  tool("拆分当前格", async () => { if (!await confirmCurrent("拆分当前单元格", "原内容和格内格式留在左上格，新拆出的其余单元格为空。", "拆分")) return; await command({ kind: "split", ...head }); });
  tool("设首行为表头", () => command({ kind: "setHeader", rows: 1, columns: 0 }));
  tool("所选前几行设表头", () => { const range = selectedRange(); if (range.top !== 0) throw new Error("原生行表头必须从首行开始；请先选择完整的前几行。"); return command({ kind: "setHeader", rows: range.bottom + 1, columns: 0 }); });
  tool("取消表头", () => command({ kind: "setHeader", rows: 0, columns: 0 }));
  tool("整张表格转置", async () => { if ((table().headerRows ?? 0) > 0) throw new Error("原生表头仅支持整行，转置前请明确取消表头；不会静默转成数据。"); if (!await confirmCurrent("整张表格转置", "整张表格的行与列交换，跨格方向随之交换，内容与格内格式留在原逻辑单元格。无法安全交换的行列样式会整次拒绝。", "转置整表")) return; await command({ kind: "transpose" }, { row: head.column, column: head.row }); });
  tool("复制范围 HTML", () => copy("html")); tool("复制范围 TSV", () => copy("tsv")); tool("复制范围 CSV", () => copy("csv")); tool("复制整表 HTML", () => copy("html", false, true)); tool("复制整表 TSV", () => copy("tsv", false, true)); tool("复制整表 CSV", () => copy("csv", false, true)); tool("剪切范围", () => copy("html", true)); tool("剪切范围 TSV", () => copy("tsv", true)); tool("粘贴表格范围", readClipboard);
  tool("编辑此表源码", () => { requireCurrent(); options.revealSource?.(current.range); });
  surface.append(tools, gridHost, status, notice, error);
  controller.observeSurface(surface, transition => {
    if (transition && snapshot.sessionId === transition.before.sessionId && snapshot.source === transition.before.source && snapshot.serial === transition.before.serial && snapshot.generation === transition.before.generation) {
      const changes = ChangeSet.of(transition.patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), transition.before.source.length, "\n");
      const from = changes.mapPos(current.range.utf16From, -1), to = changes.mapPos(current.range.utf16To, 1);
      const matches = transition.map.structures.filter(item => item.kind === "table" && item.range.utf16From === from && item.range.utf16To === to);
      if (matches.length === 1 && matches[0].table) { current = matches[0]; snapshot = transition.after; }
      else stale = true;
    }
    updateButtons(); if (scheduled) return; scheduled = true;
    queueMicrotask(() => { scheduled = false; if (!surface.isConnected || controller.state.busy || controller.state.composing || controller.state.pending) return; if (!controller.map) { stale = true; updateButtons(); return; } if (refreshCurrent()) render(); });
  }, () => localBusy || pointerEngaged);
  render(); updateButtons(); return surface;
}

async function confirmProposal(title: string, message: string, label: string, preview?: ClipboardTable): Promise<boolean> {
  const dialog = node("dialog", "writing-dialog"), form = node("form", "writing-form"), heading = node("h2", "", title), help = node("p", "writing-help", message), buttons = node("div", "writing-actions"), cancel = node("button", "writing-button", "取消"), apply = node("button", "writing-button primary", label);
  heading.id = `table-confirm-${Math.random().toString(16).slice(2)}`; dialog.setAttribute("aria-labelledby", heading.id); cancel.type = "button"; apply.type = "submit"; buttons.append(cancel, apply); form.append(heading, help);
  if (preview) {
    const scroll = node("div", "writing-table-scroll"), table = node("table", "writing-table-grid"); table.setAttribute("aria-label", "待粘贴的完整表格内容");
    for (let row = 0; row < preview.rows; row++) { const tr = node("tr"); for (const cell of preview.cells.filter(cell => cell.row === row)) { const td = node(cell.header ? "th" : "td", "table-cell-display", cell.text || "\u00a0"); td.rowSpan = cell.rowspan; td.colSpan = cell.colspan; tr.append(td); } table.append(tr); } scroll.append(table); form.append(scroll);
  }
  form.append(buttons); dialog.append(form); document.body.append(dialog);
  return new Promise(resolve => { const finish = (accepted: boolean) => { dialog.close(); dialog.remove(); resolve(accepted); }; cancel.onclick = () => finish(false); dialog.oncancel = event => { event.preventDefault(); finish(false); }; form.onsubmit = event => { event.preventDefault(); finish(true); }; dialog.showModal(); });
}
