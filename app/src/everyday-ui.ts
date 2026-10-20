import { matchEveryday, type EverydayEntry, type EverydayCommand } from "./everyday-navigation";
import "./everyday.css";

/** Menus and command search share exact action objects and restore their trigger. */
export function openEverydayMenu(title: string, items: readonly EverydayCommand[], notify: (message: string) => void, searchable = true): void {
  const trigger = document.activeElement as HTMLElement | null;
  const triggerContainer=trigger?.closest<HTMLElement>("nav,[role=toolbar],main,section");
  const dialog = document.createElement("dialog"); dialog.className = "everyday-dialog"; dialog.setAttribute("aria-label", title);
  const heading = document.createElement("h2"); heading.textContent = title;
  const search = document.createElement("input"); search.type = "search"; search.setAttribute("aria-label", `搜索${title}`); search.placeholder = "输入名称查找，方向键选择，Enter 执行";
  const list = document.createElement("div"); list.className = "everyday-results"; list.setAttribute("aria-label", title + "结果");
  const close = () => { dialog.close(); dialog.remove(); if (trigger?.isConnected) trigger.focus({ preventScroll: true }); else { const fallback=triggerContainer?.isConnected?triggerContainer:document.querySelector<HTMLElement>(".everyday-bar");if(fallback){fallback.tabIndex=-1;fallback.focus({preventScroll:true});} } };
  let selected = 0;
  const render = () => {
    list.replaceChildren(); const matches = matchEveryday(items, search.value); selected = 0;
    for (const item of matches) {
      const button = document.createElement("button"); button.type = "button"; button.dataset.action = item.id;
      const label = document.createElement("strong"); label.textContent = item.label;
      const detail = document.createElement("span"); detail.textContent = item.unavailable ?? item.detail ?? "";
      button.append(label, detail); button.setAttribute("aria-disabled", String(!!item.unavailable));
      button.onclick = () => { if (item.unavailable) { notify(item.unavailable); return; } close(); void Promise.resolve().then(item.run).catch(error => notify(error instanceof Error ? error.message : String(error))); };
      list.append(button);
    }
    if (!matches.length) { const empty = document.createElement("p"); empty.textContent = "没有匹配项"; list.append(empty); }
  };
  const keys = (event: KeyboardEvent) => {
    if (event.isComposing || event.keyCode === 229) return;
    if(event.key==="Escape"){event.preventDefault();event.stopPropagation();close();return;}
    const buttons = [...list.querySelectorAll<HTMLButtonElement>("button")];
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); const focused = buttons.indexOf(document.activeElement as HTMLButtonElement);
      selected = focused < 0 ? (event.key === "ArrowDown" ? 0 : buttons.length - 1) : (focused + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
      buttons[selected]?.focus();
    } else if (event.key === "Enter" && document.activeElement === search) { event.preventDefault(); buttons[selected]?.click(); }
  };
  search.oninput = () => { if (!composing) render(); }; let composing = false;
  search.addEventListener("compositionstart", () => { composing = true; }); search.addEventListener("compositionend", () => { composing = false; render(); });
  dialog.onkeydown = keys; dialog.oncancel = event => { event.preventDefault(); close(); };
  const cancel = document.createElement("button"); cancel.type = "button"; cancel.textContent = "关闭"; cancel.onclick = close;
  dialog.append(heading); if (searchable) dialog.append(search); dialog.append(list, cancel); document.body.append(dialog); render(); dialog.showModal();
  if (searchable) search.focus(); else list.querySelector<HTMLButtonElement>("button")?.focus();
}

export function openEverydayPicker(title: string, entries: readonly EverydayEntry[], notify: (message: string) => void): void {
  openEverydayMenu(title, entries.map(entry => ({ id: entry.key, label: entry.title, detail: entry.detail, unavailable: entry.unavailable, run: entry.open })), notify);
}
