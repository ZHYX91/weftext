import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Private recovery of author-entered note proposals, never a copy of their source. */
export class NoteInputRecovery {
    private records = new Map<string, {
        id: string;
        value: string;
    }[]>();
    constructor(private storage?: Pick<Storage, "getItem" | "setItem">) { }
    private key(audience: string) { return `weftext-notes-new-input:${encodeURIComponent(audience)}`; }
    private list(audience: string) {
        if (!this.records.has(audience)) {
            let entries: {
                id: string;
                value: string;
            }[] = [];
            try {
                const raw: unknown = JSON.parse(this.storage?.getItem(this.key(audience)) ?? "[]");
                if (Array.isArray(raw) && raw.every(entry => typeof entry?.id === "string" && typeof entry?.value === "string"))
                    entries = raw;
            }
            catch { /* Memory remains available if browser storage is blocked. */ }
            this.records.set(audience, entries);
        }
        return this.records.get(audience)!;
    }
    private persist(audience: string) {
        try {
            this.storage?.setItem(this.key(audience), JSON.stringify(this.list(audience)));
        }
        catch { /* Do not discard the live recovery input. */ }
    }
    count(audience: string): number { return this.list(audience).length; }
    add(audience: string, value: string): void {
        if (!audience || !value)
            return;
        this.list(audience).push({ id: crypto.randomUUID(), value });
        this.persist(audience);
    }
    render(host: HTMLElement, audience: string, changed: () => void = () => { }): void {
        host.replaceChildren();
        const entries = this.list(audience);
        if (!entries.length)
            return;
        const title = document.createElement("h3");
        setUiText(title, uiMessage("noteInputRecovery.title"));
        host.append(title);
        const help = document.createElement("p");
        help.className = "document-notes-help";
        setUiText(help, uiMessage("noteInputRecovery.description"));
        host.append(help);
        for (const entry of entries) {
            const row = document.createElement("section"), input = document.createElement("textarea"), status = document.createElement("p");
            row.className = "document-notes-card";
            input.className = "document-notes-input";
            input.rows = 5;
            input.readOnly = true;
            input.value = entry.value;
            setUiAttr(input, "aria-label", uiMessage("noteInputRecovery.retainedNewInput"));
            status.setAttribute("role", "status");
            const action = (text: UiText, work: () => void | Promise<void>) => { const button = document.createElement("button"); button.type = "button"; button.className = "document-notes-button"; setUiText(button, text); button.onclick = () => { Promise.resolve().then(work).catch(error => { setUiText(status, errorUiText(error)); }); }; return button; };
            input.oninput = () => { entry.value = input.value; this.persist(audience); };
            row.append(input, action(uiMessage("noteInputRecovery.restoreForEditing"), () => { input.readOnly = false; input.focus(); setUiText(status, uiMessage("noteInputRecovery.restoredIndependentInput")); }), action(uiMessage("noteInputRecovery.copyRetainedInput"), async () => {
                input.focus();
                input.select();
                if (!navigator.clipboard?.writeText) {
                    setUiText(status, uiMessage("noteInputRecovery.selectedUseSystemCopy"));
                    return;
                }
                await navigator.clipboard.writeText(input.value);
                setUiText(status, uiMessage("noteInputRecovery.copiedInputRetained"));
            }), action(uiMessage("noteInputRecovery.retrievedRemoveRecord"), () => {
                const at = entries.indexOf(entry);
                if (at >= 0)
                    entries.splice(at, 1);
                this.persist(audience);
                row.remove();
                if (!entries.length)
                    host.replaceChildren();
                changed();
            }), status);
            host.append(row);
        }
    }
}
