import { disposeUiTree, replaceUiChildren, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { DocumentNotesInputTracker } from "./personal-input-text";
export { DocumentNotesInputTracker } from "./personal-input-text";
import { decodeWritingRange, exactWritingRange, type WritingController, type WritingRange, type WritingSnapshot, type WritingStructure } from "./writing";
import { decodeErrorDescription, descriptionUiText, exactKeys, type ErrorDescription } from "./ui-diagnostics";
import type { RenderNode } from "./types";
/** Disposable products of the same native compile. Labels/text never identify a note. */
export interface NotesSourceEntry {
    kind: string;
    construct: WritingRange;
    body: WritingRange | null;
    label: string | null;
    target: string | null;
    writable: boolean;
    unavailableKind: "frozenModuleRequired" | "referenceDynamic" | "referenceArgumentsComplex" | "other" | null;
    unavailableReason: ErrorDescription | null;
}
export interface NotesSourceMap {
    sourceHash: string;
    syntaxValid: boolean;
    entries: NotesSourceEntry[];
}
/** The enum is native admission evidence; translated explanations grant no edit capability. */
export function decodeNotesSourceMap(value: unknown): NotesSourceMap {
    const invalid = (): never => { throw new UiError(uiMessage("error.invalidResponse")); };
    if (!value || typeof value !== "object" || Array.isArray(value)) return invalid();
    const map = value as Record<string, unknown>;
    if (!exactKeys(map, ["sourceHash", "syntaxValid", "entries"]) || typeof map.sourceHash !== "string" || typeof map.syntaxValid !== "boolean" || !Array.isArray(map.entries)) return invalid();
    const entries = map.entries.map((value): NotesSourceEntry => {
        if (!value || typeof value !== "object" || Array.isArray(value)) return invalid();
        const item = value as Record<string, unknown>, reason = item.unavailableReason === null ? null : decodeErrorDescription(item.unavailableReason);
        if (!exactKeys(item, ["kind", "construct", "body", "label", "target", "writable", "unavailableKind", "unavailableReason"]) || typeof item.kind !== "string" || typeof item.writable !== "boolean" || item.label !== null && typeof item.label !== "string" || item.target !== null && typeof item.target !== "string" || reason === undefined || item.unavailableKind !== null && (typeof item.unavailableKind !== "string" || !["frozenModuleRequired", "referenceDynamic", "referenceArgumentsComplex", "other"].includes(item.unavailableKind))) return invalid();
        return { kind: item.kind, construct: decodeWritingRange(item.construct), body: item.body === null ? null : decodeWritingRange(item.body), label: item.label as string | null, target: item.target as string | null, writable: item.writable, unavailableKind: item.unavailableKind as NotesSourceEntry["unavailableKind"], unavailableReason: reason };
    });
    return { sourceHash: map.sourceHash, syntaxValid: map.syntaxValid, entries };
}
export interface DocumentNotesProjection {
    source: string;
    sourceHash: string;
    notes: NotesSourceMap;
    tree: RenderNode | null;
    structures?: WritingStructure[];
}
export interface NoteBodyInput {
    mode: "literal" | "source";
    value: string;
}
export type NotesNumbering = {
    kind: "none";
} | {
    kind: "pattern";
    pattern: string;
};
export type DocumentNotesAction = {
    kind: "insertFootnote" | "insertEndnote";
    at: WritingRange;
    body: NoteBodyInput;
} | {
    kind: "repeatFootnote" | "repeatEndnote";
    at: WritingRange;
    definition: WritingRange;
} | {
    kind: "setNoteBody";
    construct: WritingRange;
    body: NoteBodyInput;
} | {
    kind: "removeNote";
    construct: WritingRange;
    allReferences: boolean;
} | {
    kind: "setCaption";
    construct: WritingRange;
    body: NoteBodyInput;
    position: "top" | "bottom" | null;
    numbering: NotesNumbering | null;
} | {
    kind: "wrapCaption";
    object: WritingRange;
    body: NoteBodyInput;
    category: "figure" | "table" | "equation";
    position: "top" | "bottom";
    numbering: NotesNumbering;
} | {
    kind: "setEquationNumbering";
    construct: WritingRange;
    numbering: NotesNumbering;
};
export interface DocumentNotesIntent {
    kind: "nativeNote";
    action: DocumentNotesAction;
}
export interface DocumentNoteReference {
    key: string;
    range: WritingRange | null;
    definition: boolean;
    writable: boolean;
}
export interface DocumentNote {
    id: string;
    kind: "footnote" | "endnote";
    number: string;
    text: string;
    definition: NotesSourceEntry | null;
    references: DocumentNoteReference[];
    bodyRange: WritingRange | null;
    writable: boolean;
    reason: UiText | null;
}
export interface DocumentCaption {
    id: string;
    category: string;
    prefix: string;
    text: string;
    position: "top" | "bottom" | null;
    numbered: boolean | null;
    entry: NotesSourceEntry | null;
    objectRange: WritingRange | null;
    writable: boolean;
    reason: UiText | null;
}
export interface DocumentCaptionObject {
    key: string;
    category: "figure" | "table" | "equation";
    range: WritingRange;
    title: UiText;
}
export interface DocumentEquation {
    id: string;
    range: WritingRange | null;
    numbered: boolean | null;
    prefix: string;
    writable: boolean;
}
export interface DocumentNotesSnapshot {
    sourceHash: string;
    notes: DocumentNote[];
    captions: DocumentCaption[];
    objects: DocumentCaptionObject[];
    equations: DocumentEquation[];
    unlocated: NotesSourceEntry[];
    available: boolean;
    reason: UiText | null;
}
export interface DocumentNotesHost {
    controller: WritingController;
    projection(): DocumentNotesProjection | null;
    readable?(): boolean;
    editable(): boolean;
    perform(intent: DocumentNotesIntent, basis: WritingSnapshot): Promise<void>;
    /** A true native caret range; the default is the declared body insertion. */
    insertionRange?(): WritingRange;
    revealSource?(range: WritingRange): void;
    revealRendered?(key: string): boolean;
    ensureEndnoteModule?(): Promise<void>;
    recoverEndnoteModule?(): Promise<void>;
    retryEndnoteModule?(): Promise<void>;
    restartEndnoteModule?(): Promise<void>;
    settleEndnoteModule?(): Promise<void>;
    /** Preserve only newly entered text when the old source becomes unreadable. */
    onPendingInput?(value: string): void;
    onError?(error: Error): void;
}
export interface DocumentNotesMount {
    refresh(): void;
    open(mode?: "footnote" | "endnote" | "caption", targetId?: string): Promise<void>;
    dispose(): void;
}
const openPanels = new WeakMap<WritingController, Set<() => void>>();
/** Called after the host revokes the projection, before disposing its controller. */
export function retireDocumentNotes(controller: WritingController): void {
    for (const retire of openPanels.get(controller) ?? [])
        retire();
}
const attr = (node: RenderNode, key: string): string | null => typeof node.attrs[key] === "string" ? node.attrs[key] as string : null;
const role = (node: RenderNode) => attr(node, "data-weftext-role");
function descendants(node: RenderNode): RenderNode[] { return [node, ...node.children.flatMap(descendants)]; }
function text(node: RenderNode): string { return node.kind === "text" ? node.text ?? "" : node.children.map(text).join(""); }
function exactOrigin(source: string, node: RenderNode): WritingRange | null {
    const origin = node.origin;
    if (!origin?.isMain)
        return null;
    const range = { ...origin, writable: node.writable };
    return exactWritingRange(source, range) ? range : null;
}
function inside(outer: WritingRange, inner: WritingRange): boolean { return outer.from <= inner.from && outer.to >= inner.to; }
function sameRange(a: WritingRange, b: WritingRange): boolean { return a.from === b.from && a.to === b.to && a.raw === b.raw; }
function sourceEntry(source: string, entries: NotesSourceEntry[], node: RenderNode, kinds: string[]): NotesSourceEntry | null {
    const origin = exactOrigin(source, node);
    if (!origin)
        return null;
    const matches = entries.filter(entry => kinds.includes(entry.kind) && exactWritingRange(source, entry.construct) && inside(entry.construct, origin));
    return matches.length === 1 ? matches[0] : null;
}
function bodyEntry(source: string, entries: NotesSourceEntry[], body: RenderNode, kinds: string[]): NotesSourceEntry | null {
    const origins = descendants(body).map(node => exactOrigin(source, node)).filter((range): range is WritingRange => !!range);
    const matches = entries.filter(entry => kinds.includes(entry.kind) && entry.body && exactWritingRange(source, entry.construct) && exactWritingRange(source, entry.body) && origins.some(origin => inside(entry.body!, origin) || sameRange(entry.body!, origin)));
    return matches.length === 1 ? matches[0] : null;
}
function booleanAttr(node: RenderNode, key: string): boolean | null { const value = node.attrs[key]; return value === true || value === "true" ? true : value === false || value === "false" ? false : null; }
/** Relation joins use typed producer IDs and exact native provenance, never HTML proximity. */
export function buildDocumentNotesSnapshot(projection: DocumentNotesProjection): DocumentNotesSnapshot {
    const { source, sourceHash, tree } = projection;
    const unavailable = (reason: UiText): DocumentNotesSnapshot => ({ sourceHash, notes: [], captions: [], objects: [], equations: [], unlocated: [], available: false, reason });
    if (!projection.notes || !sourceHash || !tree)
        return unavailable(uiMessage("documentNotes.waitingProjection"));
    const map = decodeNotesSourceMap(projection.notes);
    if (map.sourceHash !== sourceHash || !map.syntaxValid) return unavailable(uiMessage("documentNotes.waitingProjection"));
    if (map.entries.some(entry => !exactWritingRange(source, entry.construct) || entry.body && (!exactWritingRange(source, entry.body) || !inside(entry.construct, entry.body))))
        return unavailable(uiMessage("documentNotes.sourceRangeMismatch"));
    const nodes = descendants(tree), used = new Set<NotesSourceEntry>(), notes: DocumentNote[] = [];
    for (const kind of ["footnote", "endnote"] as const) {
        const bodies = nodes.filter(node => role(node) === `${kind}-body`), markers = nodes.filter(node => role(node) === `${kind}-reference`);
        const ids = new Set([...markers, ...bodies].map(node => attr(node, "data-weftext-note-id")).filter((id): id is string => !!id));
        for (const id of ids) {
            const bodyNodes = bodies.filter(node => attr(node, "data-weftext-note-id") === id), markerNodes = markers.filter(node => attr(node, "data-weftext-note-id") === id);
            const definitions = [`${kind}-definition`], markerKinds = [...definitions, `${kind}-reference`, "reference"];
            let definition = bodyNodes.length === 1 ? bodyEntry(source, map.entries, bodyNodes[0], definitions) : null;
            const references = markerNodes.map(node => {
                const entry = sourceEntry(source, map.entries, node, markerKinds);
                if (entry) {
                    used.add(entry);
                    if (definitions.includes(entry.kind))
                        definition ??= entry;
                }
                return { key: node.key, range: entry?.construct ?? exactOrigin(source, node), definition: !!entry && definitions.includes(entry.kind), writable: !!entry?.writable };
            });
            // Authenticated bundled endnotes carry their explicit stable ID; only a
            // unique matching source definition may join its otherwise module origin.
            if (kind === "endnote" && !definition && bodyNodes.length === 1) {
                const candidates = map.entries.filter(entry => definitions.includes(entry.kind) && (entry.target === id || entry.label === id));
                if (candidates.length === 1)
                    definition = candidates[0];
            }
            if (definition)
                used.add(definition);
            const bodyRange = definition?.body ?? null;
            const uniqueReferences = references.every(reference => !reference.range || references.filter(other => other.range && sameRange(reference.range!, other.range)).length === 1);
            const writable = bodyNodes.length === 1 && !!definition?.writable && !!bodyRange?.writable && uniqueReferences;
            notes.push({ id, kind, number: markerNodes[0] ? text(markerNodes[0]) : "", text: bodyNodes.length === 1 ? text(bodyNodes[0]) : "", definition, references, bodyRange, writable,
                reason: writable ? null : bodyNodes.length !== 1 ? uiMessage("documentNotes.bodyMissingOrDuplicate") : definition?.unavailableReason ? descriptionUiText(definition.unavailableReason) : uiMessage("documentNotes.uniqueMainSourceRequired") });
        }
    }
    const captions: DocumentCaption[] = [];
    for (const node of nodes.filter(node => role(node) === "figure")) {
        const id = attr(node, "data-weftext-target-id");
        if (!id)
            continue;
        const owners = nodes.filter(other => role(other) === "figure" && attr(other, "data-weftext-target-id") === id);
        const bodyNodes = nodes.filter(other => role(other) === "caption-body" && attr(other, "data-weftext-target-id") === id);
        const captionNodes = nodes.filter(other => role(other) === "caption" && attr(other, "data-weftext-target-id") === id);
        const prefixes = nodes.filter(other => role(other) === "caption-prefix" && attr(other, "data-weftext-target-id") === id);
        const entry = sourceEntry(source, map.entries, node, ["figure"]);
        if (entry)
            used.add(entry);
        const writable = owners.length === 1 && bodyNodes.length <= 1 && captionNodes.length <= 1 && !!entry?.writable;
        const position = attr(captionNodes[0] ?? node, "data-weftext-caption-position");
        captions.push({ id, category: attr(captionNodes[0] ?? node, "data-weftext-caption-kind") ?? "unknown", prefix: prefixes.length === 1 ? text(prefixes[0]) : "", text: bodyNodes.length === 1 ? text(bodyNodes[0]) : "", position: position === "top" || position === "bottom" ? position : null,
            numbered: booleanAttr(node, "data-weftext-numbered"), entry, objectRange: entry?.construct ?? exactOrigin(source, node), writable, reason: writable ? null : entry?.unavailableReason ? descriptionUiText(entry.unavailableReason) : uiMessage("documentNotes.captionSourceAmbiguous") });
    }
    const equationEntries = map.entries.filter(entry => entry.kind === "equation");
    const equationRanges = [...equationEntries.map(entry => ({ ...entry.construct, writable: entry.writable })), ...(projection.structures ?? []).filter(structure => structure.kind === "displayMath").map(structure => structure.range)].filter((range, index, ranges) => exactWritingRange(source, range) && ranges.findIndex(other => sameRange(other, range)) === index);
    const candidateObjects = [...(projection.structures ?? []).filter(structure => ["table", "image"].includes(structure.kind)), ...equationRanges.map(range => ({ kind: "displayMath", range }))];
    const objects: DocumentCaptionObject[] = candidateObjects.filter(structure => structure.range.writable && exactWritingRange(source, structure.range) && !captions.some(caption => caption.objectRange && inside(caption.objectRange, structure.range))).map(structure => ({ key: `${structure.kind}:${structure.range.from}:${structure.range.to}`, category: structure.kind === "table" ? "table" : structure.kind === "displayMath" ? "equation" : "figure", range: structure.range, title: displayMessage("documentNotes.objectSourceRange", { p0: uiDisplayParameter(structure.kind === "table" ? uiMessage("documentNotes.table") : structure.kind === "displayMath" ? uiMessage("documentNotes.equation") : uiMessage("documentNotes.image")), p1: structure.range.utf16From + 1, p2: structure.range.utf16To }) }));
    const equations: DocumentEquation[] = nodes.filter(node => role(node) === "equation").map(node => {
        const origin = exactOrigin(source, node), matches = origin ? equationRanges.filter(range => inside(range, origin)) : [], target = attr(node, "data-weftext-target-id");
        const prefixes = target ? nodes.filter(other => role(other) === "equation-prefix" && attr(other, "data-weftext-target-id") === target) : [];
        if (matches.length === 1)
            for (const entry of equationEntries)
                if (sameRange(entry.construct, matches[0]))
                    used.add(entry);
        return { id: target ?? node.key, range: matches.length === 1 ? matches[0] : null, numbered: booleanAttr(node, "data-weftext-numbered"), prefix: prefixes.length === 1 ? text(prefixes[0]) : "", writable: matches.length === 1 && matches[0].writable };
    });
    for (const equation of equations)
        if (equation.range && equations.filter(other => other.range && sameRange(other.range, equation.range!)).length !== 1)
            equation.writable = false;
    return { sourceHash, notes, captions, objects, equations, unlocated: map.entries.filter(entry => !used.has(entry) && !["reference", "footnote-reference", "endnote-reference"].includes(entry.kind)), available: true, reason: null };
}
export function requireDocumentNotesBasis(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash?: string): DocumentNotesProjection {
    const projection = host.projection();
    if (!projection || !host.controller.isSnapshotCurrent(basis) || projection.source !== basis.source || sourceHash && projection.sourceHash !== sourceHash)
        throw new UiError(uiMessage("documentNotes.proposalSourceChanged"));
    return projection;
}
export async function performDocumentNotes(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash: string, action: DocumentNotesAction): Promise<void> {
    requireDocumentNotesBasis(host, basis, sourceHash);
    if (!host.editable())
        throw new UiError(uiMessage("documentNotes.writePermissionRequired"));
    if (host.controller.state.composing)
        throw new UiError(uiMessage("documentNotes.finishComposition"));
    await host.controller.flush();
    requireDocumentNotesBasis(host, basis, sourceHash);
    await host.perform({ kind: "nativeNote", action }, basis);
}
export async function copyDocumentNote(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash: string, value: string, write: (value: string) => Promise<void> = value => {
    if (!navigator.clipboard?.writeText)
        throw new UiError(uiMessage("documentNotes.clipboardWriteUnavailable"));
    return navigator.clipboard.writeText(value);
}): Promise<void> {
    requireDocumentNotesBasis(host, basis, sourceHash);
    await write(value);
    // Copy never mutates Draft, even when permission is denied or the source
    // changes while the browser clipboard permission request is outstanding.
    requireDocumentNotesBasis(host, basis, sourceHash);
}
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", value?: UiText): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    node.className = className;
    if (value != null)
        setUiText(node, value);
    return node;
}
function label(title: UiText, input: HTMLElement): HTMLElement { const field = element("label", "document-notes-field"); field.append(element("span", "", title), input); return field; }
function select(choices: [
    string,
    UiText
][], value: string, title: UiText): HTMLSelectElement {
    const node = element("select", "document-notes-input");
    setUiAttr(node, "aria-label", title);
    for (const [key, text] of choices) {
        const option = element("option", "", text);
        option.value = key;
        node.append(option);
    }
    node.value = value;
    return node;
}
function button(title: UiText, work: () => void | Promise<void>, report: (error: unknown) => void): HTMLButtonElement { const node = element("button", "document-notes-button", title); node.type = "button"; node.onclick = () => { Promise.resolve().then(work).catch(report); }; return node; }
function failure(error: unknown): Error { return error instanceof Error ? error : new UiError(errorUiText(error)); }
const categoryName = (value: string) => value === "table" ? uiMessage("documentNotes.tableCategory") : value === "equation" ? uiMessage("documentNotes.equation") : value === "figure" ? uiMessage("documentNotes.figureCategory") : value;
/** Single proven replacement helper, retained for callers that have one edit. */
export function documentNotesNewInput(before: string, after: string): string {
    let from = 0, to = before.length, end = after.length;
    while (from < to && from < end && before[from] === after[from])
        from++;
    while (to > from && end > from && before[to - 1] === after[end - 1]) {
        to--;
        end--;
    }
    // Only a pure contiguous insertion is provable without edit history.
    return from === to ? after.slice(from, end) : "";
}
function insertion(host: DocumentNotesHost, source: string): WritingRange {
    const range = host.insertionRange?.() ?? host.controller.map?.bodyInsertion;
    if (!range?.writable || range.from !== range.to || !exactWritingRange(source, range))
        throw new UiError(uiMessage("documentNotes.insertionLocationRequired"));
    return range;
}
export function mountDocumentNotes(container: HTMLElement, host: DocumentNotesHost): DocumentNotesMount {
    let disposed = false, repainting = false;
    // Summary buttons are not editors: observing an unfocusable sentinel keeps
    // them fresh without retaining an obsolete projection after a dialog closes.
    const observation = element("span", "document-notes-observer");
    observation.hidden = true;
    const report = (error: unknown) => host.onError?.(failure(error));
    const refresh = () => {
        if (disposed || repainting)
            return;
        repainting = true;
        try {
            const projection = host.projection(), summary = projection && projection.source === host.controller.snapshot.source ? buildDocumentNotesSnapshot(projection) : null;
            const controls = element("div", "document-notes-tools");
            controls.setAttribute("role", "group");
            setUiAttr(controls, "aria-label", uiMessage("documentNotes.title"));
            for (const [mode, title] of [["footnote", uiMessage("documentNotes.footnote")], ["endnote", uiMessage("documentNotes.endnote")], ["caption", uiMessage("documentNotes.captionsAndNumbering")]] as const) {
                const trigger = button(title, () => openDocumentNotes(host, mode), report);
                trigger.disabled = !summary?.available;
                controls.append(trigger);
            }
            replaceUiChildren(container, controls, observation);
            const state = element("p", "document-notes-status", summary?.available ? displayMessage("documentNotes.summary", { p0: summary.notes.filter(note => note.kind === "footnote").length, p1: summary.notes.filter(note => note.kind === "endnote").length, p2: summary.captions.length }) : summary?.reason ?? uiMessage("documentNotes.waitingAuthorizedProjection"));
            state.setAttribute("aria-live", "polite");
            container.append(state, observation);
        }
        finally {
            repainting = false;
        }
    };
    const stop = host.controller.observeSurface(observation, refresh);
    refresh();
    return { refresh, open: (mode, targetId) => openDocumentNotes(host, mode, targetId), dispose() { disposed = true; stop(); replaceUiChildren(container); } };
}
/** All edits are reviewed proposals over one captured Draft, applied by the shared adapter. */
export async function openDocumentNotes(host: DocumentNotesHost, initialMode: "footnote" | "endnote" | "caption" = "footnote", targetId?: string): Promise<void> {
    await host.controller.deferUntilCompositionEnds(async () => { await host.controller.flush(); });
    const basis = host.controller.snapshot, projection = requireDocumentNotesBasis(host, basis), model = buildDocumentNotesSnapshot(projection);
    const authorInputs = host.controller.authorInputs();
    let inputSlot = 0;
    if (!model.available)
        throw new UiError(model.reason ?? uiMessage("documentNotes.projectionUnavailable"));
    const dialog = element("dialog", "document-notes-dialog"), form = element("form", "document-notes-panel"), title = element("h2", "", uiMessage("documentNotes.title")), status = element("p", "document-notes-status"), error = element("p", "document-notes-error"), list = element("section", "document-notes-list"), editor = element("section", "document-notes-editor");
    setUiAttr(dialog, "aria-label", uiMessage("documentNotes.title"));
    error.setAttribute("role", "alert");
    error.hidden = true;
    status.setAttribute("aria-live", "polite");
    form.onsubmit = event => event.preventDefault();
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    let mode = initialMode, busy = false, disposed = false, localComposition = false, pendingValue = "", dirty = false, retainedInput = false, tracker = new DocumentNotesInputTracker("");
    let recoveryDisplay: HTMLLabelElement | undefined;
    const stopCloseGuard = host.controller.registerCloseGuard(() => !disposed && (dirty || busy || localComposition));
    const navigation: {
        afterClose?: () => void;
    } = {};
    const report = (value: unknown) => { if (disposed) return; const message = failure(value); setUiText(error, errorUiText(message)); error.hidden = false; host.onError?.(message); };
    const current = () => { requireDocumentNotesBasis(host, basis, model.sourceHash); return !disposed; };
    const close = () => {
        if (localComposition || host.controller.state.composing || busy) {
            setUiText(status, uiMessage("documentNotes.finishInputOrWait"));
            return;
        }
        dialog.close();
    };
    const run = async (work: () => Promise<void>) => {
        if (busy || localComposition)
            return;
        busy = true;
        error.hidden = true;
        updateState();
        try {
            current();
            await work();
            if (!disposed) {
                dirty = false;
                dialog.close();
            }
        }
        catch (value) {
            if (!disposed)
                report(value);
        }
        finally {
            busy = false;
            if (!disposed)
                updateState();
        }
    };
    const perform = (action: DocumentNotesAction) => {
        if (dirty && !["setNoteBody", "setCaption", "wrapCaption", "insertFootnote", "insertEndnote", "setEquationNumbering"].includes(action.kind))
            throw new UiError(uiMessage("documentNotes.applyContentBeforeMarkers"));
        return run(() => performDocumentNotes(host, basis, model.sourceHash, action));
    };
    const toolbar = element("div", "document-notes-tools");
    toolbar.setAttribute("role", "tablist");
    const tabs = (["footnote", "endnote", "caption"] as const).map(kind => {
        const tab = button(kind === "footnote" ? uiMessage("documentNotes.footnote") : kind === "endnote" ? uiMessage("documentNotes.endnote") : uiMessage("documentNotes.captionsAndNumbering"), () => {
            if (dirty) {
                setUiText(status, uiMessage("documentNotes.applyBeforeSwitch"));
                return;
            }
            mode = kind;
            render();
        }, report);
        tab.setAttribute("role", "tab");
        toolbar.append(tab);
        return tab;
    });
    toolbar.onkeydown = event => {
        if (localComposition || event.isComposing || host.controller.state.composing || busy)
            return;
        const at = tabs.indexOf(document.activeElement as HTMLButtonElement);
        if (at < 0)
            return;
        const next = event.key === "ArrowRight" ? (at + 1) % tabs.length : event.key === "ArrowLeft" ? (at + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : -1;
        if (next >= 0) {
            event.preventDefault();
            tabs[next].focus();
            if (!dirty)
                tabs[next].click();
        }
    };
    const reveal = (range: WritingRange | null, key?: string) => {
        current();
        if (dirty || localComposition || busy)
            throw new UiError(uiMessage("documentNotes.applyBeforeNavigate"));
        if (range && !exactWritingRange(basis.source, range))
            throw new UiError(uiMessage("documentNotes.sourceUnlocatable"));
        navigation.afterClose = () => {
            current();
            const rendered = key ? host.revealRendered?.(key) : false;
            if (!rendered && range && host.editable())
                host.revealSource?.(range);
        };
        dialog.close();
    };
    const editorText = (value: string, textMode: "literal" | "source") => {
        const input = element("textarea", "document-notes-input document-notes-body");
        input.rows = 7;
        input.value = value;
        setUiAttr(input, "aria-label", uiMessage("documentNotes.bodyLabel"));
        input.spellcheck = false;
        pendingValue = value;
        tracker = new DocumentNotesInputTracker(value);
        retainedInput = false;
        authorInputs?.bind(input, `notes:${++inputSlot}:body`);
        let edit: {
            from: number;
            to: number;
            inputType: string;
        } | undefined;
        let composition: {
            tracker: DocumentNotesInputTracker;
            from: number;
            to: number;
        } | undefined;
        input.addEventListener("beforeinput", event => { edit = { from: input.selectionStart, to: input.selectionEnd, inputType: (event as InputEvent).inputType }; });
        const track = (event: Event) => {
            const data = (event as InputEvent | CompositionEvent).data;
            if (composition) {
                tracker = composition.tracker.copy();
                tracker.observe(input.value, { from: composition.from, to: composition.to, inputType: "insertCompositionText" }, data);
            }
            else
                tracker.observe(input.value, edit, data ?? (event as InputEvent).dataTransfer?.getData("text/plain"));
            edit = undefined;
            pendingValue = input.value;
            dirty = true;
        };
        input.oninput = track;
        input.addEventListener("compositionstart", () => { composition = { tracker: tracker.copy(), from: input.selectionStart, to: input.selectionEnd }; localComposition = true; updateState(); });
        input.addEventListener("compositionend", event => { track(event); composition = undefined; localComposition = false; updateState(); });
        const format = select([["literal", uiMessage("documentNotes.literalMode")], ["source", uiMessage("documentNotes.nativeMode")]] as const, textMode, uiMessage("documentNotes.noteContentMode"));
        format.onchange = () => { dirty = true; };
        editor.append(label(uiMessage("documentNotes.contentMode"), format), label(uiMessage("documentNotes.content"), input), element("p", "document-notes-help", uiMessage("documentNotes.nativeModeHelp")));
        return { input, body: (): NoteBodyInput => ({ mode: format.value as "literal" | "source", value: pendingValue }) };
    };
    const numberingControls = (preserve: boolean, initial: "none" | "pattern" = "pattern") => {
        const option = select([...(preserve ? [["preserve", uiMessage("documentNotes.preserveNumbering")] as [
                    string,
                    UiText
                ]] : []), ["none", uiMessage("documentNotes.captionWithoutNumber")], ["pattern", uiMessage("documentNotes.nativePatternNumbering")]], preserve ? "preserve" : initial, uiMessage("documentNotes.captionNumberingMode")), pattern = element("input", "document-notes-input");
        pattern.value = "1";
        setUiAttr(pattern, "aria-label", uiMessage("documentNotes.nativeNumberingPattern"));
        pattern.maxLength = 256;
        authorInputs?.bind(pattern, `notes:${++inputSlot}:numbering`);
        option.onchange = () => { pattern.disabled = option.value !== "pattern"; dirty = true; };
        pattern.oninput = () => { dirty = true; };
        pattern.disabled = option.value !== "pattern";
        editor.append(label(uiMessage("documentNotes.numbering"), option), label(uiMessage("documentNotes.nativeNumberingExamples"), pattern));
        const read = (): NotesNumbering | null => option.value === "preserve" ? null : option.value === "none" ? { kind: "none" } : { kind: "pattern", pattern: pattern.value };
        return Object.assign(read, { selectMode(value: "none" | "pattern") { option.value = value; pattern.disabled = value !== "pattern"; } });
    };
    const showNote = (note: DocumentNote) => {
        dirty = false;
        replaceUiChildren(editor, element("h3", "", () => `${uiTextValue(note.kind === "footnote" ? uiMessage("documentNotes.footnote") : uiMessage("documentNotes.endnote"))} ${uiTextValue(note.number || uiMessage("documentNotes.nativeNumberingPlaceholder"))}`));
        if (!host.editable()) {
            editor.append(element("p", "document-notes-reading", note.text));
            for (let index = 0; index < note.references.length; index++) {
                const reference = note.references[index];
                editor.append(button(displayMessage("documentNotes.revealMarker", { p0: index + 1 }), () => reveal(reference.range, reference.key), report));
            }
            updateState();
            return;
        }
        const edit = editorText(note.bodyRange?.raw ?? note.text, "source");
        if (!note.writable) {
            edit.input.readOnly = true;
            editor.append(element("p", "document-notes-help", note.reason ?? uiMessage("documentNotes.readOnly")));
        }
        const apply = button(uiMessage("documentNotes.applyNoteContent"), () => perform({ kind: "setNoteBody", construct: note.definition!.construct, body: edit.body() }), report);
        apply.dataset.write = "true";
        apply.dataset.capable = String(note.writable);
        editor.append(apply);
        for (let index = 0; index < note.references.length; index++) {
            const reference = note.references[index], row = element("div", "document-notes-reference");
            row.append(button(displayMessage("documentNotes.revealMarker", { p0: index + 1 }), () => reveal(reference.range, reference.key), report));
            if (reference.range) {
                const remove = button(reference.definition && note.references.length > 1 ? uiMessage("documentNotes.removeDefinitionMarker") : uiMessage("documentNotes.removeMarker"), () => perform({ kind: "removeNote", construct: reference.range!, allReferences: false }), report);
                remove.dataset.write = "true";
                remove.dataset.capable = String(note.writable && reference.writable);
                row.append(remove);
            }
            editor.append(row);
        }
        const repeat = button(uiMessage("documentNotes.repeatAtOriginalLocation"), () => perform({ kind: note.kind === "footnote" ? "repeatFootnote" : "repeatEndnote", at: insertion(host, basis.source), definition: note.definition!.construct }), report);
        repeat.dataset.write = "true";
        repeat.dataset.capable = String(note.writable);
        editor.append(repeat);
        const confirm = element("label", "document-notes-confirm"), checkbox = element("input");
        checkbox.type = "checkbox";
        setUiAttr(checkbox, "aria-label", uiMessage("documentNotes.confirmRemoveAllLabel"));
        confirm.append(checkbox, uiTextNode(displayMessage("documentNotes.confirmRemoveAllHelp", { p0: note.references.length })));
        const removeAll = button(uiMessage("documentNotes.removeNoteAndReferences"), () => {
            if (!checkbox.checked)
                throw new UiError(uiMessage("documentNotes.confirmRemovalScope"));
            return perform({ kind: "removeNote", construct: note.definition!.construct, allReferences: true });
        }, report);
        removeAll.dataset.write = "true";
        removeAll.dataset.capable = String(note.writable);
        editor.append(confirm, removeAll);
        updateState();
    };
    const showCaption = (caption: DocumentCaption) => {
        dirty = false;
        replaceUiChildren(editor, element("h3", "", displayMessage("documentNotes.captionHeading", { p0: uiDisplayParameter(categoryName(caption.category)), p1: uiDisplayParameter(caption.prefix || uiMessage("documentNotes.unnumbered")) })), element("p", "document-notes-help", uiMessage("documentNotes.captionOwnershipHelp")));
        if (!host.editable()) {
            editor.append(element("p", "document-notes-reading", caption.text));
            updateState();
            return;
        }
        const edit = editorText(caption.entry?.body?.raw ?? caption.text, "source"), position = select([["preserve", uiMessage("documentNotes.preserveCaptionPosition")], ["top", uiMessage("documentNotes.aboveObject")], ["bottom", uiMessage("documentNotes.belowObject")]] as const, "preserve", uiMessage("documentNotes.captionPosition"));
        editor.append(label(uiMessage("documentNotes.position"), position));
        const numbering = numberingControls(true);
        if (!caption.writable) {
            edit.input.readOnly = true;
            editor.append(element("p", "document-notes-help", caption.reason ?? uiMessage("documentNotes.readOnly")));
        }
        const apply = button(uiMessage("documentNotes.applyCaptionAndNumbering"), () => perform({ kind: "setCaption", construct: caption.entry!.construct, body: edit.body(), position: position.value === "preserve" ? null : position.value as "top" | "bottom", numbering: numbering() }), report);
        apply.dataset.write = "true";
        apply.dataset.capable = String(caption.writable);
        editor.append(apply, button(uiMessage("documentNotes.revealObjectSource"), () => reveal(caption.objectRange), report));
        updateState();
    };
    const showEquation = (equation: DocumentEquation) => {
        dirty = false;
        pendingValue = "";
        tracker = new DocumentNotesInputTracker("");
        replaceUiChildren(editor, element("h3", "", uiMessage("documentNotes.equationNumbering")), element("p", "document-notes-help", displayMessage("documentNotes.equationNumberingHelp", { p0: uiDisplayParameter(equation.numbered ? equation.prefix || uiMessage("documentNotes.nativeGenerated") : uiMessage("documentNotes.unnumberedOrUnverified")) })));
        if (!host.editable()) {
            updateState();
            return;
        }
        const numbering = numberingControls(true), apply = button(uiMessage("documentNotes.applyEquationNumbering"), () => {
            const chosen = numbering();
            if (!chosen || !equation.range)
                throw new UiError(uiMessage("documentNotes.chooseEquationNumbering"));
            return perform({ kind: "setEquationNumbering", construct: equation.range, numbering: chosen });
        }, report);
        apply.dataset.write = "true";
        apply.dataset.capable = String(equation.writable);
        editor.append(apply, button(uiMessage("documentNotes.revealEquationSource"), () => reveal(equation.range), report));
        if (!equation.writable)
            editor.append(element("p", "document-notes-help", uiMessage("documentNotes.equationSourceAmbiguous")));
        updateState();
    };
    const showNew = () => {
        dirty = false;
        replaceUiChildren(editor, element("h3", "", mode === "caption" ? uiMessage("documentNotes.addCaptionToObject") : displayMessage("documentNotes.insertNote", { p0: uiDisplayParameter(mode === "footnote" ? uiMessage("documentNotes.footnote") : uiMessage("documentNotes.endnote")) })));
        if (!host.editable()) {
            replaceUiChildren(editor, element("p", "document-notes-help", uiMessage("documentNotes.selectNoteHelp")));
            updateState();
            return;
        }
        const edit = editorText("", "literal");
        if (mode === "caption") {
            const object = select(model.objects.map(item => [item.key, item.title]), model.objects[0]?.key ?? "", uiMessage("documentNotes.captionObject")), position = select([["bottom", uiMessage("documentNotes.belowObject")], ["top", uiMessage("documentNotes.aboveObject")]] as const, "bottom", uiMessage("documentNotes.captionPosition"));
            editor.append(label(uiMessage("documentNotes.owningObject"), object), label(uiMessage("documentNotes.position"), position));
            const numbering = numberingControls(false, model.objects[0]?.category === "equation" ? "none" : "pattern");
            object.onchange = () => { dirty = true; numbering.selectMode(model.objects.find(item => item.key === object.value)?.category === "equation" ? "none" : "pattern"); };
            editor.append(element("p", "document-notes-help", uiMessage("documentNotes.chooseObjectHelp")));
            const apply = button(uiMessage("documentNotes.addCaptionToSelected"), () => {
                const chosen = model.objects.find(item => item.key === object.value);
                if (!chosen)
                    throw new UiError(uiMessage("documentNotes.uniqueObjectRequired"));
                return perform({ kind: "wrapCaption", object: chosen.range, category: chosen.category, body: edit.body(), position: position.value as "top" | "bottom", numbering: numbering()! });
            }, report);
            apply.dataset.write = "true";
            apply.dataset.capable = String(model.objects.length > 0);
            editor.append(apply);
        }
        else {
            editor.append(element("p", "document-notes-help", host.insertionRange ? uiMessage("documentNotes.caretInsertionHelp") : uiMessage("documentNotes.bodyEndInsertionHelp")));
            if (mode === "endnote")
                editor.append(element("p", "document-notes-help", uiMessage("documentNotes.endnoteModuleHelp")));
            if (mode === "endnote" && host.ensureEndnoteModule)
                editor.append(button(uiMessage("documentNotes.prepareEndnoteModule"), () => {
                    if (dirty)
                        throw new UiError(uiMessage("documentNotes.applyBeforePrepareModule"));
                    return run(async () => { await host.ensureEndnoteModule!(); });
                }, report));
            if (mode === "endnote" && host.recoverEndnoteModule)
                editor.append(button(uiMessage("documentNotes.inspectOriginalModuleInstall"), async () => {
                    if (busy)
                        return;
                    busy = true;
                    updateState();
                    try {
                        await host.recoverEndnoteModule!();
                    }
                    catch (value) {
                        report(value);
                    }
                    finally {
                        busy = false;
                        updateState();
                    }
                }, report));
            if (mode === "endnote" && host.retryEndnoteModule)
                editor.append(button(uiMessage("documentNotes.retryOriginalModuleInstall"), () => {
                    if (dirty)
                        throw new UiError(uiMessage("documentNotes.applyBeforeRetryModule"));
                    return run(async () => { await host.retryEndnoteModule!(); });
                }, report));
            if (mode === "endnote" && host.restartEndnoteModule)
                editor.append(button(uiMessage("documentNotes.restartCancelledModuleInstall"), () => {
                    if (dirty)
                        throw new UiError(uiMessage("documentNotes.applyBeforeRestartModule"));
                    return run(async () => { await host.restartEndnoteModule!(); });
                }, report));
            if (mode === "endnote" && host.settleEndnoteModule)
                editor.append(button(uiMessage("documentNotes.settleUninstalledModule"), () => {
                    if (dirty)
                        throw new UiError(uiMessage("documentNotes.applyBeforeSettleModule"));
                    return run(async () => { await host.settleEndnoteModule!(); });
                }, report));
            const apply = button(displayMessage("documentNotes.insertNote", { p0: uiDisplayParameter(mode === "footnote" ? uiMessage("documentNotes.footnote") : uiMessage("documentNotes.endnote")) }), () => perform({ kind: mode === "footnote" ? "insertFootnote" : "insertEndnote", at: insertion(host, basis.source), body: edit.body() }), report);
            apply.dataset.write = "true";
            apply.dataset.capable = "true";
            editor.append(apply);
        }
        updateState();
    };
    function updateState() {
        let valid = false;
        try {
            valid = current();
        }
        catch { /* Keep input; never refresh its target by text or row. */ }
        const readable = host.readable?.() ?? !!host.projection();
        for (const input of Array.from(form.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("input,textarea,select"))) {
            if (input.dataset.documentNotesRecovery === "true")
                continue;
            if (!readable) {
                if (input instanceof HTMLTextAreaElement)
                    input.value = "";
                else if (input instanceof HTMLInputElement && input.type !== "checkbox")
                    input.value = "";
                input.disabled = true;
            }
        }
        if (!readable) {
            replaceUiChildren(list);
            replaceUiChildren(editor, ...(recoveryDisplay ? [recoveryDisplay] : []));
            if (dirty && !retainedInput) {
                const added = tracker.recoveryInput();
                retainedInput = true;
                if (added) {
                    host.onPendingInput?.(added);
                    const recovery = element("textarea", "document-notes-input");
                    recovery.value = added;
                    recovery.readOnly = true;
                    recovery.dataset.documentNotesRecovery = "true";
                    setUiAttr(recovery, "aria-label", uiMessage("documentNotes.retainedNewInput"));
                    recoveryDisplay = label(uiMessage("documentNotes.retainedNewInputHelp"), recovery);
                    editor.append(recoveryDisplay);
                }
            }
            setUiText(status, uiMessage("documentNotes.sourceUnreadable"));
        }
        else if (!valid)
            setUiText(status, uiMessage("documentNotes.proposalReadOnly"));
        else
            setUiText(status, busy ? uiMessage("documentNotes.applying") : displayMessage("documentNotes.undoAfterApplyHelp", { p0: uiDisplayParameter(mode === "caption" ? uiMessage("documentNotes.captionScope") : uiMessage("documentNotes.noteScope")) }));
        for (const control of Array.from(form.querySelectorAll<HTMLButtonElement>("button[data-write]")))
            control.disabled = busy || localComposition || !valid || !host.editable() || control.dataset.capable === "false";
        for (const control of tabs)
            control.disabled = busy || localComposition;
    }
    function render() {
        replaceUiChildren(list);
        tabs.forEach((tab, index) => tab.setAttribute("aria-selected", String(["footnote", "endnote", "caption"][index] === mode)));
        if (host.editable())
            list.append(button(mode === "caption" ? uiMessage("documentNotes.addCaption") : displayMessage("documentNotes.newNote", { p0: uiDisplayParameter(mode === "footnote" ? uiMessage("documentNotes.footnote") : uiMessage("documentNotes.endnote")) }), () => {
                if (dirty) {
                    setUiText(status, uiMessage("documentNotes.applyBeforeNewProposal"));
                    return;
                }
                showNew();
            }, report));
        if (mode === "caption") {
            for (const caption of model.captions)
                list.append(button(() => `${uiTextValue(categoryName(caption.category))} · ${uiTextValue(caption.prefix || uiMessage("documentNotes.unnumbered"))} · ${uiTextValue(caption.text.slice(0, 70) || uiMessage("documentNotes.noCaption"))}`, () => {
                    if (dirty) {
                        setUiText(status, uiMessage("documentNotes.applyBeforeNewProposal"));
                        return;
                    }
                    showCaption(caption);
                }, report));
            for (const equation of model.equations)
                list.append(button(displayMessage("documentNotes.equationNumberingSummary", { p0: uiDisplayParameter(equation.prefix || (equation.numbered ? uiMessage("documentNotes.nativeNumbering") : uiMessage("documentNotes.unnumbered"))) }), () => {
                    if (dirty) {
                        setUiText(status, uiMessage("documentNotes.applyBeforeNewProposal"));
                        return;
                    }
                    showEquation(equation);
                }, report));
        }
        else {
            for (const note of model.notes.filter(note => note.kind === mode)) {
                const card = element("article", "document-notes-card");
                card.append(button(displayMessage("documentNotes.noteCardSummary", { p0: uiDisplayParameter(note.number || uiMessage("documentNotes.nativeNumbering")), p1: uiDisplayParameter(note.text.slice(0, 70) || uiMessage("documentNotes.emptyNote")), p2: note.references.length }), () => {
                    if (dirty) {
                        setUiText(status, uiMessage("documentNotes.applyBeforeNewProposal"));
                        return;
                    }
                    showNote(note);
                }, report));
                card.append(button(uiMessage("documentNotes.copyNoteText"), () => copyDocumentNote(host, basis, model.sourceHash, note.text), report));
                list.append(card);
            }
        }
        for (const entry of model.unlocated.filter(entry => host.editable() && (mode === "caption" ? entry.kind === "figure" : entry.kind.startsWith(mode)))) {
            const card = element("article", "document-notes-card");
            card.append(element("p", "document-notes-help", displayMessage("documentNotes.unlocatedNativeSource", { p0: uiDisplayParameter(entry.unavailableReason ? descriptionUiText(entry.unavailableReason) : uiMessage("documentNotes.noWritableProjection")) })), button(uiMessage("documentNotes.revealNativeSource"), () => reveal(entry.construct), report));
            list.append(card);
        }
        const wanted = targetId && (mode === "caption" ? model.captions.find(item => item.id === targetId) ?? model.equations.find(item => item.id === targetId) : model.notes.find(item => item.id === targetId && item.kind === mode));
        if (wanted) {
            targetId = undefined;
            if ("references" in wanted)
                showNote(wanted);
            else if ("category" in wanted)
                showCaption(wanted);
            else
                showEquation(wanted);
        }
        else
            showNew();
    }
    const cancel = button(uiMessage("documentNotes.cancelProposal"), close, report);
    form.append(title, toolbar, status, error, list, editor, cancel);
    dialog.append(form);
    document.body.append(dialog);
    const stop = host.controller.observeSurface(dialog, updateState);
    const retire = () => { updateState(); dialog.close(); }, panels = openPanels.get(host.controller) ?? new Set<() => void>();
    panels.add(retire);
    openPanels.set(host.controller, panels);
    dialog.addEventListener("cancel", event => { event.preventDefault(); close(); });
    try {
        render();
        dialog.showModal();
        tabs[["footnote", "endnote", "caption"].indexOf(mode)].focus();
        await new Promise<void>(resolve => dialog.addEventListener("close", () => resolve(), { once: true }));
    }
    finally {
        stopCloseGuard();
        authorInputs?.retire();
        panels.delete(retire);
        stop();
        disposeUiTree(dialog);
        dialog.remove();
        if (navigation.afterClose) {
            try {
                navigation.afterClose();
            }
            catch (value) {
                report(value);
            }
        }
        else if (previousFocus?.isConnected && host.projection())
            previousFocus.focus({ preventScroll: true });
        disposed = true;
    }
}
