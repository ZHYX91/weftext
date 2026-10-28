import { disposeUiTree, joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { createTableGrid, disposeTableGrid } from "./table-grid";
import { descriptionUiText } from "./ui-diagnostics";
import { captureAuthorInput, type AuthorInputSession } from "./author-input";
import { delimitedTable, parseHTMLTable, type ClipboardTable } from "./table-clipboard";
import { rectangularTSV, textareaSourceDraft, type WritingController, type WritingRange, type WritingStructure, type WritingIntent, type WritingBlock, type WritingSnapshot } from "./writing";
export interface WritingControlOptions {
    onError?(error: Error): void;
    renderedTable?: Element;
    insertLink?(): Promise<void>;
    revealSource?(range: WritingRange): void;
}
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: UiText): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    node.className = className;
    if (text != null)
        setUiText(node, text);
    return node;
}
function report(options: WritingControlOptions, error: unknown): void { options.onError?.(error instanceof Error ? error : new UiError(errorUiText(error))); }
function action(label: UiText, run: () => Promise<void> | void, options: WritingControlOptions, className = "writing-button"): HTMLButtonElement {
    const button = element("button", className, label);
    button.type = "button";
    // Formatting keeps the current source/native text selection when clicked.
    button.addEventListener("mousedown", event => event.preventDefault());
    button.onclick = () => { Promise.resolve().then(run).catch(error => report(options, error)); };
    return button;
}
function field(label: UiText, input: HTMLElement): HTMLLabelElement { const wrapper = element("label", "writing-field"); wrapper.append(element("span", "", label), input); return wrapper; }
function textInput(value = "", rows = 3, inputs?: AuthorInputSession, slot = "text"): HTMLTextAreaElement { const input = element("textarea", "writing-input"); input.value = value; input.rows = rows; inputs?.bind(input, slot); return input; }
function numberInput(value: number, min: number, max?: number): HTMLInputElement {
    const input = element("input", "writing-input");
    input.type = "number";
    input.min = String(min);
    if (max != null)
        input.max = String(max);
    input.value = String(value);
    input.step = "1";
    return input;
}
function checkInput(label: UiText, value: boolean): {
    field: HTMLLabelElement;
    input: HTMLInputElement;
} { const input = element("input"); input.type = "checkbox"; input.checked = value; const wrapper = element("label", "writing-checkbox"); wrapper.append(input, uiTextNode(label)); return { field: wrapper, input }; }
function selectInput(choices: [
    string,
    UiText
][], value: string): HTMLSelectElement {
    const select = element("select", "writing-input");
    for (const [key, label] of choices) {
        const option = element("option", "", label);
        option.value = key;
        select.append(option);
    }
    select.value = value;
    return select;
}
function validInteger(input: HTMLInputElement): number {
    const value = Number(input.value);
    if (!input.checkValidity() || !Number.isSafeInteger(value))
        throw new UiError(uiMessage("writing.integerRequired"));
    return value;
}
function guard(controller: WritingController, snapshot: WritingSnapshot, intent: WritingIntent): () => WritingIntent {
    return () => {
        if (!controller.isSnapshotCurrent(snapshot))
            throw new UiError(uiMessage("writing.basisChanged"));
        return intent;
    };
}
/** Accessible host dialog: source stays in the shared controller until apply. */
async function dialog(controller: WritingController, title: UiText, help: UiText, build: (form: HTMLFormElement, done: () => void, error: HTMLElement, inputs: AuthorInputSession | undefined, onClose: (cleanup: () => void) => void) => void): Promise<void> {
    const host = element("dialog", "writing-dialog");
    const form = element("form", "writing-form");
    const heading = element("h2", "", title);
    const id = `writing-dialog-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    heading.id = id;
    host.setAttribute("aria-labelledby", id);
    form.append(heading, element("p", "writing-help", help));
    const error = element("p", "writing-error");
    error.setAttribute("role", "alert");
    error.hidden = true;
    const inputs = controller.authorInputs(), stopCloseGuard = controller.registerCloseGuard(() => host.open);
    const cleanups = new Set<() => void>();
    await new Promise<void>(resolve => {
        let settled = false;
        const done = () => {
            if (settled)
                return;
            settled = true;
            stopCloseGuard();
            inputs?.retire();
            cleanups.forEach(cleanup => cleanup());
            host.close();
            disposeUiTree(host);
            host.remove();
            resolve();
        };
        host.addEventListener("cancel", event => { event.preventDefault(); done(); });
        build(form, done, error, inputs, cleanup => cleanups.add(cleanup));
        form.append(error);
        host.append(form);
        document.body.append(host);
        host.showModal();
    });
}
function actions(form: HTMLFormElement, done: () => void, label: UiText = uiMessage("writing.apply")): HTMLButtonElement {
    const row = element("div", "writing-actions");
    const cancel = element("button", "writing-button", uiMessage("common.cancel"));
    cancel.type = "button";
    cancel.onclick = done;
    const submit = element("button", "writing-button primary", label);
    submit.type = "submit";
    row.append(cancel, submit);
    form.append(row);
    return submit;
}
function submit(form: HTMLFormElement, error: HTMLElement, done: () => void, run: () => Promise<void>): void {
    form.onsubmit = event => {
        event.preventDefault();
        if (!form.reportValidity())
            return;
        const buttons = Array.from(form.querySelectorAll<HTMLButtonElement>("button"));
        buttons.forEach(button => { button.disabled = true; });
        Promise.resolve().then(run).then(done).catch(failure => { if (!error.isConnected) return; error.hidden = false; setUiText(error, errorUiText(failure)); buttons.forEach(button => { button.disabled = false; }); });
    };
}
export async function editDocumentRole(controller: WritingController, role: "title" | "subtitle"): Promise<void> {
    await controller.flush();
    const snapshot = controller.snapshot, map = controller.map;
    if (!map)
        throw new UiError(uiMessage("writing.waitEditingMap"));
    const current = map[role];
    if (!["absent", "writable"].includes(current.status))
        throw new UiError(displayMessage("writing.roleNotLossless", { p0: uiDisplayParameter(role === "title" ? uiMessage("writing.title") : uiMessage("writing.subtitle")), p1: uiDisplayParameter(current.status) }));
    if (role === "subtitle" && map.title.status !== "writable")
        throw new UiError(uiMessage("writing.uniqueTitleRequired"));
    await dialog(controller, role === "title" ? uiMessage("writing.documentTitle") : uiMessage("writing.documentSubtitle"), role === "title" ? uiMessage("writing.titleHelp") : uiMessage("writing.subtitleHelp"), (form, done, error, inputs) => {
        const input = textInput(current.text ?? "", 2, inputs, role);
        form.append(field(role === "title" ? uiMessage("writing.titleText") : uiMessage("writing.subtitleText"), input));
        actions(form, done);
        submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: role === "title" ? "setTitle" : "setSubtitle", text: input.value || null })));
    });
}
export async function insertWritingBlock(controller: WritingController, initial = "paragraph"): Promise<void> {
    await controller.prepareCommand();
    const snapshot = controller.snapshot;
    const at = controller.insertionPoint();
    await dialog(controller, uiMessage("writing.addContent"), uiMessage("writing.insertHelp"), (form, done, error, inputs) => {
        const kind = selectInput([["paragraph", uiMessage("writing.paragraph")], ["heading", uiMessage("writing.heading")], ["bulletList", uiMessage("writing.bulletList")], ["numberedList", uiMessage("writing.numberedList")], ["termList", uiMessage("writing.termList")], ["image", uiMessage("writing.image")], ["math", uiMessage("writing.equation")], ["table", uiMessage("writing.plainTable")], ["raw", uiMessage("writing.codeBlock")]] as const, initial);
        const input = textInput("", 5, inputs, "block");
        setUiAttr(input, "aria-label", uiMessage("writing.newContent"));
        const level = numberInput(1, 1, 1024);
        inputs?.bind(level, "heading-level");
        const levelField = field(uiMessage("writing.headingLevelHelp"), level);
        const display = checkInput(uiMessage("writing.displayEquation"), true);
        const language = element("input", "writing-input");
        setUiAttr(language, "placeholder", uiMessage("writing.codeLanguagePlaceholder"));
        inputs?.bind(language, "language");
        const languageField = field(uiMessage("writing.codeLanguage"), language);
        const preview = element("p", "writing-help");
        preview.setAttribute("aria-live", "polite");
        const refresh = () => {
            levelField.hidden = kind.value !== "heading";
            display.field.hidden = kind.value !== "math";
            languageField.hidden = kind.value !== "raw";
            setUiAttr(input, "placeholder", kind.value === "image" ? uiMessage("writing.imagePathHelp") : kind.value === "table" ? uiMessage("writing.tsvInputHelp") : kind.value === "termList" ? uiMessage("writing.termInputHelp") : ["bulletList", "numberedList"].includes(kind.value) ? uiMessage("writing.listInputHelp") : kind.value === "math" ? "x^2 + y^2 = z^2" : uiMessage("writing.contentPlaceholder"));
            if (kind.value === "table") {
                try {
                    const grid = rectangularTSV(input.value);
                    setUiText(preview, displayMessage("writing.tableDimensions", { p0: grid.length, p1: grid[0].length }));
                }
                catch (failure) {
                    if (preview.isConnected) setUiText(preview, errorUiText(failure));
                }
            }
            else
                setUiText(preview, "");
        };
        kind.onchange = refresh;
        input.oninput = refresh;
        form.append(field(uiMessage("writing.contentKind"), kind), field(uiMessage("writing.content"), input), levelField, display.field, languageField, preview);
        refresh();
        actions(form, done, uiMessage("writing.add"));
        submit(form, error, done, async () => {
            let block: WritingBlock;
            if (kind.value === "image")
                block = { kind: "image", path: input.value.trim(), alt: "" };
            else if (kind.value === "heading")
                block = { kind: "heading", text: input.value, level: validInteger(level) };
            else if (kind.value === "bulletList" || kind.value === "numberedList")
                block = { kind: kind.value, items: input.value.replace(/\r\n/g, "\n").split("\n") };
            else if (kind.value === "termList") {
                const items = input.value.replace(/\r\n/g, "\n").split("\n").map(row => { const [term, ...description] = row.split("\t"); return { term, description: description.join("\t") }; });
                block = { kind: "termList", items };
            }
            else if (kind.value === "math")
                block = { kind: "math", text: input.value, display: display.input.checked };
            else if (kind.value === "table") {
                const grid = rectangularTSV(input.value);
                block = { kind: "table", columns: grid[0].length, cells: grid.flat() };
            }
            else if (kind.value === "raw")
                block = { kind: "raw", text: input.value, language: language.value || null, block: true };
            else
                block = { kind: "paragraph", text: input.value };
            await controller.execute(guard(controller, snapshot, { kind: "insertBlock", at, block }));
        });
    });
}
export async function editHeading(controller: WritingController, heading?: WritingStructure): Promise<void> {
    const originSnapshot = controller.snapshot;
    await controller.flush();
    if (heading && !controller.isSnapshotCurrent(originSnapshot))
        throw new UiError(uiMessage("writing.headingSourceChanged"));
    const current = heading ?? controller.structure("heading");
    if (!current) {
        await insertWritingBlock(controller, "heading");
        return;
    }
    const snapshot = controller.snapshot;
    await dialog(controller, uiMessage("writing.heading"), uiMessage("writing.headingHelp"), (form, done, error, inputs) => {
        const level = numberInput(current.level ?? 1, 1, 1024);
        inputs?.bind(level, "heading-level");
        const runIn = checkInput(uiMessage("writing.runInHeading"), current.runIn);
        const operation = selectInput([["level", uiMessage("writing.setHeadingLevel")], ["runIn", uiMessage("writing.setRunIn")]] as const, "level");
        if (!current.levelWritable && current.levelReason) form.append(element("p", "writing-help", descriptionUiText(current.levelReason)));
        form.append(field(uiMessage("writing.operation"), operation), field(uiMessage("writing.level"), level), runIn.field);
        const refresh = () => { level.disabled = operation.value !== "level" || !current.levelWritable; runIn.input.disabled = operation.value !== "runIn"; };
        operation.onchange = refresh;
        refresh();
        actions(form, done);
        submit(form, error, done, () => {
            if (operation.value !== "runIn" && !current.levelWritable) throw new UiError(current.levelReason ? descriptionUiText(current.levelReason) : uiMessage("error.invalidResponse"));
            return controller.execute(guard(controller, snapshot, operation.value === "runIn" ? { kind: "setRunIn", range: current.range, enabled: runIn.input.checked } : { kind: "setHeadingLevel", range: current.range, level: validInteger(level) }));
        });
    });
}
export async function editMath(controller: WritingController, math?: WritingStructure): Promise<void> {
    const originSnapshot = controller.snapshot;
    await controller.flush();
    if (math && !controller.isSnapshotCurrent(originSnapshot))
        throw new UiError(uiMessage("writing.equationSourceChanged"));
    const current = math ?? controller.structure("inlineMath") ?? controller.structure("displayMath");
    if (!current) {
        await insertWritingBlock(controller, "math");
        return;
    }
    const snapshot = controller.snapshot;
    await dialog(controller, uiMessage("writing.editEquation"), uiMessage("writing.equationHelp"), (form, done, error, inputs) => {
        const input = textInput(current.body?.raw ?? current.range.raw.replace(/^\$\s?|\s?\$$/g, ""), 4, inputs, "math");
        input.classList.add("writing-math-source");
        setUiAttr(input, "aria-label", uiMessage("writing.nativeMathExpression"));
        const display = checkInput(uiMessage("writing.displayEquation"), current.kind === "displayMath");
        form.append(field(uiMessage("writing.mathExpression"), input), display.field);
        actions(form, done);
        submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: "replaceMath", range: current.range, text: input.value, display: display.input.checked })));
    });
}
export async function editRaw(controller: WritingController, raw?: WritingStructure): Promise<void> {
    const originSnapshot = controller.snapshot;
    await controller.flush();
    if (raw && !controller.isSnapshotCurrent(originSnapshot))
        throw new UiError(uiMessage("writing.codeSourceChanged"));
    const current = raw ?? controller.structure("inlineRaw") ?? controller.structure("blockRaw");
    if (!current) {
        await insertWritingBlock(controller, "raw");
        return;
    }
    if (!current.body)
        throw new UiError(uiMessage("writing.codeRangeUnavailable"));
    const snapshot = controller.snapshot;
    await dialog(controller, uiMessage("writing.editCode"), uiMessage("writing.codeHelp"), (form, done, error, inputs) => {
        const input = textInput(current.body!.raw, 8, inputs, "raw");
        input.classList.add("writing-math-source");
        input.spellcheck = false;
        const language = element("input", "writing-input");
        language.value = current.language ?? "";
        setUiAttr(language, "placeholder", uiMessage("writing.codeLanguagePlaceholder"));
        inputs?.bind(language, "language");
        const block = checkInput(uiMessage("writing.standaloneCodeBlock"), current.kind === "blockRaw");
        form.append(field(uiMessage("writing.codeContent"), input), field(uiMessage("writing.codeLanguage"), language), block.field);
        actions(form, done);
        submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: "replaceRaw", range: current.range, text: textareaSourceDraft(current.body!.raw, input.value), language: language.value || null, block: block.input.checked })));
    });
}
export async function editTable(controller: WritingController, structure?: WritingStructure): Promise<void> {
    const originSnapshot = controller.snapshot;
    await controller.flush();
    if (structure && !controller.isSnapshotCurrent(originSnapshot))
        throw new UiError(uiMessage("writing.tableSourceChanged"));
    const current = structure ?? controller.structure("table");
    if (!current?.table) {
        await insertWritingBlock(controller, "table");
        return;
    }
    await dialog(controller, uiMessage("writing.table"), uiMessage("writing.tableHelp"), (form, done, error, inputs, onClose) => {
        form.closest("dialog")?.classList.add("writing-table-dialog");
        const surface = createTableGrid(controller, current, { onError: failure => { if (!error.isConnected) return; error.hidden = false; setUiText(error, errorUiText(failure)); } });
        onClose(() => disposeTableGrid(surface));
        const close = element("button", "writing-button", uiMessage("writing.finishTableEditing"));
        close.type = "button";
        close.onclick = () => { void controller.deferUntilCompositionEnds(async () => { await controller.flush(); done(); }).catch(failure => { if (!error.isConnected) return; error.hidden = false; setUiText(error, errorUiText(failure)); }); };
        form.append(surface, close);
    });
}
export async function insertWritingLink(controller: WritingController): Promise<void> {
    await controller.flush();
    const snapshot = controller.snapshot, at = controller.insertionPoint();
    await dialog(controller, uiMessage("writing.addLink"), uiMessage("writing.linkHelp"), (form, done, error, inputs) => {
        const target = element("input", "writing-input"), text = element("input", "writing-input");
        target.required = true;
        text.required = true;
        inputs?.bind(target, "link-target");
        inputs?.bind(text, "link-text");
        form.append(field(uiMessage("writing.linkTarget"), target), field(uiMessage("writing.linkText"), text));
        actions(form, done, uiMessage("writing.add"));
        submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: "insertBlock", at, block: { kind: "link", target: target.value, text: text.value } })));
    });
}
/** Body insertion is distinct from text-in-cell and grid-range paste. */
export async function insertClipboardTable(controller: WritingController): Promise<void> {
    await controller.flush();
    const snapshot = controller.snapshot, at = controller.insertionPoint();
    await dialog(controller, uiMessage("writing.insertClipboardTable"), uiMessage("writing.clipboardTableHelp"), (form, done, error, inputs) => {
        const format = selectInput([["tsv", uiMessage("writing.tsvFormat")], ["csv", uiMessage("writing.csvFormat")], ["html", uiMessage("writing.htmlTableFormat")]] as const, "tsv"), input = textInput("", 7, inputs, "clipboard-table"), preview = element("p", "writing-help"), acceptLoss = checkInput(uiMessage("writing.acceptTextConversionLoss"), false);
        setUiAttr(input, "aria-label", uiMessage("writing.pendingTableInput"));
        preview.setAttribute("aria-live", "polite");
        let parsed: ClipboardTable | null = null;
        const refresh = () => {
            parsed = null;
            try {
                parsed = format.value === "html" ? parseHTMLTable(input.value) : delimitedTable(input.value, format.value === "csv" ? "," : "\t");
                setUiText(preview, displayMessage("writing.clipboardTableSummary", { p0: parsed.rows, p1: parsed.columns, p2: parsed.cells.length, p3: joinUiText(parsed.losses, "\n") }));
                acceptLoss.field.hidden = !parsed.losses.length;
            }
            catch (failure) {
                if (preview.isConnected) setUiText(preview, errorUiText(failure));
                acceptLoss.field.hidden = true;
            }
            acceptLoss.input.checked = false;
        };
        input.oninput = refresh;
        format.onchange = refresh;
        input.onpaste = event => {
            const html = event.clipboardData?.getData("text/html");
            if (html && /<table[\s>]/i.test(html)) {
                event.preventDefault();
                const lease = captureAuthorInput(input);
                if (lease ? lease.apply(html) : !inputs && input.isConnected && controller.isSnapshotCurrent(snapshot) && (input.value = html, true)) {
                    format.value = "html";
                    refresh();
                }
            }
        };
        const read = action(uiMessage("writing.readClipboardTable"), async () => {
            const lease = captureAuthorInput(input), originalValue = input.value;
            const deliver = (value: string, kind: string) => {
                if (lease ? lease.apply(value) : !inputs && input.isConnected && input.value === originalValue && controller.isSnapshotCurrent(snapshot) && (input.value = value, true)) {
                    format.value = kind;
                    refresh();
                }
                else
                    throw new UiError(uiMessage("writing.clipboardTargetChanged"));
            };
            if (navigator.clipboard?.read) {
                const items = await navigator.clipboard.read();
                for (const item of items)
                    if (item.types.includes("text/html")) {
                        const value = await (await item.getType("text/html")).text();
                        deliver(value, "html");
                        return;
                    }
            }
            if (!navigator.clipboard?.readText)
                throw new UiError(uiMessage("writing.clipboardReadUnavailable"));
            const value = await navigator.clipboard.readText();
            deliver(value, "tsv");
        }, { onError: failure => { if (!error.isConnected) return; error.hidden = false; setUiText(error, errorUiText(failure)); } });
        form.append(field(uiMessage("writing.inputFormat"), format), read, field(uiMessage("writing.tableInput"), input), preview, acceptLoss.field);
        refresh();
        actions(form, done, uiMessage("writing.insertTable"));
        submit(form, error, done, async () => {
            if (!parsed)
                throw new UiError(uiMessage("writing.fixTableInput"));
            if (parsed.losses.length && !acceptLoss.input.checked)
                throw new UiError(uiMessage("writing.acceptFormatLossRequired"));
            await controller.execute(guard(controller, snapshot, { kind: "insertBlock", at, block: { kind: "clipboardTable", rows: parsed.rows, columns: parsed.columns, cells: parsed.cells } }));
        });
    });
}
export interface WritingCommandGroups {
    format: HTMLElement;
    insert: HTMLElement;
    history: HTMLElement;
}
/** Shared ribbon commands retain the original writer, composition deferral and
 * selection-preserving action nodes. The shell only chooses their placement. */
export function createWritingCommandGroups(controller: WritingController, options: WritingControlOptions = {}): WritingCommandGroups {
    const format = element("div", "writing-command-group"), insert = element("div", "writing-command-group"), history = element("div", "writing-command-group");
    const tool = (label: UiText, run: () => Promise<void>) => action(label, () => controller.deferUntilCompositionEnds(run), options);
    for (const [label, kind] of [[uiMessage("writing.bold"), "strong"], [uiMessage("writing.italic"), "emph"], [uiMessage("writing.strikethrough"), "strike"]] as const)
        format.append(tool(label, () => controller.execute(() => ({ kind: "setInline", range: controller.selectionRange(), format: kind }))));
    insert.append(tool(uiMessage("writing.title"), () => editDocumentRole(controller, "title")), tool(uiMessage("writing.subtitle"), () => editDocumentRole(controller, "subtitle")), tool(uiMessage("writing.section"), () => editHeading(controller)), tool(uiMessage("writing.list"), () => insertWritingBlock(controller, "bulletList")), tool(uiMessage("writing.table"), () => editTable(controller)), tool(uiMessage("writing.equation"), () => editMath(controller)), tool(uiMessage("writing.code"), () => editRaw(controller)));
    if (options.insertLink)
        insert.append(tool(uiMessage("writing.link"), options.insertLink));
    insert.append(tool(uiMessage("writing.addContent"), () => insertWritingBlock(controller)), tool(uiMessage("writing.insertClipboardTable"), () => insertClipboardTable(controller)));
    history.append(tool(uiMessage("writing.undo"), () => controller.undo()), tool(uiMessage("writing.redo"), () => controller.redo()));
    return { format, insert, history };
}
export function createStructureControls(controller: WritingController, structure: WritingStructure, options: WritingControlOptions = {}): HTMLElement | null {
    const snapshot = controller.snapshot;
    const checked = (run: () => Promise<void> | void) => async () => {
        if (!controller.isSnapshotCurrent(snapshot))
            throw new UiError(uiMessage("writing.structureExpired"));
        await run();
    };
    const controls = element("div", "writing-structure-controls");
    controls.setAttribute("role", "group");
    setUiAttr(controls, "aria-label", uiMessage("writing.structureActions"));
    if (structure.kind === "heading")
        controls.append(action(displayMessage("writing.headingSummary", { p0: uiDisplayParameter(structure.level), p1: uiDisplayParameter(structure.runIn ? uiMessage("writing.runIn") : uiMessage("writing.separateParagraph")) }), checked(() => editHeading(controller, structure)), options));
    else if (structure.kind === "table" && structure.table)
        return createTableGrid(controller, structure, options);
    else if (["inlineMath", "displayMath"].includes(structure.kind))
        controls.append(action(uiMessage("writing.editEquation"), checked(() => editMath(controller, structure)), options));
    else if (["inlineRaw", "blockRaw"].includes(structure.kind))
        controls.append(action(uiMessage("writing.editCode"), checked(() => editRaw(controller, structure)), options));
    else if (["bulletListItem", "numberedListItem"].includes(structure.kind)) {
        const execute = (kind: "indent" | "outdent") => controller.execute(guard(controller, snapshot, { kind: "listEdit", range: structure.range, action: { kind } }));
        controls.append(action(uiMessage("writing.indent"), checked(() => execute("indent")), options), action(uiMessage("writing.outdent"), checked(() => execute("outdent")), options));
    }
    if (options.revealSource)
        controls.append(action(uiMessage("writing.editSource"), checked(() => options.revealSource!(structure.range)), options));
    return controls.childElementCount ? controls : null;
}
