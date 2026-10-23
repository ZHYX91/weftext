import { disposeUiTree, replaceUiChildren, joinUiText, uiDisplayParameter } from "./ui-locale";
import { descriptionUiText } from "./ui-diagnostics";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { type WritingController, type WritingStructure, type WritingTable, type WritingTableCell, type WritingSnapshot, type TableAction } from "./writing";
import { delimitedTable, flatClipboard, parseHTMLTable, writeDelimited, writeHTMLTable, type ClipboardTable } from "./table-clipboard";
import { ChangeSet } from "@codemirror/state";
export interface TableGridOptions {
    onError?(error: Error): void;
    revealSource?(range: WritingStructure["range"]): void;
    renderedTable?: Element;
}
interface Position {
    row: number;
    column: number;
}
interface Rectangle {
    top: number;
    left: number;
    bottom: number;
    right: number;
}
/** Personal selection only, keyed by a freshly verified native source hash. */
const selections = new WeakMap<WritingController, Map<string, {
    anchor: Position;
    head: Position;
}>>();
function node<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: UiText): HTMLElementTagNameMap[K] {
    const result = document.createElement(tag);
    result.className = className;
    if (text != null)
        setUiText(result, text);
    return result;
}
export function logicalTableCells(table: WritingTable): WritingTableCell[] {
    return table.logicalCells;
}
export function tableUnavailableReason(table: WritingTable, fallback: UiText = uiMessage("tableGrid.topologyUnverified")): UiText { return table.unavailableReason ? descriptionUiText(table.unavailableReason) : fallback; }
const tableDisposers = new WeakMap<HTMLElement, () => void>();
export function disposeTableGrid(surface: HTMLElement): void { tableDisposers.get(surface)?.(); tableDisposers.delete(surface); }
export function tableCellAt(table: WritingTable, at: Position): WritingTableCell | undefined {
    return logicalTableCells(table).find(cell => cell.row <= at.row && cell.row + cell.rowspan > at.row && cell.column <= at.column && cell.column + cell.colspan > at.column);
}
/** Span closure always selects complete logical author slots. */
export function tableRectangle(table: WritingTable, anchor: Position, head: Position): Rectangle {
    const range = { top: Math.min(anchor.row, head.row), left: Math.min(anchor.column, head.column), bottom: Math.max(anchor.row, head.row), right: Math.max(anchor.column, head.column) };
    let changed = true;
    while (changed) {
        changed = false;
        for (const cell of logicalTableCells(table))
            if (cell.row <= range.bottom && cell.row + cell.rowspan > range.top && cell.column <= range.right && cell.column + cell.colspan > range.left) {
                const next = { top: Math.min(range.top, cell.row), left: Math.min(range.left, cell.column), bottom: Math.max(range.bottom, cell.row + cell.rowspan - 1), right: Math.max(range.right, cell.column + cell.colspan - 1) };
                changed ||= next.top !== range.top || next.left !== range.left || next.bottom !== range.bottom || next.right !== range.right;
                Object.assign(range, next);
            }
    }
    return range;
}
export function adjacentTableCell(table: WritingTable, current: Position, direction: "previous" | "next" | "up" | "down" | "left" | "right"): Position | null {
    const cell = tableCellAt(table, current);
    if (!cell)
        return null;
    const cells = [...logicalTableCells(table)].sort((a, b) => a.row - b.row || a.column - b.column);
    if (direction === "previous" || direction === "next") {
        const index = cells.findIndex(item => item.row === cell.row && item.column === cell.column), target = cells[index + (direction === "next" ? 1 : -1)];
        return target ? { row: target.row, column: target.column } : null;
    }
    const at = { row: direction === "up" ? cell.row - 1 : direction === "down" ? cell.row + cell.rowspan : current.row, column: direction === "left" ? cell.column - 1 : direction === "right" ? cell.column + cell.colspan : current.column };
    const target = tableCellAt(table, at);
    return target ? { row: target.row, column: target.column } : null;
}
/** Native source remains the only table model; this state stores disposable selection. */
export function createTableGrid(controller: WritingController, initial: WritingStructure, options: TableGridOptions = {}): HTMLElement {
    const surface = node("section", "writing-table-surface"), tools = node("div", "writing-table-tools"), gridHost = node("div", "writing-table-scroll"), status = node("p", "writing-help"), notice = node("p", "writing-help"), error = node("p", "writing-error");
    setUiAttr(surface, "aria-label", uiMessage("tableGrid.editLabel"));
    tools.setAttribute("role", "toolbar");
    setUiAttr(tools, "aria-label", uiMessage("tableGrid.toolsLabel"));
    error.setAttribute("role", "alert");
    error.hidden = true;
    status.setAttribute("aria-live", "polite");
    notice.setAttribute("role", "status");
    const nativeSnapshot = controller.snapshot;
    let current = initial, snapshot = controller.snapshot, rendered: WritingSnapshot | null = null, anchor: Position = { row: 0, column: 0 }, head = { ...anchor }, editing = false, updating = false, scheduled = false, localBusy = false, stale = false, dragging: {
        axis: "row" | "column";
        from: number;
    } | null = null;
    let grid: HTMLTableElement, inputs = new Map<string, HTMLTextAreaElement>(), bindings: (() => void)[] = [];
    let selectionVersion = 0, pointerEngaged = false, disposed = false, wholeCellNotice = false;
    const selectionKey = () => `${controller.snapshot.sessionId}:${controller.map?.sourceHash}:${current.range.utf16From}:${current.range.utf16To}`;
    const remembered = selections.get(controller)?.get(selectionKey()), sourceSelection = controller.sourceSelection;
    const sourceCell = logicalTableCells(initial.table!).find(cell => cell.body.utf16From <= sourceSelection.from && cell.body.utf16To >= sourceSelection.to);
    if (remembered) {
        anchor = { ...remembered.anchor };
        head = { ...remembered.head };
    }
    else if (sourceCell) {
        anchor = { row: sourceCell.row, column: sourceCell.column };
        head = { ...anchor };
    }
    const rememberSelection = () => {
        let saved = selections.get(controller);
        if (!saved) {
            saved = new Map();
            selections.set(controller, saved);
        }
        const key = selectionKey();
        saved.delete(key);
        saved.set(key, { anchor: { ...anchor }, head: { ...head } });
        if (saved.size > 128)
            saved.delete(saved.keys().next().value!);
    };
    // Mousedown can blur a focused grid before its later click starts editing.
    // Keep this exact mounted target through that gesture, including cross-table clicks.
    surface.addEventListener("pointerdown", event => {
        if (controller.state.composing && event.target !== document.activeElement) {
            event.preventDefault();
            return;
        }
        pointerEngaged = true;
        const release = () => { document.removeEventListener("pointerup", release, true); document.removeEventListener("pointercancel", release, true); setTimeout(() => { pointerEngaged = false; controller.surfaceStateChanged(); }, 0); };
        document.addEventListener("pointerup", release, { capture: true, once: true });
        document.addEventListener("pointercancel", release, { capture: true, once: true });
    }, { capture: true });
    surface.addEventListener("click", () => queueMicrotask(() => { pointerEngaged = false; controller.surfaceStateChanged(); }));
    surface.addEventListener("pointercancel", () => { pointerEngaged = false; controller.surfaceStateChanged(); });
    const key = (at: Position) => `${at.row}:${at.column}`;
    const table = () => current.table!;
    const available = () => table().topologyWritable;
    const report = (failure: unknown) => { if (disposed) return; const problem = failure instanceof Error ? failure : new UiError(errorUiText(failure)); error.hidden = false; setUiText(error, errorUiText(problem)); options.onError?.(problem); };
    const requireCurrent = () => {
        if (!surface.isConnected || stale || !controller.isSnapshotCurrent(snapshot))
            throw new UiError(uiMessage("tableGrid.sourceChanged"));
    };
    const literal = (cell: WritingTableCell) => {
        const matches = controller.map?.literalRegions.filter(region => region.range.utf16From === cell.body.utf16From && region.range.utf16To === cell.body.utf16To && region.range.raw === cell.body.raw) ?? [];
        return matches.length === 1 ? matches[0] : cell.body.raw === "" && cell.body.writable ? { range: cell.body, text: "", role: "tableCell", paragraphBreaks: [] } : null;
    };
    const refreshCurrent = () => {
        if (!controller.map)
            return false;
        // Only the certified transition below advances this surface's source basis.
        // An unrelated map after Undo/source replacement cannot renew old controls.
        if (!controller.isSnapshotCurrent(snapshot)) {
            stale = true;
            return false;
        }
        const matches = controller.map.structures.filter(item => item.kind === "table" && item.range.utf16From === current.range.utf16From);
        if (matches.length !== 1 || !matches[0].table) {
            stale = true;
            return false;
        }
        current = matches[0];
        head = { row: Math.min(head.row, table().rows - 1), column: Math.min(head.column, table().columns - 1) };
        anchor = { row: Math.min(anchor.row, table().rows - 1), column: Math.min(anchor.column, table().columns - 1) };
        return true;
    };
    const selectedRange = () => tableRectangle(table(), anchor, head);
    const paintSelection = () => {
        if (!grid)
            return;
        const range = selectedRange();
        for (const element of Array.from(grid.querySelectorAll<HTMLElement>("[data-cell-row]"))) {
            const row = Number(element.dataset.cellRow), column = Number(element.dataset.cellColumn), selected = row >= range.top && row <= range.bottom && column >= range.left && column <= range.right;
            element.setAttribute("aria-selected", String(selected));
            element.classList.toggle("table-selected", selected);
            element.classList.toggle("table-current", row === head.row && column === head.column);
        }
        const active = tableCellAt(table(), head);
        grid.setAttribute("aria-activedescendant", active ? `${grid.id}-${key(active)}` : "");
        rememberSelection();
        setUiText(status, joinUiText([displayMessage("tableGrid.selectionStatus", { p0: table().rows, p1: table().columns, p2: range.top + 1, p3: range.bottom + 1, p4: range.left + 1, p5: range.right + 1, p6: uiDisplayParameter(editing ? uiMessage("tableGrid.cellEditingHelp") : uiMessage("tableGrid.gridNavigationHelp")), p7: uiDisplayParameter(available() ? uiMessage("tableGrid.appendRowHelp") : displayMessage("tableGrid.structureReadOnly", { p0: uiDisplayParameter(tableUnavailableReason(table())) })) }), editing && wholeCellNotice ? uiMessage("tableGrid.wholeCellSelectionNotice") : ""]));
    };
    const select = (position: Position, extend = false, focus = true) => {
        const cell = tableCellAt(table(), position);
        if (!cell)
            return;
        head = { row: cell.row, column: cell.column };
        if (!extend)
            anchor = { ...head };
        selectionVersion++;
        editing = false;
        wholeCellNotice = false;
        controller.selectRange(cell.body);
        paintSelection();
        if (focus)
            grid.focus({ preventScroll: true });
        grid.querySelector<HTMLElement>(`[data-cell-row="${head.row}"][data-cell-column="${head.column}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
    };
    const updateButtons = () => {
        tools.setAttribute("aria-busy", String(localBusy));
        const unavailable = controller.state.composing || stale || !controller.map;
        for (const button of Array.from(tools.querySelectorAll<HTMLButtonElement>("button"))) {
            button.disabled = unavailable;
            button.setAttribute("aria-disabled", String(unavailable || localBusy));
        }
    };
    const run = (operation: () => Promise<void> | void) => {
        if (localBusy)
            return;
        localBusy = true;
        updateButtons();
        error.hidden = true;
        setUiText(notice, "");
        Promise.resolve().then(operation).catch(report).finally(() => { localBusy = false; updateButtons(); controller.surfaceStateChanged(); });
    };
    const settle = async () => {
        requireCurrent();
        if (controller.state.composing)
            throw new UiError(uiMessage("tableGrid.compositionPending"));
        await controller.flush();
        if (!refreshCurrent())
            throw new UiError(uiMessage("tableGrid.mappingUnavailable"));
    };
    const confirmCurrent = async (title: UiText, message: UiText, label: UiText): Promise<boolean> => {
        await settle();
        const basis = controller.snapshot, selection = selectionVersion;
        const accepted = await confirmProposal(title, message, label);
        if (!accepted)
            return false;
        if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection)
            throw new UiError(uiMessage("tableGrid.proposalExpired"));
        return true;
    };
    const command = async (change: TableAction, next: Position = head) => {
        await settle();
        if (!available())
            throw new UiError(tableUnavailableReason(table(), uiMessage("tableGrid.structureChangeUnsafe")));
        const beforeRange = selectedRange(), rowSelected = beforeRange.top === beforeRange.bottom && beforeRange.left === 0 && beforeRange.right === table().columns - 1, columnSelected = beforeRange.left === beforeRange.right && beforeRange.top === 0 && beforeRange.bottom === table().rows - 1;
        editing = false;
        grid.focus({ preventScroll: true });
        await controller.execute(() => { requireCurrent(); return { kind: "tableEdit", range: current.range, action: change }; });
        if (!refreshCurrent())
            throw new UiError(uiMessage("tableGrid.changeAwaitingPreview"));
        anchor = { ...next };
        head = { ...next };
        selectionVersion++;
        render(true);
        select(head);
        if (change.kind === "moveRow" && rowSelected) {
            anchor = { row: next.row, column: 0 };
            head = { row: next.row, column: table().columns - 1 };
            paintSelection();
        }
        else if (change.kind === "moveColumn" && columnSelected) {
            anchor = { row: 0, column: next.column };
            head = { row: table().rows - 1, column: next.column };
            paintSelection();
        }
    };
    const edit = async (position = head, selectWhole = false) => {
        await settle();
        render();
        const cell = tableCellAt(table(), position);
        if (!cell)
            return;
        const input = inputs.get(key(cell));
        if (!input) {
            select(cell);
            throw new UiError(uiMessage("tableGrid.nativeCellUseSource"));
        }
        anchor = { row: cell.row, column: cell.column };
        head = { ...anchor };
        selectionVersion++;
        editing = true;
        wholeCellNotice = selectWhole;
        input.hidden = false;
        input.parentElement!.classList.add("table-editing");
        input.focus({ preventScroll: true });
        input.setSelectionRange(selectWhole ? 0 : input.value.length, input.value.length);
        paintSelection();
    };
    const finishText = async () => { await settle(); editing = false; grid.focus({ preventScroll: true }); render(true); paintSelection(); };
    const go = async (direction: "previous" | "next" | "up" | "down", enter = false) => {
        await finishText();
        const target = adjacentTableCell(table(), head, direction);
        if (target) {
            select(target);
            if (enter)
                await edit();
            return;
        }
        if (enter && direction === "down") {
            const next = { row: table().rows, column: head.column };
            await command({ kind: "insertRow", index: table().rows }, next);
            await edit(next);
        }
        else if (direction === "next" || direction === "previous") {
            // The native Tab boundary stays reachable outside the table.
            const candidates = Array.from(document.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]')).filter(item => item.getClientRects().length && !surface.contains(item));
            const candidate = direction === "next" ? candidates.find(item => !!(surface.compareDocumentPosition(item) & Node.DOCUMENT_POSITION_FOLLOWING)) : candidates.filter(item => !!(surface.compareDocumentPosition(item) & Node.DOCUMENT_POSITION_PRECEDING)).at(-1);
            candidate?.focus();
        }
    };
    function render(force = false): void {
        if (disposed || updating || stale || !controller.map)
            return;
        if (!force && rendered && controller.isSnapshotCurrent(rendered)) {
            paintSelection();
            return;
        }
        if (editing && document.activeElement instanceof HTMLTextAreaElement && surface.contains(document.activeElement))
            return;
        updating = true;
        bindings.splice(0).forEach(dispose => dispose());
        inputs = new Map();
        const previousFocus = grid && document.activeElement === grid, previousScroll = { top: gridHost.scrollTop, left: gridHost.scrollLeft };
        grid = node("table", "writing-table-grid");
        grid.id = `writing-grid-${Math.random().toString(16).slice(2)}`;
        grid.tabIndex = 0;
        grid.setAttribute("role", "grid");
        setUiAttr(grid, "aria-label", uiMessage("tableGrid.selectionLabel"));
        grid.setAttribute("aria-multiselectable", "true");
        grid.setAttribute("aria-rowcount", String(table().rows));
        grid.setAttribute("aria-colcount", String(table().columns));
        const nativeCells = Array.from(options.renderedTable?.querySelectorAll("td,th") ?? []), cells = logicalTableCells(table()), tbody = node("tbody");
        const header = node("tr");
        header.append(node("th"));
        for (let column = 0; column < table().columns; column++) {
            const th = node("th", "table-handle"), button = node("button", "writing-button", `${column + 1}`);
            button.type = "button";
            setUiAttr(button, "aria-label", displayMessage("tableGrid.selectColumn", { p0: column + 1 }));
            button.onclick = () => {
                void finishText().then(() => {
                    anchor = { row: 0, column };
                    head = { row: table().rows - 1, column };
                    selectionVersion++;
                    const cell = tableCellAt(table(), head);
                    if (cell)
                        controller.selectRange(cell.body);
                    paintSelection();
                    grid.focus();
                }).catch(report);
            };
            setupDrag(button, "column", column);
            th.append(button);
            header.append(th);
        }
        tbody.append(header);
        for (let row = 0; row < table().rows; row++) {
            const tr = node("tr"), handle = node("th", "table-handle"), button = node("button", "writing-button", `${row + 1}`);
            button.type = "button";
            setUiAttr(button, "aria-label", displayMessage("tableGrid.selectRow", { p0: row + 1 }));
            button.onclick = () => {
                void finishText().then(() => {
                    anchor = { row, column: 0 };
                    head = { row, column: table().columns - 1 };
                    selectionVersion++;
                    const cell = tableCellAt(table(), head);
                    if (cell)
                        controller.selectRange(cell.body);
                    paintSelection();
                    grid.focus();
                }).catch(report);
            };
            setupDrag(button, "row", row);
            handle.append(button);
            tr.append(handle);
            for (const cell of cells.filter(item => item.row === row).sort((a, b) => a.column - b.column)) {
                const td = node(cell.header ? "th" : "td", "writing-table-cell");
                td.rowSpan = cell.rowspan;
                td.colSpan = cell.colspan;
                td.dataset.cellRow = String(cell.row);
                td.dataset.cellColumn = String(cell.column);
                td.id = `${grid.id}-${key(cell)}`;
                td.setAttribute("role", "gridcell");
                td.setAttribute("aria-rowindex", String(cell.row + 1));
                td.setAttribute("aria-colindex", String(cell.column + 1));
                const region = literal(cell), display = node("div", "table-cell-display");
                if (region?.range.writable) {
                    setUiText(display, region.text || "\u00a0");
                    const input = node("textarea", "literal-input table-cell-input");
                    input.rows = 1;
                    input.hidden = true;
                    setUiAttr(input, "aria-label", displayMessage("tableGrid.cellTextLabel", { p0: cell.row + 1, p1: cell.column + 1 }));
                    bindings.push(controller.bindLiteral(input, region));
                    inputs.set(key(cell), input);
                    input.addEventListener("keydown", event => {
                        if (event.isComposing || event.keyCode === 229 || controller.state.composing)
                            return;
                        if (event.key === "Tab" || event.key === "Escape" || event.key === "Enter" && !event.shiftKey && !(event.ctrlKey || event.metaKey || event.altKey)) {
                            event.preventDefault();
                            event.stopImmediatePropagation();
                            run(() => event.key === "Escape" ? finishText() : go(event.key === "Tab" ? event.shiftKey ? "previous" : "next" : "down", true));
                        }
                    }, { capture: true });
                    td.append(display, input);
                }
                else {
                    const native = controller.isSnapshotCurrent(nativeSnapshot) ? nativeCells[cells.indexOf(cell)] : null;
                    if (region)
                        setUiText(display, region.text || "\u00a0");
                    else if (native) {
                        const clone = native.cloneNode(true) as HTMLElement, originalLinks = Array.from(native.querySelectorAll<HTMLAnchorElement>("a"));
                        clone.querySelectorAll<HTMLTextAreaElement>("textarea").forEach(input => input.replaceWith(uiTextNode(input.value)));
                        clone.querySelectorAll("button,input,[contenteditable]").forEach(element => element.remove());
                        clone.querySelectorAll<HTMLAnchorElement>("a").forEach((link, index) => {
                            link.onclick = event => {
                                event.preventDefault();
                                event.stopPropagation();
                                if (!controller.isSnapshotCurrent(nativeSnapshot)) {
                                    report(new UiError(uiMessage("tableGrid.linkProjectionExpired")));
                                    return;
                                }
                                originalLinks[index]?.click();
                            };
                        });
                        while (clone.firstChild)
                            display.append(clone.firstChild);
                    }
                    else
                        setUiText(display, uiMessage("tableGrid.nativeReadOnlyContent"));
                    setUiAttr(td, "title", uiMessage("tableGrid.cellSourceUnavailable"));
                    td.append(display);
                }
                td.addEventListener("click", event => {
                    if (event.target instanceof HTMLTextAreaElement || controller.state.composing)
                        return;
                    if (event.shiftKey) {
                        run(async () => { await finishText(); select(cell, true); });
                    }
                    else
                        run(async () => {
                            await finishText();
                            select(cell);
                            if (region?.range.writable)
                                await edit(cell, true);
                        });
                });
                td.addEventListener("pointerdown", event => {
                    if (!(event.ctrlKey || event.metaKey) || controller.state.composing)
                        return;
                    event.preventDefault();
                    run(async () => { await finishText(); select(cell, event.shiftKey); });
                });
                tr.append(td);
            }
            tbody.append(tr);
        }
        grid.append(tbody);
        replaceUiChildren(gridHost, grid);
        gridHost.scrollTop = previousScroll.top;
        gridHost.scrollLeft = previousScroll.left;
        rendered = controller.snapshot;
        updating = false;
        paintSelection();
        if (previousFocus)
            grid.focus({ preventScroll: true });
        grid.onkeydown = event => {
            if (event.target !== grid || event.isComposing || event.keyCode === 229 || controller.state.composing)
                return;
            const directions = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" } as const;
            if (event.key in directions) {
                event.preventDefault();
                const target = adjacentTableCell(table(), head, directions[event.key as keyof typeof directions]);
                if (target)
                    select(target, event.shiftKey);
            }
            else if (["Enter", "F2"].includes(event.key)) {
                event.preventDefault();
                run(() => edit());
            }
            else if (event.key === "Tab") {
                event.preventDefault();
                run(() => go(event.shiftKey ? "previous" : "next"));
            }
            else if (["Delete", "Backspace"].includes(event.key)) {
                event.preventDefault();
                run(() => command({ kind: "clearRange", ...selectedRange() }));
            }
            else if (event.key === "Escape") {
                dragging = null;
                surface.querySelectorAll(".table-drop-target").forEach(item => item.classList.remove("table-drop-target"));
            }
            else if ((event.ctrlKey || event.metaKey) && ["z", "y"].includes(event.key.toLowerCase())) {
                event.preventDefault();
                run(() => event.key.toLowerCase() === "y" || event.shiftKey ? controller.redo() : controller.undo());
            }
        };
        grid.oncopy = event => {
            if (event.target !== grid)
                return;
            try {
                requireCurrent();
                const data = exportSelection(false);
                event.clipboardData?.setData("text/plain", writeDelimited(flatClipboard(data), "\t"));
                event.clipboardData?.setData("text/html", writeHTMLTable(data));
                event.preventDefault();
            }
            catch (failure) {
                report(failure);
            }
        };
        // ClipboardEvent cannot prove a platform write succeeded, so cut uses the
        // explicit asynchronous clipboard command, which clears only after success.
        grid.oncut = event => {
            if (event.target !== grid)
                return;
            event.preventDefault();
            report(new UiError(uiMessage("tableGrid.useExplicitCut")));
        };
        grid.onpaste = event => {
            if (event.target !== grid)
                return;
            event.preventDefault();
            const html = event.clipboardData?.getData("text/html"), text = event.clipboardData?.getData("text/plain");
            run(() => pastePreview(html ? parseHTMLTable(html) : delimitedTable(text ?? "", "\t")));
        };
    }
    function setupDrag(button: HTMLButtonElement, axis: "row" | "column", index: number): void {
        button.draggable = true;
        button.ondragstart = event => {
            if (controller.state.composing || !available()) {
                event.preventDefault();
                return;
            }
            dragging = { axis, from: index };
            event.dataTransfer?.setData("text/plain", `${axis}:${index}`);
            if (event.dataTransfer)
                event.dataTransfer.effectAllowed = "move";
        };
        button.ondragover = event => {
            if (dragging?.axis !== axis)
                return;
            event.preventDefault();
            button.classList.add("table-drop-target");
        };
        button.ondragleave = () => button.classList.remove("table-drop-target");
        button.ondrop = event => {
            event.preventDefault();
            button.classList.remove("table-drop-target");
            const origin = dragging;
            dragging = null;
            if (!origin || origin.axis !== axis || origin.from === index)
                return;
            run(() => command({ kind: axis === "row" ? "moveRow" : "moveColumn", from: origin.from, to: index }, axis === "row" ? { row: index, column: head.column } : { row: head.row, column: index }));
        };
        button.ondragend = () => { dragging = null; surface.querySelectorAll(".table-drop-target").forEach(item => item.classList.remove("table-drop-target")); };
    }
    function exportSelection(whole: boolean): ClipboardTable {
        const range = whole ? { top: 0, left: 0, bottom: table().rows - 1, right: table().columns - 1 } : selectedRange();
        const cells = logicalTableCells(table()).filter(cell => cell.row >= range.top && cell.row <= range.bottom && cell.column >= range.left && cell.column <= range.right).map(cell => {
            const region = literal(cell);
            if (!region)
                throw new UiError(uiMessage("tableGrid.selectionTextExportUnavailable"));
            return { row: cell.row - range.top, column: cell.column - range.left, rowspan: cell.rowspan, colspan: cell.colspan, text: region.text, header: cell.header };
        });
        return { rows: range.bottom - range.top + 1, columns: range.right - range.left + 1, cells, losses: [] };
    }
    async function copy(format: "html" | "tsv" | "csv", cut = false, whole = false): Promise<void> {
        await settle();
        const basis = controller.snapshot, selection = selectionVersion, range = selectedRange(), data = exportSelection(whole), flat = flatClipboard(data);
        if (format !== "html" && data.cells.some(cell => cell.rowspan > 1 || cell.colspan > 1) && !await confirmProposal(uiMessage("tableGrid.flattenCopyTitle"), uiMessage("tableGrid.flattenCopyHelp"), uiMessage("tableGrid.copy")))
            return;
        if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection)
            throw new UiError(uiMessage("tableGrid.copyConfirmationExpired"));
        if (format === "html") {
            if (!navigator.clipboard?.write || typeof ClipboardItem === "undefined")
                throw new UiError(uiMessage("tableGrid.htmlClipboardUnavailable"));
            await navigator.clipboard.write([new ClipboardItem({ "text/html": new Blob([writeHTMLTable(data)], { type: "text/html" }), "text/plain": new Blob([writeDelimited(flat, "\t")], { type: "text/plain" }) })]);
        }
        else {
            if (!navigator.clipboard?.writeText)
                throw new UiError(uiMessage("tableGrid.clipboardWriteUnavailable"));
            await navigator.clipboard.writeText(writeDelimited(flat, format === "tsv" ? "\t" : ","));
        }
        if (cut) {
            if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection)
                throw new UiError(uiMessage("tableGrid.cutTargetChanged"));
            await command({ kind: "clearRange", ...range });
        }
        else
            setUiText(notice, displayMessage("tableGrid.copyComplete", { p0: uiDisplayParameter(whole ? uiMessage("tableGrid.wholeTableScope") : uiMessage("tableGrid.selectedRangeScope")), p1: uiDisplayParameter(format.toUpperCase()), p2: data.rows, p3: data.columns }));
    }
    async function pastePreview(data: ClipboardTable): Promise<void> {
        await settle();
        const basis = controller.snapshot, selection = selectionVersion, range = selectedRange(), at = { row: range.top, column: range.left }, extendsTable = at.row + data.rows > table().rows || at.column + data.columns > table().columns;
        if ((range.bottom > range.top || range.right > range.left) && (data.rows !== range.bottom - range.top + 1 || data.columns !== range.right - range.left + 1))
            throw new UiError(displayMessage("tableGrid.pasteSelectionSizeMismatch", { p0: data.rows, p1: data.columns }));
        const message = displayMessage("tableGrid.pastePreviewDescription", { p0: at.row + 1, p1: at.column + 1, p2: data.rows, p3: data.columns, p4: uiDisplayParameter(extendsTable ? uiMessage("tableGrid.pasteRequiresExpansion") : uiMessage("tableGrid.pasteWithinTable")), p5: joinUiText(data.losses, "\n") });
        if (!await confirmProposal(uiMessage("tableGrid.pasteRange"), message, extendsTable ? uiMessage("tableGrid.expandAndPaste") : uiMessage("tableGrid.paste"), data))
            return;
        if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection)
            throw new UiError(uiMessage("tableGrid.pastePreviewExpired"));
        await command({ kind: "pasteCells", ...at, rows: data.rows, columns: data.columns, cells: data.cells, expand: extendsTable });
    }
    async function readClipboard(): Promise<void> {
        await settle();
        const basis = controller.snapshot, selection = selectionVersion;
        const check = () => {
            if (!controller.isSnapshotCurrent(basis) || selectionVersion !== selection)
                throw new UiError(uiMessage("tableGrid.clipboardReadTargetChanged"));
        };
        if (navigator.clipboard?.read) {
            const items = await navigator.clipboard.read();
            let text: string | null = null;
            for (const item of items) {
                if (item.types.includes("text/html")) {
                    const html = await (await item.getType("text/html")).text();
                    check();
                    await pastePreview(parseHTMLTable(html));
                    return;
                }
                if (item.types.includes("text/plain"))
                    text = await (await item.getType("text/plain")).text();
            }
            if (text != null) {
                check();
                await pastePreview(delimitedTable(text, "\t"));
                return;
            }
            throw new UiError(uiMessage("tableGrid.clipboardFormatUnsupported"));
        }
        if (!navigator.clipboard?.readText)
            throw new UiError(uiMessage("tableGrid.clipboardReadUnavailable"));
        const text = await navigator.clipboard.readText();
        check();
        await pastePreview(delimitedTable(text, "\t"));
    }
    const tool = (label: UiText, operation: () => Promise<void> | void) => { const button = node("button", "writing-button", label); button.type = "button"; button.onclick = () => run(operation); tools.append(button); return button; };
    tool(uiMessage("tableGrid.insertRowAbove"), () => command({ kind: "insertRow", index: selectedRange().top }));
    tool(uiMessage("tableGrid.insertRowBelow"), () => command({ kind: "insertRow", index: selectedRange().bottom + 1 }));
    tool(uiMessage("tableGrid.insertColumnLeft"), () => command({ kind: "insertColumn", index: selectedRange().left }));
    tool(uiMessage("tableGrid.insertColumnRight"), () => command({ kind: "insertColumn", index: selectedRange().right + 1 }));
    tool(uiMessage("tableGrid.deleteSelectedRow"), async () => {
        const range = selectedRange();
        if (range.top !== range.bottom)
            throw new UiError(uiMessage("tableGrid.deleteRequiresOneRow"));
        await command({ kind: "removeRow", index: range.top }, { row: Math.max(0, range.top - 1), column: head.column });
    });
    tool(uiMessage("tableGrid.deleteSelectedColumn"), async () => {
        const range = selectedRange();
        if (range.left !== range.right)
            throw new UiError(uiMessage("tableGrid.deleteRequiresOneColumn"));
        await command({ kind: "removeColumn", index: range.left }, { row: head.row, column: Math.max(0, range.left - 1) });
    });
    for (const [label, axis, delta] of [[uiMessage("tableGrid.moveRowUp"), "row", -1], [uiMessage("tableGrid.moveRowDown"), "row", 1], [uiMessage("tableGrid.moveColumnLeft"), "column", -1], [uiMessage("tableGrid.moveColumnRight"), "column", 1]] as const)
        tool(label, async () => {
            const from = axis === "row" ? head.row : head.column, to = from + delta, bound = axis === "row" ? table().rows : table().columns;
            if (to < 0 || to >= bound)
                throw new UiError(uiMessage("tableGrid.moveAtBoundary"));
            await command({ kind: axis === "row" ? "moveRow" : "moveColumn", from, to }, axis === "row" ? { row: to, column: head.column } : { row: head.row, column: to });
        });
    tool(uiMessage("tableGrid.mergeSelectedCells"), async () => {
        const range = selectedRange();
        if (!await confirmCurrent(uiMessage("tableGrid.mergeSelectedCellsTitle"), uiMessage("tableGrid.mergeHelp"), uiMessage("tableGrid.merge")))
            return;
        await command({ kind: "merge", ...range }, { row: range.top, column: range.left });
    });
    tool(uiMessage("tableGrid.splitCurrentCell"), async () => {
        if (!await confirmCurrent(uiMessage("tableGrid.splitCurrentCellTitle"), uiMessage("tableGrid.splitHelp"), uiMessage("tableGrid.split")))
            return;
        await command({ kind: "split", ...head });
    });
    tool(uiMessage("tableGrid.firstRowAsHeader"), () => command({ kind: "setHeader", rows: 1, columns: 0 }));
    tool(uiMessage("tableGrid.leadingRowsAsHeader"), () => {
        const range = selectedRange();
        if (range.top !== 0)
            throw new UiError(uiMessage("tableGrid.headerMustStartAtFirstRow"));
        return command({ kind: "setHeader", rows: range.bottom + 1, columns: 0 });
    });
    tool(uiMessage("tableGrid.removeHeader"), () => command({ kind: "setHeader", rows: 0, columns: 0 }));
    tool(uiMessage("tableGrid.transposeWholeTable"), async () => {
        if ((table().headerRows ?? 0) > 0)
            throw new UiError(uiMessage("tableGrid.transposeRequiresNoHeader"));
        if (!await confirmCurrent(uiMessage("tableGrid.transposeWholeTable"), uiMessage("tableGrid.transposeHelp"), uiMessage("tableGrid.confirmTranspose")))
            return;
        await command({ kind: "transpose" }, { row: head.column, column: head.row });
    });
    tool(uiMessage("tableGrid.copyRangeHtml"), () => copy("html"));
    tool(uiMessage("tableGrid.copyRangeTsv"), () => copy("tsv"));
    tool(uiMessage("tableGrid.copyRangeCsv"), () => copy("csv"));
    tool(uiMessage("tableGrid.copyWholeTableHtml"), () => copy("html", false, true));
    tool(uiMessage("tableGrid.copyWholeTableTsv"), () => copy("tsv", false, true));
    tool(uiMessage("tableGrid.copyWholeTableCsv"), () => copy("csv", false, true));
    tool(uiMessage("tableGrid.cutRange"), () => copy("html", true));
    tool(uiMessage("tableGrid.cutRangeTsv"), () => copy("tsv", true));
    tool(uiMessage("tableGrid.pasteRange"), readClipboard);
    tool(uiMessage("tableGrid.editSource"), () => { requireCurrent(); options.revealSource?.(current.range); });
    surface.append(tools, gridHost, status, notice, error);
    const stopObserving = controller.observeSurface(surface, transition => {
        if (disposed) return;
        if (transition && snapshot.sessionId === transition.before.sessionId && snapshot.source === transition.before.source && snapshot.serial === transition.before.serial && snapshot.generation === transition.before.generation) {
            const changes = ChangeSet.of(transition.patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), transition.before.source.length, "\n");
            const from = changes.mapPos(current.range.utf16From, -1), to = changes.mapPos(current.range.utf16To, 1);
            const matches = transition.map.structures.filter(item => item.kind === "table" && item.range.utf16From === from && item.range.utf16To === to);
            if (matches.length === 1 && matches[0].table) {
                current = matches[0];
                snapshot = transition.after;
            }
            else
                stale = true;
        }
        updateButtons();
        if (scheduled)
            return;
        scheduled = true;
        queueMicrotask(() => {
            scheduled = false;
            if (disposed || !surface.isConnected || controller.state.busy || controller.state.composing || controller.state.pending)
                return;
            if (!controller.map) {
                stale = true;
                updateButtons();
                return;
            }
            if (refreshCurrent())
                render();
        });
    }, { basis: () => snapshot, active: () => !stale && (localBusy || pointerEngaged) });
    tableDisposers.set(surface, () => { if (disposed) return; disposed = true; stale = true; stopObserving(); bindings.splice(0).forEach(stop => stop()); inputs.clear(); disposeUiTree(surface); });
    render();
    updateButtons();
    return surface;
}
async function confirmProposal(title: UiText, message: UiText, label: UiText, preview?: ClipboardTable): Promise<boolean> {
    const dialog = node("dialog", "writing-dialog"), form = node("form", "writing-form"), heading = node("h2", "", title), help = node("p", "writing-help", message), buttons = node("div", "writing-actions"), cancel = node("button", "writing-button", uiMessage("common.cancel")), apply = node("button", "writing-button primary", label);
    heading.id = `table-confirm-${Math.random().toString(16).slice(2)}`;
    dialog.setAttribute("aria-labelledby", heading.id);
    cancel.type = "button";
    apply.type = "submit";
    buttons.append(cancel, apply);
    form.append(heading, help);
    if (preview) {
        const scroll = node("div", "writing-table-scroll"), table = node("table", "writing-table-grid");
        setUiAttr(table, "aria-label", uiMessage("tableGrid.pastePreviewLabel"));
        for (let row = 0; row < preview.rows; row++) {
            const tr = node("tr");
            for (const cell of preview.cells.filter(cell => cell.row === row)) {
                const td = node(cell.header ? "th" : "td", "table-cell-display", cell.text || "\u00a0");
                td.rowSpan = cell.rowspan;
                td.colSpan = cell.colspan;
                tr.append(td);
            }
            table.append(tr);
        }
        scroll.append(table);
        form.append(scroll);
    }
    form.append(buttons);
    dialog.append(form);
    document.body.append(dialog);
    return new Promise(resolve => { let settled = false; const finish = (accepted: boolean) => { if (settled) return; settled = true; dialog.close(); disposeUiTree(dialog); dialog.remove(); resolve(accepted); }; cancel.onclick = () => finish(false); dialog.oncancel = event => { event.preventDefault(); finish(false); }; form.onsubmit = event => { event.preventDefault(); finish(true); }; dialog.showModal(); });
}
