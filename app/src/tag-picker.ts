import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { TagTree } from "./tag-tree";
export interface TagPickerOptions {
    label: UiText;
    controlKey?: string;
    value?: string;
    root?: boolean;
    excludedRoot?: string;
    changed(reference: string | null): void;
}
/** Bounded native controls keep keyboard paths usable even for a 5,000-tag chain. */
export function tagPicker(tree: TagTree, options: TagPickerOptions): HTMLElement {
    const host = document.createElement("fieldset"), legend = document.createElement("legend");
    setUiText(legend, options.label);
    host.append(legend);
    const search = document.createElement("input");
    search.type = "search";
    search.dataset.controlRole = "tag-search";
    if (options.controlKey)
        search.dataset.controlKey = JSON.stringify([options.controlKey, "search"]);
    search.className = "data-input";
    setUiAttr(search, "aria-label", displayMessage("tagPicker.searchAria", { p0: uiDisplayParameter(options.label) }));
    const select = document.createElement("select");
    select.className = "data-input";
    select.dataset.controlRole = "tag-selection";
    if (options.controlKey)
        select.dataset.controlKey = JSON.stringify([options.controlKey, "selection"]);
    setUiAttr(select, "aria-label", options.label);
    const previous = document.createElement("button"), next = document.createElement("button"), status = document.createElement("p"), path = document.createElement("p"), full = document.createElement("button");
    for (const button of [previous, next, full]) {
        button.type = "button";
        button.className = "data-button compact";
    }
    setUiText(previous, message("tagPicker.previousPage"));
    setUiText(next, message("tagPicker.nextPage"));
    setUiText(full, message("tagPicker.fullPath"));
    let offset = 0, selected = options.value ?? "";
    const render = () => {
        const page = tree.page(search.value, offset, options.excludedRoot);
        offset = page.offset;
        select.replaceChildren();
        const add = (value: string, label: UiText) => { const option = document.createElement("option"); option.value = value; setUiText(option, label); select.append(option); return option; };
        add("", options.root ? message("tagPicker.rootLevel") : message("tagPicker.directTagPlaceholder"));
        const selectedTag = tree.byRef.get(selected), eligible = selectedTag && (!options.excludedRoot || !tree.contains(options.excludedRoot, selectedTag.tagId));
        if (selected && !page.items.some(tag => tag.reference === selected))
            add(selected, eligible ? displayMessage("tagPicker.currentSelection", { p0: uiDisplayParameter(tree.label(selected)) }) : message("tagPicker.unavailableSelection")).disabled = !eligible;
        for (const item of page.items)
            add(item.reference, tree.label(item.reference));
        select.value = selected;
        setUiText(status, displayMessage("tagPicker.pageSummary", { p0: page.total, p1: page.total ? offset + 1 : 0, p2: offset + page.items.length }));
        previous.disabled = offset === 0;
        next.disabled = offset + page.items.length >= page.total;
        full.disabled = !selected || !tree.byRef.has(selected);
        setUiText(path, selected && tree.byRef.has(selected) ? tree.label(selected) : "");
    };
    search.oninput = () => { offset = 0; render(); };
    previous.onclick = () => { offset -= 100; render(); };
    next.onclick = () => { offset += 100; render(); };
    select.onchange = () => {
        if (select.isConnected === false)
            return;
        selected = select.value;
        options.changed(selected || null);
        render();
    };
    full.onclick = () => { setUiText(path, tree.label(selected, true)); };
    host.append(search, select, previous, next, status, full, path);
    render();
    return host;
}
