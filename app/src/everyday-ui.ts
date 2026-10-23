import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { disposeUiTree, message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { matchEveryday, type EverydayEntry, type EverydayCommand } from "./everyday-navigation";
import "./everyday.css";
/** Menus and command search share exact action objects and restore their trigger. */
export function openEverydayMenu(title: UiText, items: readonly EverydayCommand[], notify: (message: UiText) => void, searchable = true): void {
    const trigger = document.activeElement as HTMLElement | null;
    const triggerContainer = trigger?.closest<HTMLElement>("nav,[role=toolbar],main,section");
    const dialog = document.createElement("dialog");
    dialog.className = "everyday-dialog";
    setUiAttr(dialog, "aria-label", title);
    const heading = document.createElement("h2");
    setUiText(heading, title);
    const search = document.createElement("input");
    search.type = "search";
    setUiAttr(search, "aria-label", displayMessage("everyday.searchWithin", { p0: uiDisplayParameter(title) }));
    setUiAttr(search, "placeholder", message("everyday.searchKeyboardHint"));
    const list = document.createElement("div");
    list.className = "everyday-results";
    setUiAttr(list, "aria-label", displayMessage("everyday.resultsTitle", { title: uiDisplayParameter(title) }));
    const close = () => {
        dialog.close();
        disposeUiTree(dialog);
        dialog.remove();
        if (trigger?.isConnected)
            trigger.focus({ preventScroll: true });
        else {
            const fallback = triggerContainer?.isConnected ? triggerContainer : document.querySelector<HTMLElement>(".everyday-bar");
            if (fallback) {
                fallback.tabIndex = -1;
                fallback.focus({ preventScroll: true });
            }
        }
    };
    let selected = 0;
    const render = () => {
        disposeUiTree(list);
        list.replaceChildren();
        const matches = matchEveryday(items, search.value);
        selected = 0;
        for (const item of matches) {
            const button = document.createElement("button");
            button.type = "button";
            button.dataset.action = item.id;
            const label = document.createElement("strong");
            setUiText(label, item.label);
            const detail = document.createElement("span");
            setUiText(detail, item.unavailable ?? item.detail ?? "");
            button.append(label, detail);
            button.setAttribute("aria-disabled", String(!!item.unavailable));
            button.onclick = () => {
                if (item.unavailable) {
                    notify(uiTextValue(item.unavailable));
                    return;
                }
                close();
                void Promise.resolve().then(item.run).catch(error => notify(error instanceof Error ? errorUiText(error) : String(error)));
            };
            list.append(button);
        }
        if (!matches.length) {
            const empty = document.createElement("p");
            setUiText(empty, message("everyday.noMatches"));
            list.append(empty);
        }
    };
    const keys = (event: KeyboardEvent) => {
        if (event.isComposing || event.keyCode === 229)
            return;
        if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            close();
            return;
        }
        const buttons = [...list.querySelectorAll<HTMLButtonElement>("button")];
        if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            const focused = buttons.indexOf(document.activeElement as HTMLButtonElement);
            selected = focused < 0 ? (event.key === "ArrowDown" ? 0 : buttons.length - 1) : (focused + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
            buttons[selected]?.focus();
        }
        else if (event.key === "Enter" && document.activeElement === search) {
            event.preventDefault();
            buttons[selected]?.click();
        }
    };
    search.oninput = () => {
        if (!composing)
            render();
    };
    let composing = false;
    search.addEventListener("compositionstart", () => { composing = true; });
    search.addEventListener("compositionend", () => { composing = false; render(); });
    dialog.onkeydown = keys;
    dialog.oncancel = event => { event.preventDefault(); close(); };
    const cancel = document.createElement("button");
    cancel.type = "button";
    setUiText(cancel, message("common.close"));
    cancel.onclick = close;
    dialog.append(heading);
    if (searchable)
        dialog.append(search);
    dialog.append(list, cancel);
    document.body.append(dialog);
    render();
    dialog.showModal();
    if (searchable)
        search.focus();
    else
        list.querySelector<HTMLButtonElement>("button")?.focus();
}
export function openEverydayPicker(title: UiText, entries: readonly EverydayEntry[], notify: (message: UiText) => void): void {
    openEverydayMenu(title, entries.map(entry => ({ id: entry.key, label: entry.title, detail: entry.detail, unavailable: entry.unavailable, run: entry.open })), notify);
}
