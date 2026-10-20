/** Private recovery of author-entered note proposals, never a copy of their source. */
export class NoteInputRecovery {
  private records = new Map<string, { id: string; value: string }[]>();
  constructor(private storage?: Pick<Storage, "getItem" | "setItem">) {}
  private key(audience: string) { return `weftext-notes-new-input:${encodeURIComponent(audience)}`; }
  private list(audience: string) {
    if (!this.records.has(audience)) {
      let entries: { id: string; value: string }[] = [];
      try { const raw: unknown = JSON.parse(this.storage?.getItem(this.key(audience)) ?? "[]"); if (Array.isArray(raw) && raw.every(entry => typeof entry?.id === "string" && typeof entry?.value === "string")) entries = raw; } catch { /* Memory remains available if browser storage is blocked. */ }
      this.records.set(audience, entries);
    }
    return this.records.get(audience)!;
  }
  private persist(audience: string) { try { this.storage?.setItem(this.key(audience), JSON.stringify(this.list(audience))); } catch { /* Do not discard the live recovery input. */ } }
  count(audience: string): number { return this.list(audience).length; }
  add(audience: string, value: string): void { if (!audience || !value) return; this.list(audience).push({ id: crypto.randomUUID(), value }); this.persist(audience); }
  render(host: HTMLElement, audience: string, changed: () => void = () => {}): void {
    host.replaceChildren();
    const entries = this.list(audience); if (!entries.length) return;
    const title = document.createElement("h3"); title.textContent = "未应用的注释新输入"; host.append(title);
    const help = document.createElement("p"); help.className = "document-notes-help"; help.textContent = "这里只保留本账号新输入的文字。可继续整理并复制；不会自动写入当前或其它笔记。"; host.append(help);
    for (const entry of entries) {
      const row = document.createElement("section"), input = document.createElement("textarea"), status = document.createElement("p"); row.className = "document-notes-card"; input.className = "document-notes-input"; input.rows = 5; input.readOnly = true; input.value = entry.value; input.setAttribute("aria-label", "保留的本次新输入"); status.setAttribute("role", "status");
      const action = (text: string, work: () => void | Promise<void>) => { const button = document.createElement("button"); button.type = "button"; button.className = "document-notes-button"; button.textContent = text; button.onclick = () => { Promise.resolve().then(work).catch(error => { status.textContent = error instanceof Error ? error.message : String(error); }); }; return button; };
      input.oninput = () => { entry.value = input.value; this.persist(audience); };
      row.append(input, action("恢复这份输入继续编辑", () => { input.readOnly = false; input.focus(); status.textContent = "已恢复为独立输入；整理后请明确复制到获权笔记。"; }), action("复制保留的新输入", async () => { input.focus(); input.select(); if (!navigator.clipboard?.writeText) { status.textContent = "文字已选中，请用系统复制。"; return; } await navigator.clipboard.writeText(input.value); status.textContent = "已复制；恢复输入仍保留。"; }), action("已取回，移除此份", () => { const at = entries.indexOf(entry); if (at >= 0) entries.splice(at, 1); this.persist(audience); row.remove(); if (!entries.length) host.replaceChildren(); changed(); }), status); host.append(row);
    }
  }
}
