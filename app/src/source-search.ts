import { EditorSelection, type EditorState } from "@codemirror/state";
import { EditorView, getDialog, runScopeHandlers, showDialog, type Panel, type ViewUpdate } from "@codemirror/view";
import { closeSearchPanel, findNext, findPrevious, getSearchQuery, replaceAll, replaceNext, search, SearchQuery, selectMatches, setSearchQuery } from "@codemirror/search";
import { LocaleScope, message, uiTextValue } from "./ui-locale";

/** Library phrase identities stay fixed; their text comes from the same product catalog. */
const phraseKeys = {
    Find: "editor.cmFind", Replace: "editor.cmReplaceInput", next: "editor.cmNext", previous: "editor.cmPrevious", all: "editor.cmAllMatches",
    "match case": "editor.cmMatchCase", regexp: "editor.cmRegexp", "by word": "editor.cmWholeWord", replace: "editor.cmReplaceAction", "replace all": "editor.cmReplaceAll", close: "editor.cmClose",
    "Go to line": "editor.cmGoToLine", go: "editor.cmGo", "replaced match on line $": "editor.cmReplacedMatchLine", "replaced $ matches": "editor.cmReplacedMatchCount",
    "current match": "editor.cmCurrentMatch", "on line": "editor.cmOnLine", "Selection deleted": "editor.cmSelectionDeleted",
} as const;

export function sourceSearchPhrases(): Record<string, string> {
    const phrases: Record<string, string> = {};
    for (const [phrase, key] of Object.entries(phraseKeys)) Object.defineProperty(phrases, phrase, { enumerable: true, get: () => uiTextValue(message(key)) });
    // CodeMirror rebuilds its view when the phrases facet changes. Keep this
    // mapping fixed, and resolve a product label only when state.phrase reads it.
    return Object.freeze(phrases);
}

/** This panel translates owned labels without replacing a field or dispatching a search edit. */
class LocalizedSearchPanel implements Panel {
    readonly dom = document.createElement("div");
    private readonly scope = new LocaleScope();
    private readonly find = document.createElement("input");
    private readonly replacement = document.createElement("input");
    private readonly caseSensitive = document.createElement("input");
    private readonly regexp = document.createElement("input");
    private readonly wholeWord = document.createElement("input");
    private readonly replacementRow = document.createElement("span");
    private query: SearchQuery;
    constructor(private readonly view: EditorView) {
        this.query = getSearchQuery(view.state);
        this.dom.className = "cm-search";
        this.find.name = "search"; this.find.className = "cm-textfield"; this.find.setAttribute("main-field", "true");
        this.replacement.name = "replace"; this.replacement.className = "cm-textfield";
        for (const [input, key] of [[this.find, "editor.cmFind"], [this.replacement, "editor.cmReplaceInput"]] as const) {
            this.scope.bindAttr(input, "aria-label", message(key)); this.scope.bindAttr(input, "placeholder", message(key));
            input.addEventListener("input", () => this.commit());
        }
        const button = (name: string, key: keyof typeof phraseKeys, command: (view: EditorView) => boolean) => {
            const element = document.createElement("button"); element.type = "button"; element.name = name; element.className = "cm-button";
            this.scope.bindText(element, message(phraseKeys[key])); element.addEventListener("click", () => command(view)); return element;
        };
        this.dom.append(this.find, button("next", "next", findNext), button("prev", "previous", findPrevious), button("select", "all", selectMatches));
        for (const [input, name, key] of [[this.caseSensitive, "case", "editor.cmMatchCase"], [this.regexp, "re", "editor.cmRegexp"], [this.wholeWord, "word", "editor.cmWholeWord"]] as const) {
            input.type = "checkbox"; input.name = name; input.addEventListener("change", () => this.commit());
            const label = document.createElement("label"); label.append(input); this.scope.bindText(label, message(key)); this.dom.append(label);
        }
        this.replacementRow.append(document.createElement("br"), this.replacement, button("replace", "replace", replaceNext), button("replaceAll", "replace all", replaceAll));
        const close = button("close", "close", closeSearchPanel); this.scope.bindAttr(close, "aria-label", message("editor.cmClose"));
        this.dom.append(this.replacementRow, close);
        this.dom.addEventListener("keydown", event => {
            if (runScopeHandlers(view, event, "search-panel")) event.preventDefault();
            else if (event.key === "Enter" && !event.isComposing) {
                if (event.target === this.find) { event.preventDefault(); (event.shiftKey ? findPrevious : findNext)(view); }
                else if (event.target === this.replacement) { event.preventDefault(); replaceNext(view); }
            }
        });
        this.setQuery(this.query); this.updateReadOnly(view.state);
    }
    private commit(): void {
        const query = new SearchQuery({ search: this.find.value, replace: this.replacement.value, caseSensitive: this.caseSensitive.checked, regexp: this.regexp.checked, wholeWord: this.wholeWord.checked, literal: this.query.literal, test: this.query.test });
        if (!query.eq(this.query)) { this.query = query; this.view.dispatch({ effects: setSearchQuery.of(query) }); }
    }
    private setQuery(query: SearchQuery): void {
        this.query = query; this.find.value = query.search; this.replacement.value = query.replace;
        this.caseSensitive.checked = query.caseSensitive; this.regexp.checked = query.regexp; this.wholeWord.checked = query.wholeWord;
    }
    private updateReadOnly(state: EditorState): void { this.replacementRow.hidden = state.readOnly; }
    update(update: ViewUpdate): void {
        for (const transaction of update.transactions) for (const effect of transaction.effects) if (effect.is(setSearchQuery) && !effect.value.eq(this.query)) this.setQuery(effect.value);
        if (update.startState.readOnly !== update.state.readOnly) this.updateReadOnly(update.state);
    }
    mount(): void { this.find.select(); }
    destroy(): void { this.scope.dispose(); }
}

export const localizedSourceSearch = search({ createPanel: view => new LocalizedSearchPanel(view) });

/** Same public go-to semantics, with a live label around the original input. */
export function localizedGotoLine(view: EditorView): boolean {
    const open = getDialog(view, "cm-goto-line");
    if (open) { open.dom.querySelector<HTMLInputElement>('input[name="line"]')?.select(); return true; }
    const startState = view.state, scope = new LocaleScope();
    const { close, result } = showDialog(view, {
        class: "cm-goto-line", focus: true,
        content: () => {
            const form = document.createElement("form"), label = document.createElement("label"), input = document.createElement("input"), submit = document.createElement("button");
            input.type = "text"; input.name = "line"; input.value = String(startState.doc.lineAt(startState.selection.main.head).number);
            scope.bindText(label, message("editor.cmGoToLine")); scope.bindAttr(input, "aria-label", message("editor.cmGoToLine")); label.append(input);
            submit.type = "submit"; scope.bindText(submit, message("editor.cmGo")); form.append(label, submit); return form;
        },
    });
    const closeButton = getDialog(view, "cm-goto-line")?.dom.querySelector<HTMLButtonElement>(".cm-dialog-close");
    if (closeButton) scope.bindAttr(closeButton, "aria-label", message("editor.cmClose"));
    void result.then(form => {
        const input = form?.elements.namedItem("line"), match = input instanceof HTMLInputElement && /^([+-])?(\d+)?(:\d+)?(%)?$/.exec(input.value);
        if (!match) { view.dispatch({ effects: close }); return; }
        const start = startState.doc.lineAt(startState.selection.main.head), [, sign, number, column, percent] = match;
        let line = number ? Number(number) : start.number;
        if (number && percent) line = Math.round(startState.doc.lines * (line / 100 * (sign === "-" ? -1 : 1) + (sign ? start.number / startState.doc.lines : 0)));
        else if (number && sign) line = line * (sign === "-" ? -1 : 1) + start.number;
        const target = view.state.doc.line(Math.max(1, Math.min(view.state.doc.lines, line)));
        const position = target.from + Math.max(0, Math.min(column ? Number(column.slice(1)) : 0, target.length));
        view.dispatch({ effects: [close, EditorView.scrollIntoView(position, { y: "center" })], selection: EditorSelection.cursor(position) });
    }).finally(() => scope.dispose());
    return true;
}
