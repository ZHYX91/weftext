import { joinUiText } from "./ui-locale";
import {apiErrorUiText,type ApiErrorBody} from "./api-error";
import { InterfaceLocaleControl, localLocalePreferences } from "./ui-locale-settings";
import { locale } from "./ui-locale";
import "./ui-locale.css";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./style.css";
import { originalCommitted, submitOriginal, TerminatedAttempt, type OriginalFact } from "./original-operation";
import { fileSaveReceipt } from "./file-save-result";
import { RecognitionPanel } from "./recognition-panel";
import { api, apiBlob, ApiError, operationId } from "./api";
import { mountNodeContents } from "./node-contents";
import { SourceEditor, applySourcePatches } from "./editor";
import { countSourceCodePoints } from "./source-statistics";
import { DeferredPaint } from "./deferred-paint";
import { EverydayNavigation, type EverydayEntry, type EverydayCommand } from "./everyday-navigation";
import { openEverydayMenu, openEverydayPicker } from "./everyday-ui";
import { everydayWritingCommands } from "./everyday-writing";
import { EmbeddedQueryController } from "./embedded-query";
import "./embedded-query.css";
import { HistoryPanel } from "./history-panel";
import { restoreBufferDisposition, type HistoryContext } from "./history-state";
import { DocumentSession, encodeLiteral, canRebindSaveAs, rebindCommittedSave, sourceConsumerNeedsComparison, sourceMoveComparisonMessage } from "./session";
import { WritingController, groupWritingRenderNodes, groupWritingEmptySlots, writingStructureIndex, textareaSourceDraft, type WritingMap, type WritingRange } from "./writing";
import { nativeElement } from "./native-node";
import { installNavigationLayout } from "./navigation-layout";
import { ensureOfflineFonts } from "./font-readiness";
import "./navigation-layout.css";
import "./writing.css";
import "./paged-preview.css";
import { createWritingToolbar, createStructureControls } from "./writing-controls";
import { createMathSelectionControls } from "./math-selection";
import { mountDocumentNotes, openDocumentNotes, type DocumentNotesMount } from "./document-notes";
import { AuthoringAssetsAdapter } from "./authoring-assets-adapter";
import { documentNotesHost, numberingHost, personalNumberingKeys, personalNumberingVisible, markPersonalHeading, displayPersonalHeadings, type AuthoringPanelsEnvironment } from "./authoring-panels-adapter";
import { openNumberingPanel } from "./numbering-panel";
import { headingMatchesDisplay } from "./heading-structure-state";
import { NoteInputRecovery } from "./note-input-recovery";
import { openLibraryCreation } from "./library-creation";
import "./document-notes.css";
import { WorkspaceTools } from "./workspace-tools";
import type { KnowledgeTextMapping, KnowledgeContext } from "./knowledge-panel";
import { PagedPreview } from "./paged-preview";
import { absoluteFileTarget, relativeFileTarget, quoteTypstString } from "./links";
import type { Inventory, InventoryEntry, OpenedDocument, Origin, Receipt, RecoveryInfo, RenderNode, RenderOutput, ViewMode } from "./types";
const paths: Record<string, string> = {
    book: "M4 4h6a3 3 0 0 1 3 3v14a4 4 0 0 0-4-2H4z M20 4h-4a3 3 0 0 0-3 3v14a4 4 0 0 1 4-2h3z",
    folder: "M3 6h7l2 2h9v11H3z",
    file: "M6 3h8l4 4v14H6z M14 3v5h4 M9 12h6 M9 16h6",
    plus: "M12 5v14 M5 12h14",
    search: "M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13 M16 16l5 5",
    arrow: "M4 12h15 M13 6l6 6-6 6",
    down: "M6 9l6 6 6-6",
    close: "M6 6l12 12 M18 6 6 18",
    save: "M4 3h13l3 3v15H4z M8 3v6h8V3 M8 21v-8h9v8",
    code: "M8 6 2 12l6 6 M16 6l6 6-6 6 M14 3l-4 18",
    eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12 M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
    menu: "M4 6h16 M4 12h16 M4 18h16",
    link: "M10 13a5 5 0 0 0 7 0l4-4a5 5 0 0 0-7-7l-3 3 M14 11a5 5 0 0 0-7 0l-4 4a5 5 0 0 0 7 7l3-3",
    check: "M5 12l4 4L20 5",
    alert: "M12 3 2 21h20z M12 9v5 M12 17v1",
    undo: "M8 4 3 9l5 5 M3 9h10a7 7 0 0 1 7 8",
    redo: "M16 4l5 5-5 5 M21 9H11a7 7 0 0 0-7 8",
};
function icon(name: string): SVGSVGElement {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "1.6");
    svg.setAttribute("stroke-linecap", "round");
    svg.setAttribute("stroke-linejoin", "round");
    svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(svg.namespaceURI, "path");
    path.setAttribute("d", paths[name] ?? paths.file);
    svg.append(path);
    return svg;
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] {
    const element = document.createElement(tag);
    element.className = className;
    if (text)
        setUiText(element, text);
    return element;
}
function button(label: UiText, onClick: () => void | Promise<void>, className = "", iconName?: string): HTMLButtonElement {
    const result = el("button", className);
    result.type = "button";
    if (iconName)
        result.append(icon(iconName));
    if (label)
        result.append(uiTextNode(label));
    result.onclick = () => { void Promise.resolve(onClick()).catch(showError); };
    return result;
}
const basename = (path: string): string => path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ?? path;
const normalize = (path: string): string => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
const labelStatus: Record<string, UiText> = { ordinary: uiMessage("shell.fileStatusPlain"), unmanaged: uiMessage("shell.fileStatusUnmanaged"), unadmitted: uiMessage("shell.fileStatusRegistrationPending"), admitted: uiMessage("queryColumn.note"), duplicate: uiMessage("shell.fileStatusDuplicateIdentity"), invalid: uiMessage("shell.fileStatusInvalidIdentity"), unsupported: uiMessage("shell.fileStatusUnsupported"), missing: uiMessage("shell.fileStatusMainMissing") };
const modes: {
    id: ViewMode;
    title: UiText;
    hint: UiText;
    short: string;
}[] = [
    { id: "source", title: uiMessage("shell.viewSource"), short: "V01", hint: uiMessage("shell.viewSourceDescription") },
    { id: "styled", title: uiMessage("shell.viewStyledSource"), short: "V02", hint: uiMessage("shell.viewStyledSourceDescription") },
    { id: "focus", title: uiMessage("shell.viewSyntaxOnDemand"), short: "V03", hint: uiMessage("shell.viewSyntaxOnDemandDescription") },
    { id: "write", title: uiMessage("shell.viewVisualEditing"), short: "V04", hint: uiMessage("shell.viewVisualEditingDescription") },
    { id: "read", title: uiMessage("shell.viewReading"), short: "V05", hint: uiMessage("shell.viewReadingDescription") },
];
interface LiveDocument {
    navigationKey: string;
    session: DocumentSession;
    editor: SourceEditor;
    writing: WritingController;
    queries: EmbeddedQueryController;
    mode: ViewMode;
    renderTimer?: ReturnType<typeof setTimeout>;
    draftTimer?: ReturnType<typeof setTimeout>;
    draftChain: Promise<void>;
    draftFailure: UiText | null;
    rendering: number;
    renderGeneration: number;
    lastSave?: {
        source: string;
        sourcePath: string;
        baseId: string;
        operationId: string;
        sessionId: string;
        path?: string;
        receipt?: Receipt & OriginalFact;
        deliverySessionId?: string;
        deliveryEnded?: boolean;
    };
    externalSource?: string;
    comparisonTarget?: OpenedDocument;
    saveAsChangedTarget?: string;
    pendingFragment?: string;
    scroll: Partial<Record<ViewMode, number>>;
    sourceScroll?: number;
    layout: "continuous" | "paged";
    deferredProjection: DeferredPaint;
    sourceTrashed?: boolean;
    sourceRefreshing?: boolean;
    notesMount?: DocumentNotesMount;
    authoringAssets: AuthoringAssetsAdapter;
}
function sourceConsumersAvailable(doc: LiveDocument): boolean { return !doc.sourceRefreshing && !sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget); }
const app = document.querySelector<HTMLDivElement>("#app")!;
const shell = el("div", "shell");
const sidebar = el("aside", "sidebar");
setUiAttr(sidebar, "aria-label", uiMessage("shell.libraryNavigation"));
const workspace = el("main", "workspace");
const topbar = el("header", "topbar");
const tabs = el("nav", "tabs");
setUiAttr(tabs, "aria-label", uiMessage("shell.openDocuments"));
const content = el("div", "content");
const workspaceBody = el("div", "workspace-body");
const statusbar = el("footer", "statusbar");
const toastHost = el("div", "toast-host");
toastHost.setAttribute("aria-live", "polite");
shell.append(sidebar, workspace);
workspace.append(topbar, tabs, workspaceBody, statusbar);
workspaceBody.append(content);
app.append(shell, toastHost);
let inventory: Inventory | null = null;
let documents: LiveDocument[] = [];
let bodyCloseFrozen = false;
const bodyOwners = new WeakMap<LiveDocument, {
    sessionId: string;
    generation: string;
}>();
function bodySnapshot(doc: LiveDocument) {
    const sessionId = doc.session.opened.sessionId;
    let owner = bodyOwners.get(doc);
    if (!owner || owner.sessionId !== sessionId) {
        owner = { sessionId, generation: crypto.randomUUID() };
        bodyOwners.set(doc, owner);
    }
    return { doc, sessionId, generation: owner.generation, baseId: doc.session.baseId, source: doc.session.source, serial: doc.session.serial, editorGeneration: doc.editor.generation };
}
function currentBody(snapshot: ReturnType<typeof bodySnapshot>): boolean {
    const { doc } = snapshot;
    return documents.includes(doc) && bodyOwners.get(doc)?.generation === snapshot.generation && doc.session.opened.sessionId === snapshot.sessionId && doc.session.baseId === snapshot.baseId && doc.session.source === snapshot.source && doc.session.serial === snapshot.serial && doc.editor.generation === snapshot.editorGeneration && doc.editor.source === snapshot.source && !doc.session.savePending && !doc.lastSave && !doc.editor.composing && !doc.writing.state.composing && !doc.writing.state.busy;
}
let preparedBody: {
    request: string;
    snapshots: ReturnType<typeof bodySnapshot>[];
} | undefined;
function thawBodyClose(): void {
    bodyCloseFrozen = false;
    preparedBody = undefined;
    for (const doc of documents)
        doc.editor.thawAfterClose(doc.mode === "read" || doc.mode === "write" || readOnlySource(doc));
}
/** Body-only close preparation. Other producer owners must supply their own
 * Core proof or block close; this never marks the whole surface ready. */
export const bodyCloseParticipant = {
    async prepare(request: object) {
        preparedBody = undefined;
        if (bodyCloseFrozen)
            throw new UiError(uiMessage("shell.closePreparationFrozen"));
        const owned = [...documents], snapshots: ReturnType<typeof bodySnapshot>[] = [], proofs = [];
        for (const doc of owned) {
            doc.writing.assertCloseReady();
            if (doc.editor.composing || doc.session.savePending || doc.lastSave || doc.queries.hasUnappliedAuthorInputs)
                throw new UiError(uiMessage("shell.closeAuthorInputPending"));
            await doc.writing.settleForSessionChange();
            await persistDraft(doc);
            const snapshot = bodySnapshot(doc);
            if (doc.session.lastDraftSource !== snapshot.source || doc.session.lastDraftSerial !== snapshot.serial)
                throw new UiError(uiMessage("shell.closeNewInputAfterDraft"));
            const sourceDigest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(snapshot.source)))].map(value => value.toString(16).padStart(2, "0")).join("");
            const result = await api<{
                sessionId: string;
                baseId: string;
                sourceDigest: string;
                recordDigest: string;
            }>("/api/desktop/document-flush", { sessionId: snapshot.sessionId, baseId: snapshot.baseId, sourceDigest });
            if (!currentBody(snapshot) || !result || result.sessionId !== snapshot.sessionId || result.baseId !== snapshot.baseId || result.sourceDigest !== sourceDigest || typeof result.recordDigest !== "string" || !/^[a-f0-9]{64}$/.test(result.recordDigest))
                throw new UiError(uiMessage("shell.closeRecoveryUnconfirmed"));
            doc.writing.assertCloseReady();
            snapshots.push(snapshot);
            proofs.push({ kind: "document" as const, ownerId: JSON.stringify([doc.session.opened.workspaceId ?? "local", doc.session.opened.nodeId ?? doc.session.path]), ownerGeneration: snapshot.generation, sessionId: snapshot.sessionId, baseId: snapshot.baseId, sourceDigest, recordDigest: result.recordDigest });
        }
        if (documents.length !== owned.length || owned.some((doc, at) => documents[at] !== doc) || snapshots.some(snapshot => !currentBody(snapshot)))
            throw new UiError(uiMessage("shell.closeBasisChanged"));
        preparedBody = { request: JSON.stringify(request), snapshots };
        return proofs;
    },
    freeze(request: object): boolean {
        if (!preparedBody || preparedBody.request !== JSON.stringify(request) || preparedBody.snapshots.length !== documents.length || preparedBody.snapshots.some((snapshot, at) => documents[at] !== snapshot.doc || !currentBody(snapshot) || snapshot.doc.queries.hasUnappliedAuthorInputs))
            return false;
        try {
            for (const snapshot of preparedBody.snapshots)
                snapshot.doc.writing.assertCloseReady();
        }
        catch {
            return false;
        }
        bodyCloseFrozen = true;
        for (const snapshot of preparedBody.snapshots)
            if (!snapshot.doc.editor.freezeForClose()) {
                thawBodyClose();
                return false;
            }
        return true;
    },
    thaw: thawBodyClose,
};
let current: LiveDocument | null = null;
let renderedDocument: LiveDocument | null = null;
let renderedMode: ViewMode | null = null;
const everydayNavigation = new EverydayNavigation();
let navigatingHistory = false;
let navigationGeneration = 0;
const navigationContexts = new Map<LiveDocument, {
    inventory: Inventory | null;
    query: string;
    folder: string | null;
    collapsed: string[];
    scroll: number;
    contentScroll: number;
    selection: string[];
}>();
const everydayBar = el("nav", "everyday-bar");
setUiAttr(everydayBar, "aria-label", uiMessage("shell.everydayNavigation"));
workspace.insertBefore(everydayBar, tabs);
let query = "";
let selectedFolder: string | null = null;
let sidebarComposing = false;
const contentSelections = new Set<string>();
let nodeContents: {
    dispose(): void;
} | undefined;
const folderViews = new Map<string, {
    query: string;
    scroll: number;
}>();
const collapsed = new Set<string>();
let previewVisible = localStorage.getItem("weftext-preview") !== "false";
let activeSourceOrigin: Origin | null = null;
let startupError: UiText | null = null;
let recoveries: RecoveryInfo[] = [];
let renderHost: HTMLElement | null = null;
let flowScroll: HTMLElement | null = null;
let flowHasProjection = false;
let pendingCreation: {
    route: string;
    payload: Record<string, unknown>;
    receipt?: OriginalFact;
} | null = null;
let pagedPreview: PagedPreview | null = null;
function captureNavigation(): void {
    if (!current)
        return;
    navigationContexts.set(current, { inventory, query, folder: selectedFolder, collapsed: [...collapsed], scroll: sidebar.querySelector(".file-tree")?.scrollTop ?? 0, contentScroll: sidebar.querySelector(".node-contents")?.scrollTop ?? 0, selection: [...contentSelections] });
}
function restoreNavigation(doc: LiveDocument): void {
    const saved = navigationContexts.get(doc);
    if (!saved)
        return;
    inventory = saved.inventory;
    query = saved.query;
    selectedFolder = saved.folder;
    collapsed.clear();
    saved.collapsed.forEach(path => collapsed.add(path));
    contentSelections.clear();
    saved.selection.forEach(path => contentSelections.add(path));
}
function everydayEntries(): EverydayEntry[] {
    const entries = new Map<string, EverydayEntry>();
    for (const doc of documents)
        entries.set(doc.navigationKey, { key: doc.navigationKey, title: basename(doc.session.path), detail: () => `${uiTextValue(doc.session.path)} · ${uiTextValue(saveState(doc))}`, open: () => activateDocument(doc) });
    for (const entry of inventory?.entries ?? []) {
        if (!entry.path.toLowerCase().endsWith(".typ") || entry.kind === "blocked")
            continue;
        const existing = documents.find(doc => normalize(doc.session.path) === normalize(entry.path));
        if (existing)
            continue;
        const root = inventory!.root, workspaceId = inventory!.workspaceId;
        entries.set(entry.path, { key: entry.path, title: entry.title || entry.name, detail: () => `${uiTextValue(entry.relativePath)} · ${uiTextValue(labelStatus[entry.identityStatus] ?? entry.identityStatus)}`, unavailable: ["duplicate", "invalid", "missing"].includes(entry.identityStatus) ? uiMessage("shell.identityCannotOpenExactly") : undefined,
            open: () => openInventoryEntry(entry, root, workspaceId) });
    }
    return [...entries.values()];
}
async function openInventoryEntry(entry: InventoryEntry, root: string, workspaceId?: string): Promise<void> {
    if (!entry.nodeId || !workspaceId) {
        await openDocument(entry.path, root);
        return;
    }
    const generation = ++navigationGeneration;
    await prepareSwitch();
    if (generation !== navigationGeneration)
        return;
    const opened = await api<OpenedDocument>("/api/data/open", { root, entityKey: `weftext-node://${workspaceId}/${entry.nodeId}` });
    if (generation !== navigationGeneration)
        return;
    if (opened.nodeId !== entry.nodeId || opened.workspaceId !== workspaceId)
        throw new UiError(uiMessage("shell.returnTargetMismatch"));
    await acceptResolvedDocument(opened);
}
function everydayCommands(): EverydayCommand[] {
    const doc = current;
    return [
        { id: "quick-open", label: uiMessage("shell.quickOpen"), detail: uiMessage("shell.quickOpenShortcut"), run: () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), toast) },
        { id: "recent", label: uiMessage("shell.recentVisits"), run: () => showRecent() },
        { id: "new-note", label: uiMessage("shell.newNote"), unavailable: !inventory?.workspaceId ? uiMessage("shell.openLibraryFirst") : undefined, run: createNote },
        { id: "open-file", label: uiMessage("shell.openOutsideTypst"), run: openOutsideFile },
        { id: "save", label: uiMessage("shell.saveCurrentNote"), unavailable: !doc ? uiMessage("shell.openNoteFirst") : undefined, run: () => saveDocument(doc!) },
        { id: "resources", label: uiMessage("shell.browseImportResources"), run: async () => { await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("files"); } },
        { id: "relationship-graph", label: uiMessage("shell.openRelationshipGraph"), unavailable: !doc ? uiMessage("shell.openNoteFirst") : undefined, run: async () => { await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("graph"); } },
        { id: "heading-structure", label: uiMessage("shell.openHeadingStructure"), unavailable: !doc ? uiMessage("shell.openNoteFirst") : undefined, run: async () => { await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("headings"); } },
        ...everydayWritingCommands(doc?.writing, !!doc && sourceConsumersAvailable(doc) && doc.mode !== "read" && !readOnlySource(doc) && !doc.writing.state.composing && !doc.editor.composing, insertEverydayQuery),
    ];
}
function showRecent(): void {
    const entries = everydayEntries();
    openEverydayPicker(uiMessage("shell.recentVisits"), everydayNavigation.recent.flatMap(key => entries.filter(entry => entry.key === key)), toast);
}
async function stepNavigation(direction: -1 | 1): Promise<void> {
    const key = everydayNavigation.target(direction), target = documents.find(doc => doc.navigationKey === key);
    if (!key || !target)
        return;
    navigatingHistory = true;
    try {
        await activateDocument(target);
        if (current === target)
            everydayNavigation.arrive(direction, key);
    }
    finally {
        navigatingHistory = false;
        renderEverydayBar();
    }
}
function renderEverydayBar(): void {
    if (current && !navigatingHistory)
        everydayNavigation.visit(current.navigationKey);
    everydayBar.replaceChildren();
    for (const [direction, label] of [[-1, uiMessage("shell.back")], [1, uiMessage("shell.forward")]] as const) {
        const control = button(label, () => stepNavigation(direction));
        control.disabled = !everydayNavigation.target(direction);
        everydayBar.append(control);
    }
    everydayBar.append(button(uiMessage("shell.quickOpen"), () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), toast)), button(uiMessage("shell.recentVisits"), showRecent), button(uiMessage("cli.commandsHeading"), () => openEverydayMenu(uiMessage("cli.commandsHeading"), everydayCommands(), toast)), button(uiMessage("shell.insertBlockActions"), () => openEverydayMenu(uiMessage("shell.insertBlockActions"), everydayCommands().filter(item => /^(insert-|convert-|move-|undo|redo|resources)/.test(item.id)), toast)));
}
async function insertEverydayQuery(): Promise<void> {
    const doc = current;
    if (!doc?.session.opened.nodeId || !doc.session.opened.workspaceId)
        throw new UiError(uiMessage("shell.embeddedQueryRequirements"));
    await prepareSwitch();
    const basis = { sessionId: doc.session.opened.sessionId, source: doc.session.source, serial: doc.session.serial, baseId: doc.session.baseId };
    const operation = operationId();
    await modal<void>(uiMessage("shell.insertEmbeddedQuery"), uiMessage("shell.embeddedQueryDescription"), (form, close) => {
        const label = el("input", "text-input");
        label.value = "笔记列表";
        label.required = true;
        const condition = el("input", "text-input");
        setUiAttr(condition, "placeholder", uiMessage("shell.queryEmptyScopePlaceholder"));
        const name = el("label", "field-label", uiMessage("shell.queryName"));
        name.append(label);
        const where = el("label", "field-label", uiMessage("shell.queryFilter"));
        where.append(condition);
        const kind = el("select", "text-input");
        setUiAttr(kind, "aria-label", uiMessage("shell.querySource"));
        for (const [value, text] of [["new", uiMessage("shell.queryNewDefinition")], ["call", uiMessage("shell.queryReuseSavedDefinition")]] as const) {
            const option = el("option", "", text);
            option.value = value;
            kind.append(option);
        }
        const shared = el("fieldset");
        shared.hidden = true;
        shared.append(el("legend", "", uiMessage("shell.querySelectDefinitionOwner")));
        const owners = el("select", "text-input");
        setUiAttr(owners, "aria-label", uiMessage("shell.queryDefinitionOwnerNote"));
        const definitions = el("select", "text-input");
        setUiAttr(definitions, "aria-label", uiMessage("shell.queryReusedDefinition"));
        const mode = el("select", "text-input");
        setUiAttr(mode, "aria-label", uiMessage("shell.queryReuseVersionMode"));
        for (const [value, text] of [["current", uiMessage("queryPanel.currentDefinition")], ["pinned", uiMessage("shell.queryPinExactVersion")]] as const) {
            const option = el("option", "", text);
            option.value = value;
            mode.append(option);
        }
        const revision = el("input", "text-input");
        setUiAttr(revision, "aria-label", uiMessage("shell.queryReusedExactVersion"));
        revision.disabled = true;
        mode.onchange = () => { revision.disabled = mode.value !== "pinned"; };
        if (inventory?.root === doc.session.opened.root)
            for (const entry of inventory.entries.filter(entry => entry.kind === "node" && entry.identityStatus === "admitted" && entry.nodeId)) {
                const option = el("option", "", entry.title || entry.name);
                option.value = `weftext-node://${doc.session.opened.workspaceId}/${entry.nodeId}`;
                owners.append(option);
            }
        let loadedOwner = "";
        const load = button(uiMessage("shell.queryLoadOwnerDefinitions"), async () => {
            const owner = owners.value;
            if (!owner)
                throw new UiError(uiMessage("shell.querySelectActualOwnerFirst"));
            load.disabled = true;
            try {
                const result = await api<{
                    ownerRevision: string | null;
                    definitions: {
                        key: string;
                        label: string;
                    }[];
                }>("/api/query/definitions", { root: doc.session.opened.root, owner });
                if (!form.isConnected || owners.value !== owner)
                    return;
                definitions.replaceChildren();
                for (const definition of result.definitions) {
                    const option = el("option", "", definition.label.replace(/^weftext-query-/, ""));
                    option.value = definition.key;
                    definitions.append(option);
                }
                revision.value = result.ownerRevision ?? "";
                loadedOwner = owner;
                if (!result.definitions.length) {
                    setUiText(error, uiMessage("shell.queryOwnerHasNoDefinition"));
                    error.hidden = false;
                }
            }
            catch (failure) {
                setUiText(error, failure instanceof Error ? errorUiText(failure) : String(failure));
                error.hidden = false;
            }
            finally {
                load.disabled = false;
            }
        });
        owners.onchange = () => { definitions.replaceChildren(); loadedOwner = ""; };
        shared.append(owners, load, definitions, mode, revision, el("p", "field-help", uiMessage("shell.queryReuseDescription")));
        kind.onchange = () => { shared.hidden = kind.value !== "call"; where.hidden = kind.value === "call"; };
        const error = el("p", "writing-error");
        error.setAttribute("role", "alert");
        error.hidden = true;
        const submit = el("button", "button-primary", uiMessage("shell.insertQuery"));
        submit.type = "submit";
        form.append(name, kind, where, shared, error, button(uiMessage("common.cancel"), () => close(null)), submit);
        const selection = () => JSON.stringify([kind.value, owners.value, definitions.value, mode.value, revision.value]);
        const available = () => current === doc && doc.session.opened.sessionId === basis.sessionId && doc.session.baseId === basis.baseId && doc.session.source === basis.source && doc.session.serial === basis.serial && sourceConsumersAvailable(doc) && !doc.sourceTrashed && !readOnlySource(doc) && doc.mode !== "read";
        form.onsubmit = event => {
            event.preventDefault();
            submit.disabled = true;
            error.hidden = true;
            void (async () => {
                if (!available())
                    throw new UiError(uiMessage("shell.querySourceChangedDuringConfirmation"));
                const nameValue = label.value, conditionValue = condition.value, selected = selection();
                const nativeName = nameValue.trim().replace(/\s+/g, "-");
                if (!/^[\p{L}\p{N}_.:-]+$/u.test(nativeName))
                    throw new UiError(uiMessage("shell.queryNameCharacters"));
                let plan: import("./query-panel").QueryPlan = { scope: { descendants: true, lifecycle: "live" }, types: [], ...(conditionValue ? { where: conditionValue } : {}), select: [{ name: "title", expr: "title" }], order: [], take: 100 };
                if (kind.value === "call") {
                    if (loadedOwner !== owners.value || !definitions.value || mode.value === "pinned" && !revision.value)
                        throw new UiError(uiMessage("shell.querySelectExactDefinitionVersion"));
                    plan = { scope: { descendants: true, lifecycle: "live" }, types: [], select: [], order: [], advanced: { call: { owner: owners.value, selection: mode.value === "pinned" ? { mode: "pinned", key: definitions.value, revision: revision.value } : { mode: "current", key: definitions.value }, arguments: {} } } };
                }
                const result = await api<{
                    draftOnly: boolean;
                    source: string;
                    serial: number;
                    patchUnit: string;
                    patches: {
                        from: number;
                        to: number;
                        text: string;
                        expected: string;
                    }[];
                }>("/api/query/definition/draft", { ...basis, operationId: operation, label: `weftext-query-${nativeName}`, plan });
                if (!available() || !form.isConnected || label.value !== nameValue || condition.value !== conditionValue || selection() !== selected || !result.draftOnly || result.serial !== basis.serial || result.patchUnit !== "utf16")
                    throw new UiError(uiMessage("shell.queryProposalOrInputChanged"));
                const patches = result.patches.map(patch => ({ ...patch, insert: patch.text }));
                if (applySourcePatches(basis.source, patches) !== result.source)
                    throw new UiError(uiMessage("shell.queryPatchMismatch"));
                doc.editor.applyPatches(basis.source, patches, undefined, "input.query.insert");
                close(undefined);
                await persistDraft(doc);
                scheduleRender(doc, 0);
            })().catch(failure => {
                if (!form.isConnected) {
                    showError(failure);
                    return;
                }
                setUiText(error, failure instanceof ApiError && failure.code === "schemaRevisionRequired" ? uiMessage("shell.queryInstallFieldsFirst") : failure instanceof Error ? errorUiText(failure) : String(failure));
                error.hidden = false;
                submit.disabled = false;
            });
        };
    });
}
let projectionMappings: KnowledgeTextMapping[] = [];
let readingSelection: {
    sessionId: string;
    serial: number;
    selection: NonNullable<KnowledgeContext["selection"]>;
} | null = null;
const recognitionPanel = new RecognitionPanel({
    request: (path, body) => api(`/api/recognition${path}`, body),
    identity: () => current ? current.session.opened.root : null,
    committed: root => workspaceTools.invalidateQueryEnvironment(root),
    target: () => current ? { sessionId: current.session.opened.sessionId, parent: selectedFolder ?? current.session.opened.root, name: "资料导入" } : null,
    notify: toast,
    created: async (receipt, target) => {
        const node = receipt.nodeRefs[0];
        if (!node) throw new UiError(uiMessage("shell.importCreatedNoteUnavailable"));
        await prepareSwitch();
        const opened = await api<OpenedDocument>("/api/recognition/open", { operationId: receipt.operationId, target, node });
        await acceptCreatedDocument(opened);
    },
});
let recognitionSession = "";
const workspaceTools = new WorkspaceTools({
    revealHeading: heading => {
        const doc = current;
        if (!doc || !doc.session.projectionCurrent || !sourceConsumersAvailable(doc) || !renderHost || !renderHost.isConnected || !renderHost.getClientRects().length || !headingMatchesDisplay(doc.session.projection?.tree, heading))
            return false;
        const target = Array.from(renderHost.querySelectorAll<HTMLElement>("[data-document-node-key]")).find(node => node.dataset.documentNodeKey === heading.key);
        if (!target)
            return false;
        if (!target.hasAttribute("tabindex"))
            target.tabIndex = -1;
        target.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
        target.focus({ preventScroll: true });
        return true;
    },
    embeddedQueries: () => current && sourceConsumersAvailable(current) ? current.queries.outputSelections() : [],
    getDocument: () => current && !current.sourceTrashed && sourceConsumersAvailable(current) ? { sessionId: current.session.opened.sessionId, source: current.session.source, serial: current.session.serial, baseId: current.session.baseId, path: current.session.path, root: current.session.opened.root, nodeId: current.session.opened.nodeId, workspaceId: current.session.opened.workspaceId, dirty: current.session.dirty } : null,
    selection: () => {
        if (!current)
            return undefined;
        if (readingSelection?.sessionId === current.session.opened.sessionId && readingSelection.serial === current.session.serial)
            return readingSelection.selection;
        const range = current.editor.selection;
        return range.from === range.to ? undefined : range;
    },
    beforeAction: prepareSwitch, notify: toast, openDocument, openResolved: acceptResolvedDocument,
    refreshLibrary: async (root) => {
        if (inventory && normalize(inventory.root) === normalize(root))
            await refreshInventory();
    },
    syncApplied: async (root) => {
        workspaceTools.invalidateQueryEnvironment(root);
        if (inventory && normalize(inventory.root) === normalize(root))
            await refreshInventory();
        for (const doc of documents.filter(d => normalize(d.session.opened.root) === normalize(root))) {
            const originalSession = doc.session.opened.sessionId;
            const stillOpen = (): boolean => documents.includes(doc) && doc.session.opened.sessionId === originalSession;
            doc.sourceRefreshing = true;
            doc.renderGeneration++;
            clearTimeout(doc.renderTimer);
            doc.queries.invalidate();
            doc.session.projection = null;
            doc.editor.clearTokens();
            if (doc === current) {
                pagedPreview?.invalidate(uiMessage("shell.syncCheckingDraftRetained"));
                workspaceTools.documentChanged();
            }
            try {
                // A committed sync can arrive during later preedit. Settle/checkpoint
                // input before replacing any surface, retaining shared editor/Undo.
                await doc.writing.settleForSessionChange();
                if (!stillOpen())
                    continue;
                if (!readOnlySource(doc))
                    await persistDraft(doc);
                if (!stillOpen())
                    continue;
                let fresh: OpenedDocument | undefined, unavailable = false;
                try {
                    if (doc.session.opened.nodeId && doc.session.opened.workspaceId)
                        fresh = await api<OpenedDocument>("/api/data/open", { root, entityKey: `weftext-node://${doc.session.opened.workspaceId}/${doc.session.opened.nodeId}` });
                }
                catch {
                    unavailable = true;
                }
                await doc.writing.deferUntilCompositionEnds(() => {
                    if (!stillOpen())
                        return;
                    if (fresh && (fresh.source !== doc.session.baseSource || normalize(fresh.path) !== normalize(doc.session.path))) {
                        doc.externalSource = fresh.source;
                        doc.comparisonTarget = fresh;
                        doc.session.error = uiMessage("shell.syncDiskChanged");
                    }
                    else if (unavailable)
                        doc.session.error = uiMessage("shell.syncNoteUnreadable");
                    // Equality cannot prove unchanged sidecars or compilation inputs.
                    // A moved writer waits for explicit comparison instead of compiling
                    // its old pathname or silently installing the fresh Save basis.
                    doc.renderGeneration++;
                    doc.queries.invalidate();
                    doc.session.projection = null;
                });
            }
            finally {
                doc.sourceRefreshing = false;
                if (stillOpen())
                    scheduleRender(doc, 0);
            }
        }
        const displayed = current;
        if (displayed && normalize(displayed.session.opened.root) === normalize(root)) {
            await displayed.writing.deferUntilCompositionEnds(() => {
                if (current !== displayed)
                    return;
                pagedPreview?.invalidate();
                if (!displayed.writing.retainInput)
                    renderContent();
                else
                    refreshChrome();
            });
        }
        await workspaceTools.refreshAfterSync(root);
        renderSidebar();
    },
    canEditSource: () => !!current && sourceConsumersAvailable(current) && current.mode !== "read" && !readOnlySource(current) && !current.writing.state.composing,
    savedDefinition: async (result, basis) => {
        const receipt = result as {
            document?: OpenedDocument;
            committedSource?: string;
            documentSourceMatchesCommit?: boolean;
        };
        const opened = receipt.document;
        if (!opened || typeof opened.sessionId !== "string" || typeof opened.source !== "string")
            throw new UiError(uiMessage("shell.querySaveFileReceiptMissing"));
        workspaceTools.invalidateQueryEnvironment(basis.root);
        const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
        if (!doc)
            throw new UiError(uiMessage("shell.querySavedReopen"));
        if (receipt.documentSourceMatchesCommit !== true || receipt.committedSource !== opened.source) {
            doc.session.error = uiMessage("shell.queryCommittedDiskChanged");
            if (doc === current)
                renderContent();
            return;
        }
        await doc.writing.settleForSessionChange();
        if (doc.session.source !== basis.source || doc.session.serial !== basis.serial || doc.session.baseId !== basis.baseId) {
            doc.session.error = uiMessage("shell.querySavedNewInputRetained");
            if (doc === current)
                renderContent();
            return;
        }
        doc.session.opened = opened;
        doc.editor.clearTokens();
        if (doc.editor.source !== opened.source)
            doc.editor.apply(0, doc.editor.source.length, opened.source);
        doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
        doc.lastSave = undefined;
        doc.session.projection = null;
        await persistDraft(doc);
        scheduleRender(doc, 0);
        await refreshInventory();
        refreshChrome();
    },
    selectSource: (from, to) => {
        if (current) {
            current.mode = "source";
            current.editor.clearReveal();
            renderContent();
            current.editor.select(from, to);
        }
    },
    applyPatch: async (patch, basis) => {
        const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
        if (!doc || doc.session.source !== basis.source || doc.session.serial !== basis.serial || doc.session.baseId !== basis.baseId || doc.mode === "read" || readOnlySource(doc) || doc.writing.state.composing)
            throw new UiError(uiMessage("shell.oldEditCannotApply"));
        await doc.writing.flush();
        if (doc.session.source !== basis.source || doc.session.serial !== basis.serial)
            throw new UiError(uiMessage("shell.inputEnteredDraft"));
        doc.editor.apply(patch.from, patch.to, patch.text);
        await persistDraft(doc);
        scheduleRender(doc, 0);
    },
    created: acceptCreatedDocument,
    navigate: async (target) => {
        if (typeof target === "string")
            await navigateLink(target);
        else {
            if (!inventory?.workspaceId || target.workspaceId !== inventory.workspaceId)
                throw new UiError(uiMessage("shell.targetOutsideCurrentLibrary"));
            await prepareSwitch();
            await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: inventory.root, entityKey: `weftext-node://${target.workspaceId}/${target.nodeId}` }));
        }
    },
    applyReceipt: async (receipt, context) => {
        if (!originalCommitted(receipt)) throw new UiError(uiMessage("shell.originalCommittedReceiptRequired"));
        workspaceTools.invalidateQueryEnvironment(context.root);
        const opened = receipt.document as OpenedDocument | undefined;
        const doc = documents.find(item => item.session.opened.sessionId === context.sessionId || Boolean(context.nodeId && context.workspaceId && item.session.opened.nodeId === context.nodeId && item.session.opened.workspaceId === context.workspaceId));
        let deliveryPending: UiText | null = receipt.documentError ? apiErrorUiText(receipt.documentError) : receipt.documentSourceMatchesCommit === false ? uiMessage("shell.committedBodyChanged") : null;
        if (doc)
            await doc.writing.settleForSessionChange();
        if (doc && (receipt.documentSourceMatchesCommit === false || receipt.documentError)) {
            doc.session.error = uiMessage("shell.committedContentReceiptMismatch");
            if (doc === current)
                renderContent();
        }
        else if (receipt.documentDisposition === "trashed" && doc) {
            doc.sourceTrashed = true;
            doc.editor.clearTokens();
            doc.session.error = uiMessage("shell.noteRecycledDraftRetained");
            await persistDraft(doc);
            if (current === doc)
                renderContent();
        }
        else if (opened && receipt.documentDisposition === "created") {
            const receiving = documents.find(item => item.session.opened.nodeId === opened.nodeId && item.session.opened.workspaceId === opened.workspaceId);
            if (current === doc || receiving === current) await acceptCreatedDocument(opened);
            else deliveryPending = displayMessage("shell.createdReturnForOriginalDelivery", {name:uiDisplayParameter(basename(opened.path))});
        }
        else if (opened && doc && opened.source === doc.session.baseSource) {
            // Namespace changes keep the same saved bytes. Preserve newer local input,
            // the shared editor and Undo while rebinding the real Core save session.
            doc.session.opened = opened;
            doc.editor.clearTokens();
            doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
            doc.lastSave = undefined;
            doc.session.projection = null;
            await persistDraft(doc);
            scheduleRender(doc, 0);
            if (current === doc)
                renderContent();
        }
        else if (opened && doc && doc.session.source === context.source && doc.session.serial === context.serial && !doc.session.dirty) {
            doc.session.opened = opened;
            doc.editor.clearTokens();
            if (doc.editor.source !== opened.source)
                doc.editor.apply(0, doc.editor.source.length, opened.source);
            doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
            doc.lastSave = undefined;
            doc.session.projection = null;
            await persistDraft(doc);
            scheduleRender(doc, 0);
            if (current === doc)
                renderContent();
        }
        else if (opened && doc) {
            doc.session.error = uiMessage("shell.fileOperationCompletedNewInputRetained");
            refreshChrome();
            doc.comparisonTarget = opened;
            deliveryPending = doc.session.error;
        }
        if (receipt.documentDisposition === "created" && !opened) deliveryPending ||= uiMessage("shell.createdDocumentNotDelivered");
        const affected = receipt.affectedDocuments as {
            priorSessionId: string;
            document?: OpenedDocument;
            documentDisposition: string;
            documentError?: ApiErrorBody;
        }[] | undefined;
        for (const effect of affected ?? []) {
            const related = documents.find(item => item.session.opened.sessionId === effect.priorSessionId);
            if (!related)
                continue;
            await related.writing.settleForSessionChange();
            if (effect.documentDisposition === "trashed") {
                related.sourceTrashed = true;
                related.editor.clearTokens();
                related.session.error = uiMessage("shell.parentFolderRecycled");
                await persistDraft(related);
            }
            else if (effect.document && effect.document.source === related.session.baseSource) {
                related.session.opened = effect.document;
                related.editor.clearTokens();
                related.session.acknowledgeSave(effect.document.source, effect.document.baseId, effect.document.path);
                related.lastSave = undefined;
                related.session.projection = null;
                await persistDraft(related);
                scheduleRender(related, 0);
            }
            else {
                related.session.error = displayMessage("shell.folderOperationCompleted", { p0: uiDisplayParameter(effect.documentError ? apiErrorUiText(effect.documentError) : uiMessage("shell.childNoteChanged")) });
                if (effect.document)
                    related.saveAsChangedTarget = effect.document.path;
                deliveryPending ||= related.session.error;
            }
            if (related === current)
                renderContent();
        }
        await refreshInventory();
        if (deliveryPending) throw new UiError(deliveryPending);
    },
    stylesChanged: async (root) => {
        pagedPreview?.invalidate(uiMessage("shell.libraryStyleRecompiling"));
        for (const doc of documents.filter(item => normalize(item.session.opened.root) === normalize(root))) {
            doc.session.projection = null;
            doc.editor.clearTokens();
            scheduleRender(doc, 0);
        }
        if (current?.layout === "paged")
            await pagedPreview?.render();
        await refreshInventory();
    },
});
workspace.insertBefore(workspaceTools.bar, workspaceBody);
const interfaceLocale = new InterfaceLocaleControl(localLocalePreferences(api));
workspaceTools.bar.append(interfaceLocale.element);
document.documentElement.lang = locale.current;
locale.subscribe(value => { document.documentElement.lang = value; });
window.addEventListener("pagehide", event => { if (!event.persisted)
    interfaceLocale.dispose(); });
workspaceBody.append(workspaceTools.inspector);
const navigationLayout = installNavigationLayout({ shell, sidebar, workspaceBody, inspector: workspaceTools.inspector, toolbar: workspaceTools.bar, notify: toast });
function historyContext(doc: LiveDocument | null): HistoryContext | null {
  if (!doc) return null;
  const { session } = doc;
  const source = doc.writing.recoverySource() ?? session.source;
  return { ownerKey: doc.navigationKey, sessionId: session.opened.sessionId, source, serial: session.serial,
    baseId: session.baseId, baseSource: session.baseSource, path: session.path,
    root: session.opened.root, nodeId: session.opened.nodeId, workspaceId: session.opened.workspaceId,
    dirty: source !== session.baseSource, readOnly: doc.mode === "read" || readOnlySource(doc) || doc.sourceTrashed,
    savePending: session.savePending, unresolvedSave: !!pendingSave(doc) || !!session.opened.recoveredDraftConflict };
}
const historyPanel = new HistoryPanel({
  getDocument: () => historyContext(current), request: api, prepareRestore: prepareSwitch, notify: toast,
  restorePending: () => { refreshChrome(); },
  restored: async (result, basis) => {
    if (result.receipt.status === "saved") workspaceTools.invalidateQueryEnvironment(basis.root);
    const doc = documents.find(item => item.navigationKey === basis.ownerKey && item.session.opened.nodeId === basis.nodeId && item.session.opened.workspaceId === basis.workspaceId);
    if (!doc) throw new UiError(uiMessage("shell.olderVersionSavedReopen"));
    await doc.writing.settleForSessionChange();
    const prior = doc.session.opened;
    const fact = await api<Receipt & OriginalFact & { committedSource?: string; document?: OpenedDocument; documentSourceMatchesCommit?: boolean }>("/api/operations/status", { operationId: result.receipt.operationId });
    const opened = fact.document;
    if (doc.session.opened !== prior || fact.operationId !== result.receipt.operationId || fact.status !== "committed" || fact.baseId !== result.receipt.baseId || fact.path !== result.receipt.path || fact.committedSource !== result.source || !opened || opened.nodeId !== basis.nodeId || opened.workspaceId !== basis.workspaceId || opened.kind !== "managed" || opened.source !== result.source || fact.documentSourceMatchesCommit !== true || fact.documentError) {
      if (opened && opened.nodeId === basis.nodeId && opened.workspaceId === basis.workspaceId) { doc.comparisonTarget = opened; doc.externalSource = opened.source; }
      throw new UiError(uiMessage("shell.restoreCommittedCurrentSessionChanged"));
    }
    const variants = [...(doc.session.opened.recoveredDraftVariants ?? []), ...(opened.recoveredDraftVariants ?? []), ...result.protectedDrafts];
    if (opened.recoveredDraftConflict) await api("/api/draft/resolve", { sessionId: opened.sessionId, action: "mergeRecovered", source: doc.session.source });
    await doc.writing.settleForSessionChange();
    if (doc.session.opened !== prior) throw new UiError(uiMessage("shell.deliverySessionChanged"));
    const disposition = restoreBufferDisposition(basis, historyContext(doc));
    if (disposition === "unrelated" || result.receipt.status !== "saved") throw new UiError(uiMessage("shell.restoreReceiptNoteMismatch"));
    opened.recoveredDraftVariants = variants.filter((value, index) => variants.findIndex(other => other.path === value.path && other.baseSource === value.baseSource && other.source === value.source && other.baseKnown === value.baseKnown) === index);
    // Keep the shared editor and Undo. Only a still-clean, unchanged buffer adopts
    // the restored text; any original or late input remains a Draft over the new Base.
    if (disposition === "replaceClean" && doc.editor.source !== result.source) doc.editor.apply(0, doc.editor.source.length, result.source);
    doc.session.opened = opened; opened.recoveredDraftConflict = false;
    doc.session.acknowledgeSave(result.source, opened.baseId, opened.path);
    doc.lastSave = undefined; doc.session.projection = null; doc.editor.clearTokens(); doc.queries.invalidate();
    await persistDraft(doc); scheduleRender(doc, 0); await refreshInventory(); refreshChrome();
  },
  finishDelivery: async (operationId, basis) => {
    const doc = documents.find(item => item.navigationKey === basis.ownerKey && item.session.opened.nodeId === basis.nodeId && item.session.opened.workspaceId === basis.workspaceId);
    if (!doc) throw new UiError(uiMessage("shell.compareOriginalBuffer"));
    const fact = await api<OriginalFact & { document?: OpenedDocument }>("/api/operations/status", { operationId });
    if (fact.operationId !== operationId || fact.status !== "committed" || !fact.document || fact.document.nodeId !== basis.nodeId || fact.document.workspaceId !== basis.workspaceId) throw new UiError(uiMessage("shell.originalCurrentObjectUnreadable"));
    doc.comparisonTarget = fact.document; doc.externalSource = fact.document.source;
    const prior = doc.session.opened;
    await showDraftComparison(doc);
    return doc.session.opened !== prior && !doc.comparisonTarget;
  },
});
function saveBusy(doc: LiveDocument): boolean { return doc.session.savePending || historyPanel.hasPending(historyContext(doc)); }
function pendingSave(doc: LiveDocument): LiveDocument["lastSave"] { return doc.lastSave?.deliveryEnded ? undefined : doc.lastSave; }

async function refreshInventory(): Promise<void> {
    if (inventory)
        inventory = await api<Inventory>("/api/library/open", { path: inventory.root });
    renderSidebar();
}
async function acceptCreatedDocument(opened: OpenedDocument): Promise<void> {
    if (!opened.nodeId || !opened.workspaceId || opened.kind !== "managed") throw new UiError(uiMessage("shell.createdNodeIdentityRequired"));
    workspaceTools.invalidateQueryEnvironment(opened.root);
    await prepareSwitch();
    const restored = documents.find(item => item.session.opened.nodeId === opened.nodeId && item.session.opened.workspaceId === opened.workspaceId);
    if (restored && (pendingSave(restored) || opened.source !== restored.session.baseSource)) {
        restored.comparisonTarget = opened; restored.externalSource = opened.source;
        throw new UiError(uiMessage("shell.createdNodeAlreadyOpenPending"));
    }
    if (restored) {
        await restored.writing.settleForSessionChange();
        restored.session.opened = opened;
        restored.editor.clearTokens();
        restored.sourceTrashed = false;
        restored.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
        restored.lastSave = undefined;
        restored.session.projection = null;
        await persistDraft(restored);
        current = restored;
        activeSourceOrigin = null;
        renderContent();
        scheduleRender(restored, 0);
    }
    else
        addDocument(opened);
    await refreshInventory();
}
async function acceptResolvedDocument(opened: OpenedDocument): Promise<void> {
    const generation = ++navigationGeneration;
    await prepareSwitch();
    if (generation !== navigationGeneration)
        return;
    const existing = opened.nodeId && opened.workspaceId ? documents.find(item => item.session.opened.nodeId === opened.nodeId && item.session.opened.workspaceId === opened.workspaceId
        && readOnlySource(item) === (opened.readOnly === true || opened.kind === "trash" || opened.kind === "managedTrash")) : undefined;
    if (!existing)
        addDocument(opened);
    else {
        await existing.writing.settleForSessionChange();
        if (existing.session.baseSource === opened.source) {
            existing.session.opened = opened;
            existing.editor.clearTokens();
            existing.sourceTrashed = false;
            existing.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
            existing.session.projection = null;
            existing.lastSave = undefined;
            await persistDraft(existing);
            scheduleRender(existing, 0);
        }
        else {
            existing.externalSource = opened.source;
            existing.comparisonTarget = opened;
            existing.session.error = uiMessage("shell.openedTargetDiskChanged");
        }
        current = existing;
        activeSourceOrigin = null;
        renderContent();
        renderSidebar();
    }
    shell.classList.remove("sidebar-open");
}
function toast(message: UiText, error = false): void {
    const item = el("div", `toast ${error ? "error" : ""}`, message);
    toastHost.append(item);
    setTimeout(() => item.remove(), error ? 7000 : 3500);
}
function showError(error: unknown): void { toast(error instanceof Error ? errorUiText(error) : String(error), true); }
function setBusy(target: HTMLButtonElement, busy: boolean): void { target.disabled = busy; target.classList.toggle("busy", busy); }
async function modal<T>(title: UiText, description: UiText, build: (form: HTMLFormElement, close: (value: T | null) => void) => void): Promise<T | null> {
    const trigger = document.activeElement as HTMLElement | null;
    const dialog = el("dialog", "modal");
    setUiAttr(dialog, "aria-label", title);
    const closeButton = button("", () => close(null), "icon-button", "close");
    setUiAttr(closeButton, "aria-label", displayMessage("shell.closeDocument", { p0: uiDisplayParameter(title) }));
    const heading = el("div", "modal-heading");
    heading.append(el("h2", "", title), closeButton);
    dialog.append(heading, el("p", "modal-description", description));
    const form = el("form", "modal-form");
    dialog.append(form);
    document.body.append(dialog);
    let resolve!: (value: T | null) => void;
    const result = new Promise<T | null>(r => { resolve = r; });
    function close(value: T | null): void {
        dialog.close();
        dialog.remove();
        if (trigger?.isConnected)
            trigger.focus({ preventScroll: true });
        resolve(value);
    }
    dialog.addEventListener("cancel", event => { event.preventDefault(); close(null); });
    build(form, close);
    dialog.showModal();
    setTimeout(() => form.querySelector<HTMLElement>("input,textarea,button")?.focus(), 20);
    return result;
}
async function pathDialog(kind: "openLibrary" | "createLibrary" | "file" | "saveAs"): Promise<string | null> {
    const config = {
        openLibrary: [uiMessage("shell.openLibrary"), uiMessage("shell.openLibraryDescription"), uiMessage("shell.folderPath"), uiMessage("common.open")],
        createLibrary: [uiMessage("shell.createLibrary"), uiMessage("shell.createLibraryDescription"), uiMessage("shell.newFolderPath"), uiMessage("shell.createLibrary")],
        file: [uiMessage("shell.openTypstFile"), uiMessage("shell.openTypstFileDescription"), uiMessage("shell.filePath"), uiMessage("shell.openFile")],
        saveAs: [uiMessage("shell.saveAs"), uiMessage("shell.saveAsDescription"), uiMessage("shell.filePath"), uiMessage("shell.saveAs")],
    }[kind];
    return modal<string>(config[0], config[1], (form, close) => {
        const label = el("label", "field-label", config[2]);
        const input = el("input", "text-input");
        input.required = true;
        setUiAttr(input, "placeholder", kind.includes("Library") ? uiMessage("shell.libraryPathPlaceholder") : uiMessage("shell.filePathPlaceholder"));
        if (kind === "saveAs" && current)
            input.value = current.session.path;
        label.append(input);
        form.append(label);
        if (kind === "openLibrary" && inventory) {
            const recent = button(displayMessage("shell.currentPath", { p0: uiDisplayParameter(inventory.root) }), () => { input.value = inventory!.root; input.focus(); }, "recent-path");
            form.append(recent);
        }
        const actions = el("div", "modal-actions");
        actions.append(button(uiMessage("common.cancel"), () => close(null), "button-secondary"));
        const submit = el("button", "button-primary", config[3]);
        submit.type = "submit";
        actions.append(submit);
        form.append(actions);
        form.onsubmit = event => {
            event.preventDefault();
            if (input.value.trim())
                close(input.value.trim());
        };
    });
}
function renderSidebar(): void {
    if (sidebarComposing)
        return;
    nodeContents?.dispose();
    nodeContents = undefined;
    navigationLayout.captureSidebarState();
    navigationLayout.setContext(inventory ? { root: inventory.root, workspaceId: inventory.workspaceId, scope: selectedFolder ? { kind: "directory", key: `path:${normalize(selectedFolder)}` } : undefined } : null);
    workspaceTools.setLibrary(inventory, selectedFolder);
    const focused = document.activeElement as HTMLInputElement | null;
    const restoreSearch = focused?.id === "library-search";
    const selection = restoreSearch ? [focused!.selectionStart, focused!.selectionEnd] : null;
    sidebar.replaceChildren();
    const brand = el("div", "brand");
    const mark = el("div", "brand-mark", "w");
    brand.append(mark, el("span", "brand-name", "Weftext"), el("span", "local-badge", uiMessage("shell.localDevice")));
    sidebar.append(brand);
    const libraryButton = button(inventory ? basename(inventory.root) : uiMessage("shell.openLibrary"), chooseLibrary, "library-button", "book");
    libraryButton.append(icon("down"));
    if (inventory)
        setUiAttr(libraryButton, "title", inventory.root);
    sidebar.append(libraryButton);
    if (inventory && !inventory.workspaceId)
        sidebar.append(el("span", "library-kind", uiMessage("shell.ordinaryFolderEditable")));
    const quick = el("div", "sidebar-quick");
    quick.append(button(uiMessage("shell.newNote"), createNote, "new-note", "plus"), button("", openOutsideFile, "icon-button", "file"));
    setUiAttr(quick.lastElementChild!, "aria-label", uiMessage("shell.openOutsideTypst"));
    (quick.firstChild as HTMLButtonElement).disabled = !inventory?.workspaceId;
    if (inventory && !inventory.workspaceId)
        setUiAttr((quick.firstChild as HTMLButtonElement), "title", uiMessage("shell.ordinaryFolderCreateLibraryHint"));
    sidebar.append(quick);
    const search = el("label", "search-box");
    search.append(icon("search"));
    const input = el("input");
    input.id = "library-search";
    setUiAttr(input, "placeholder", uiMessage("shell.searchFilesPlaceholder"));
    input.value = query;
    input.disabled = !inventory;
    setUiAttr(input, "aria-label", uiMessage("shell.searchFiles"));
    input.oninput = event => {
        query = input.value;
        if (!(event as InputEvent).isComposing && !sidebarComposing)
            renderSidebar();
    };
    input.addEventListener("compositionstart", () => { sidebarComposing = true; });
    input.addEventListener("compositionend", () => { sidebarComposing = false; query = input.value; renderSidebar(); });
    search.append(input);
    sidebar.append(search);
    if (inventory) {
        const scope = el("div", "search-scope");
        scope.append(button(uiMessage("shell.bodyPropertyQuery"), () => workspaceTools.search(query, true), "text-button"), el("span", "", selectedFolder ? basename(selectedFolder) : uiMessage("queryPanel.entireLibrary")));
        sidebar.append(scope);
        if (selectedFolder)
            sidebar.append(button(uiMessage("shell.returnEntireLibrary"), () => browseFolder(null), "clear-scope"));
        if (selectedFolder)
            renderNodeContents(sidebar, selectedFolder);
    }
    const list = el("div", "file-tree");
    if (!inventory) {
        const empty = el("div", "sidebar-empty");
        empty.append(icon("folder"), el("p", "", uiMessage("shell.emptyLibraryTitle")), el("span", "", uiMessage("shell.emptyLibraryDescription")));
        list.append(empty);
    }
    else {
        const heading = el("div", "section-label", query ? uiMessage("shell.searchResults") : uiMessage("shell.filesAndNotes"));
        if (!inventory.complete)
            heading.append(el("span", "partial-label", uiMessage("shell.scanIncomplete")));
        list.append(heading);
        const paired = new Map(inventory.entries.filter(entry => entry.kind === "node").map(entry => [normalize(entry.parentPath), entry]));
        const navigation = inventory.entries.filter(entry => entry.kind !== "node" || !inventory!.entries.some(dir => dir.kind === "directory" && normalize(dir.path) === normalize(entry.parentPath))).map(entry => {
            const primary = entry.kind === "directory" ? paired.get(normalize(entry.path)) : null;
            return primary ? { ...primary, relativePath: entry.relativePath, name: entry.name, folderPath: entry.path } : { ...entry, folderPath: entry.kind === "directory" ? entry.path : null };
        });
        const matching = navigation.filter(entry => {
            if (selectedFolder && normalize(entry.path) !== normalize(selectedFolder) && !normalize(entry.path).startsWith(normalize(selectedFolder) + "/"))
                return false;
            if (!query.trim())
                return true;
            const needle = query.toLocaleLowerCase();
            return `${entry.name} ${entry.relativePath} ${entry.title ?? ""}`.toLocaleLowerCase().includes(needle);
        });
        const visible = matching.filter(entry => query || ![...collapsed].some(path => normalize(entry.path).startsWith(normalize(path) + "/")));
        for (const entry of visible) {
            const directory = entry.kind === "directory";
            const row = button("", () => directory ? browseFolder(entry.folderPath!) : openInventoryEntry(entry, inventory!.root, inventory!.workspaceId), `tree-row ${current && normalize(current.session.path) === normalize(entry.path) ? "active" : ""}`, entry.folderPath ? "folder" : "file");
            const depth = Math.max(0, entry.relativePath.replaceAll("\\", "/").split("/").length - 1);
            row.style.paddingLeft = `${12 + (query ? 0 : Math.min(depth, 5) * 14)}px`;
            const names = el("span", "tree-names");
            names.append(el("span", "tree-name", entry.title || entry.name));
            if (query)
                names.append(el("span", "tree-path", entry.relativePath));
            row.append(names);
            if (entry.folderPath) {
                const expand = el("span", "tree-chevron", collapsed.has(entry.folderPath) ? "›" : "⌄");
                setUiAttr(expand, "title", uiMessage("shell.toggleChildFiles"));
                expand.onclick = event => { event.stopPropagation(); toggleFolder(entry.folderPath!); };
                row.append(expand);
            }
            if (!["ordinary", "admitted"].includes(entry.identityStatus))
                row.append(el("span", "identity-badge", labelStatus[entry.identityStatus] ?? entry.identityStatus));
            setUiAttr(row, "title", () => `${uiTextValue(entry.path)}\n${uiTextValue(labelStatus[entry.identityStatus] ?? entry.identityStatus)}`);
            if (entry.kind === "blocked" || !directory && !entry.path.toLowerCase().endsWith(".typ"))
                row.disabled = true;
            const menu = () => nodeMenu([entry], entry.folderPath ?? undefined);
            row.oncontextmenu = event => { event.preventDefault(); row.focus(); menu(); };
            row.onkeydown = event => {
                if (!event.isComposing && (event.key === "ContextMenu" || event.shiftKey && event.key === "F10")) {
                    event.preventDefault();
                    menu();
                }
            };
            if (entry.folderPath) {
                const wrapper = el("div", "tree-row-with-scope");
                const scopeButton = button("", () => browseFolder(entry.folderPath), "tree-scope-button", "search");
                setUiAttr(scopeButton, "aria-label", displayMessage("shell.browseFolderOnly", { p0: uiDisplayParameter(entry.name) }));
                setUiAttr(scopeButton, "title", displayMessage("shell.setFolderBrowseScope", { p0: uiDisplayParameter(entry.name) }));
                const expandButton = button(collapsed.has(entry.folderPath) ? "›" : "⌄", () => toggleFolder(entry.folderPath!), "tree-scope-button");
                setUiAttr(expandButton, "aria-label", displayMessage("shell.toggleFolder", { p0: uiDisplayParameter(entry.name) }));
                expandButton.setAttribute("aria-expanded", String(!collapsed.has(entry.folderPath)));
                const more = button("⋯", menu, "tree-scope-button");
                more.onkeydown = row.onkeydown;
                more.oncontextmenu = row.oncontextmenu;
                setUiAttr(more, "aria-label", displayMessage("shell.moreActionsForEntry", { p0: uiDisplayParameter(entry.name) }));
                wrapper.append(row, expandButton, scopeButton, more);
                list.append(wrapper);
            }
            else {
                const wrapper = el("div", "tree-row-with-scope");
                const more = button("⋯", menu, "tree-scope-button");
                more.onkeydown = row.onkeydown;
                more.oncontextmenu = row.oncontextmenu;
                setUiAttr(more, "aria-label", displayMessage("shell.moreActionsForEntry", { p0: uiDisplayParameter(entry.name) }));
                wrapper.append(row, more);
                list.append(wrapper);
            }
        }
        if (!visible.length)
            list.append(el("div", "search-empty", query ? displayMessage("shell.searchNoMatch", { p0: uiDisplayParameter(query) }) : uiMessage("shell.folderHasNoNotes")));
    }
    sidebar.append(list);
    if (inventory?.warnings.length) {
        const warnings = el("details", "library-warnings");
        warnings.append(el("summary", "", displayMessage("shell.libraryNoticeCount", { p0: inventory.warnings.length })));
        for (const message of inventory.warnings)
            warnings.append(el("p", "", coreDescriptionUiText(message)));
        sidebar.append(warnings);
    }
    const bottom = el("div", "sidebar-bottom");
    bottom.append(button(uiMessage("shell.createLibrary"), createLibrary, "text-button", "plus"), button(uiMessage("shell.recoveryRecords"), showRecovery, "text-button", "undo"));
    sidebar.append(bottom);
    if (restoreSearch) {
        const newInput = sidebar.querySelector<HTMLInputElement>("#library-search")!;
        newInput.focus();
        if (selection)
            newInput.setSelectionRange(selection[0], selection[1]);
    }
    navigationLayout.restoreSidebarState();
}
function toggleFolder(path: string): void { collapsed.has(path) ? collapsed.delete(path) : collapsed.add(path); renderSidebar(); }
function browseFolder(path: string | null): void {
    const old = selectedFolder ?? inventory?.root;
    if (old)
        folderViews.set(normalize(old), { query, scroll: sidebar.querySelector(".file-tree")?.scrollTop ?? 0 });
    selectedFolder = path;
    contentSelections.clear();
    const saved = folderViews.get(normalize(path ?? inventory?.root ?? ""));
    query = saved?.query ?? "";
    renderSidebar();
    const tree = sidebar.querySelector(".file-tree");
    if (tree)
        tree.scrollTop = saved?.scroll ?? 0;
}
function nodeMenu(entries: InventoryEntry[], folder?: string): void {
    const selected = entries.filter(entry => entry.path.toLowerCase().endsWith(".typ")), root = inventory?.root, workspaceId = inventory?.workspaceId;
    const commands: EverydayCommand[] = [
        { id: "node-open", label: uiMessage("shell.openSelectedNote"), unavailable: !selected.length || !root ? uiMessage("shell.selectedItemMainFileUnavailable") : undefined, run: async () => {
                for (const entry of selected)
                    await openInventoryEntry(entry, root!, workspaceId);
            } },
        { id: "node-preview", label: uiMessage("shell.previewSelectedItem"), run: () => previewNodes(entries) },
        { id: "node-copy-path", label: uiMessage("shell.copySelectedFullPath"), run: () => navigator.clipboard.writeText(entries.map(entry => entry.path).join("\n")) },
        { id: "node-operations", label: uiMessage("shell.wholeNodeFileActions"), detail: uiMessage("shell.openExactNoteBeforeFileImpact"), unavailable: selected.length !== 1 || !root ? uiMessage("shell.oneExactNodePerOperation") : undefined, run: async () => { await openInventoryEntry(selected[0], root!, workspaceId); await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("files"); } },
    ];
    if (folder)
        commands.unshift({ id: "node-browse", label: uiMessage("shell.browseNodeContents"), run: () => browseFolder(folder) });
    openEverydayMenu(entries.length === 1 ? displayMessage("shell.entryActions", { p0: uiDisplayParameter(entries[0].title || entries[0].name) }) : displayMessage("shell.selectedItemActions", { p0: entries.length }), commands, toast, false);
}
async function previewNodes(entries: InventoryEntry[]): Promise<void> {
    await modal(uiMessage("shell.itemPreview"), uiMessage("shell.itemPreviewDescription"), (form, close) => {
        for (const entry of entries)
            form.append(el("strong", "", entry.title || entry.name), el("pre", "", () => `${uiTextValue(entry.relativePath)}\n${uiTextValue(labelStatus[entry.identityStatus] ?? entry.identityStatus)}\n${uiTextValue(entry.excerpt ?? uiMessage("shell.itemSummaryUnavailable"))}`));
        form.append(button(uiMessage("common.close"), () => close(null)));
    });
}
function renderNodeContents(host: HTMLElement, folder: string): void {
    if (!inventory)
        return;
    const selected = inventory, panel = el("section", "node-contents");
    setUiAttr(panel, "aria-label", uiMessage("shell.nodeContents"));
    host.append(panel);
    nodeContents = mountNodeContents(panel, selected, folder, query, contentSelections, {
        request: api, readBlob: apiBlob, browse: browseFolder, menu: nodeMenu, preview: previewNodes, notify: toast,
        open: entry => entry.kind === "directory" ? browseFolder(entry.path) : openInventoryEntry(entry, selected.root, selected.workspaceId),
        changed: async () => {
            workspaceTools.invalidateQueryEnvironment(selected.root);
            if (inventory === selected) {
                inventory = await api<Inventory>("/api/library/open", { path: selected.root });
                renderSidebar();
            }
        },
    });
}
function renderTopbar(): void {
    // Writing blur can report state between pointerdown and click. Preserve the
    // actual action nodes while updating labels, so the pending click survives.
    const sessionId = current?.session.opened.sessionId ?? "";
    if (topbar.dataset.session === sessionId) {
        if (current) {
            const title = topbar.querySelector<HTMLElement>(".document-title")!;
            setUiText(title, basename(current.session.path));
            const path = topbar.querySelector<HTMLElement>(".document-path")!;
            setUiText(path, current.session.path);
            setUiAttr(path, "title", current.session.path);
            const state = topbar.querySelector<HTMLElement>(".save-state")!;
            state.className = `save-state ${current.session.dirty ? "dirty" : ""}`;
            setUiText(state, saveState(current));
            const save = topbar.querySelector<HTMLButtonElement>('[data-command="save"]')!;
            save.disabled = saveBusy(current);
            setUiText(save.lastChild!, saveBusy(current) ? uiMessage("shell.saveAwaitingConfirmationEllipsis") : uiMessage("common.save"));
        }
        return;
    }
    topbar.dataset.session = sessionId;
    topbar.replaceChildren();
    const menu = button("", () => { shell.classList.toggle("sidebar-open"); }, "icon-button mobile-menu", "menu");
    setUiAttr(menu, "aria-label", uiMessage("shell.showLibraryNavigation"));
    topbar.append(menu);
    const location = el("div", "document-location");
    if (current) {
        const name = el("div", "document-title", basename(current.session.path));
        const subtitle = el("span", "document-path", current.session.path);
        setUiAttr(subtitle, "title", current.session.path);
        location.append(name, subtitle);
    }
    else
        location.append(el("span", "home-label", uiMessage("shell.writingAndKnowledge")));
    topbar.append(location);
    const actions = el("div", "top-actions");
    if (current) {
        const state = el("span", `save-state ${current.session.dirty ? "dirty" : ""}`, saveState(current));
        actions.append(state);
        actions.append(button(uiMessage("shell.importRecognition"), () => recognitionPanel.open(), "button-secondary compact"));
        actions.append(button(uiMessage("shell.saveAs"), saveAs, "button-secondary compact"));
        const timeline = button(uiMessage("shell.timeline"), () => historyPanel.open(), "button-secondary compact");
        timeline.disabled = !current.session.opened.nodeId || !current.session.opened.workspaceId;
        actions.append(timeline);
        const saveButton = button(saveBusy(current) ? uiMessage("shell.saveAwaitingConfirmationEllipsis") : uiMessage("common.save"), () => saveDocument(current!), "button-primary compact", "save");
        saveButton.dataset.command = "save";
        saveButton.disabled = saveBusy(current);
        actions.append(saveButton);
    }
    else
        actions.append(el("span", "top-note", uiMessage("shell.localNativeTypst")));
    topbar.append(actions);
}
function saveState(doc: LiveDocument): UiText { return doc.writing.state.pending ? uiMessage("shell.textAwaitingConfirmation") : doc.session.opened.recoveredDraftConflict ? uiMessage("shell.recoveredDraftDiskConflict") : historyPanel.hasPending(historyContext(doc)) ? uiMessage("shell.originalHistoryRestorePending") : doc.session.savePending ? uiMessage("shell.saving") : pendingSave(doc) ? uiMessage("shell.saveResultAwaitingConfirmation") : doc.draftFailure ? uiMessage("shell.draftRecoveryWriteFailed") : doc.session.recovered ? uiMessage("shell.draftRecovered") : doc.session.dirty ? uiMessage("shell.unsaved") : uiMessage("shell.saved"); }
function renderTabs(): void {
    const ids = JSON.stringify(documents.map(doc => doc.session.opened.sessionId));
    if (tabs.dataset.ids === ids) {
        tabs.querySelectorAll<HTMLButtonElement>(".document-tab").forEach((tab, i) => {
            const doc = documents[i];
            tab.className = `document-tab ${doc === current ? "selected" : ""}`;
            tab.setAttribute("aria-current", doc === current ? "page" : "false");
            setUiText(tab.querySelector<HTMLElement>(".tab-name")!, basename(doc.session.path));
            const dirty = tab.querySelector(".tab-dirty");
            if (doc.session.dirty && !dirty)
                tab.append(el("span", "tab-dirty", "•"));
            else if (!doc.session.dirty)
                dirty?.remove();
        });
        return;
    }
    tabs.dataset.ids = ids;
    tabs.replaceChildren();
    tabs.hidden = !documents.length;
    for (const doc of documents) {
        const tab = button("", () => activateDocument(doc), `document-tab ${doc === current ? "selected" : ""}`, "file");
        tab.setAttribute("aria-current", doc === current ? "page" : "false");
        tab.append(el("span", "tab-name", basename(doc.session.path)));
        if (doc.session.dirty)
            tab.append(el("span", "tab-dirty", "•"));
        tabs.append(tab);
    }
    if (documents.length) {
        const add = button("", openOutsideFile, "icon-button tab-add", "plus");
        setUiAttr(add, "aria-label", uiMessage("shell.openAnotherFile"));
        tabs.append(add);
    }
}
function renderStatus(): void {
    statusbar.replaceChildren();
    if (!current) {
        statusbar.append(el("span", "", uiMessage("shell.filesOnYourDevice")), el("span", "", uiMessage("shell.localPreviewEdition")));
        return;
    }
    const doc = current;
    const session = doc.session;
    const left = el("div", "status-left");
    const chars = doc.editor.source === session.source ? doc.editor.characterCount : countSourceCodePoints(session.source);
    left.append(el("span", "", displayMessage("shell.characterCount", { p0: uiDisplayParameter(chars.toLocaleString()) })), el("span", "status-kind", session.opened.nodeId ? uiMessage("shell.libraryNote") : uiMessage("shell.ordinaryTypstFile")));
    const right = el("div", "status-right");
    if (doc.writing.state.pending || doc.writing.state.busy)
        right.append(el("span", "pending-label", doc.writing.state.busy ? uiMessage("shell.textUpdating") : uiMessage("shell.textSyncingToDraft")));
    else if (!sourceConsumersAvailable(doc))
        right.append(el("span", "pending-label", doc.sourceRefreshing ? uiMessage("shell.checkingSyncResult") : uiMessage("shell.previewAnnotationsAwaitComparison")));
    else if (doc.rendering)
        right.append(el("span", "pending-label", uiMessage("shell.previewUpdating")));
    else if (session.error && !session.projectionCurrent)
        right.append(el("span", "error-label", session.error));
    else if (!session.projectionCurrent)
        right.append(el("span", "pending-label", uiMessage("shell.previewUpdating")));
    else if (session.projection?.ok)
        right.append(icon("check"), el("span", "", uiMessage("shell.draftPreviewUpdated")));
    else
        right.append(icon("alert"), el("span", "", uiMessage("shell.previewErrorSourceRetained")));
    if (doc.draftFailure)
        right.append(el("span", "error-label", displayMessage("shell.recoveryDraftNotWritten", { p0: uiDisplayParameter(doc.draftFailure) })));
    else if (session.dirty && session.lastDraftSource === session.source)
        right.append(el("span", "", uiMessage("shell.recoveryDraftRetained")));
    statusbar.append(left, right);
}
function renderWelcome(): void {
    const welcome = el("section", "welcome");
    const eyebrow = el("div", "eyebrow", uiMessage("shell.yourWritingSpaceEyebrow"));
    welcome.append(eyebrow, el("h1", "", uiMessage("shell.welcomeHeadline")), el("p", "welcome-intro", uiMessage("shell.welcomeDescription")));
    const cards = el("div", "welcome-cards");
    for (const [label, description, action, iconName] of [
        [uiMessage("shell.openLibrary"), uiMessage("shell.openLibraryWelcomeHint"), chooseLibrary, "book"],
        [uiMessage("shell.createLibrary"), uiMessage("shell.createLibraryWelcomeHint"), createLibrary, "plus"],
        [uiMessage("shell.openDotTypFile"), uiMessage("shell.openDotTypFileWelcomeHint"), openOutsideFile, "file"],
    ] as const) {
        const card = button("", action, "welcome-card");
        const cardIcon = el("div", "welcome-card-icon");
        cardIcon.append(icon(iconName));
        const text = el("div", "welcome-card-text");
        text.append(el("strong", "", label), el("span", "", description));
        card.append(cardIcon, text, icon("arrow"));
        cards.append(card);
    }
    welcome.append(cards);
    if (startupError) {
        const error = el("div", "startup-error");
        error.append(icon("alert"), el("p", "", startupError), button(uiMessage("shell.retryConnection"), bootstrap, "button-secondary"));
        welcome.append(error);
    }
    if (inventory) {
        const section = el("div", "welcome-library");
        section.append(el("h2", "", basename(inventory.root)), el("p", "", !inventory.workspaceId ? uiMessage("shell.ordinaryFolderOpened") : inventory.complete ? uiMessage("shell.libraryOpened") : uiMessage("shell.libraryPartialScan")), inventory.workspaceId ? button(uiMessage("shell.newNote"), createNote, "button-primary", "plus") : button(uiMessage("shell.createNewLibrary"), createLibrary, "button-secondary", "plus"));
        welcome.append(section);
    }
    const footnote = el("div", "welcome-footnote");
    footnote.append(el("span", "footnote-dot"), uiTextNode(uiMessage("shell.localCapabilities")));
    welcome.append(footnote);
    content.append(welcome);
}
function documentComparisonMessage(doc: LiveDocument): UiText | null {
    if (sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget))
        return sourceMoveComparisonMessage;
    return doc.comparisonTarget && doc.comparisonTarget.source !== doc.session.baseSource
        ? uiMessage("shell.diskChangedCompareBeforeSaving") : null;
}
const noteInputRecovery = new NoteInputRecovery({ getItem: key => sessionStorage.getItem(key), setItem: (key, value) => sessionStorage.setItem(key, value) });
function localPersonalAudience(): string {
    let user = localStorage.getItem("weftext-local-personal-id");
    if (!user) {
        user = crypto.randomUUID();
        localStorage.setItem("weftext-local-personal-id", user);
    }
    return `local:${user}`;
}
function localPersonalNumbering(doc: LiveDocument) {
    return personalNumberingKeys(localPersonalAudience(), doc.session.opened.workspaceId ?? normalize(doc.session.opened.root), doc.session.opened.nodeId ?? normalize(doc.session.path));
}
function localAuthoringEnvironment(doc: LiveDocument): AuthoringPanelsEnvironment {
    const readable = () => current === doc && sourceConsumersAvailable(doc) && !doc.sourceTrashed;
    return {
        controller: doc.writing, readable, editable: () => readable() && !readOnlySource(doc) && doc.mode !== "read" && !saveBusy(doc) && !pendingSave(doc),
        projection: () => readable() && doc.session.projectionCurrent ? doc.session.projection : null,
        selection: () => doc.editor.selection, requestEdit: request => api("/api/edit", request), previewProposal: request => api("/api/editor/preview", request),
        revealSource: range => revealWritingSource(doc, range), revealRendered: key => {
            if (!readable() || !renderHost || !renderHost.isConnected || renderHost.getClientRects().length === 0)
                return false;
            const target = Array.from(renderHost.querySelectorAll<HTMLElement>("[data-document-node-key]")).find(item => item.dataset.documentNodeKey === key);
            if (!target)
                return false;
            target.scrollIntoView({ block: "center", inline: "nearest" });
            target.focus({ preventScroll: true });
            return true;
        },
        ensureEndnoteModule: async () => { await doc.authoringAssets.install(); toast(uiMessage("shell.endnoteResourcesVerified")); },
        recoverEndnoteModule: async () => { const receipt = await doc.authoringAssets.recover(); toast(receipt ? displayMessage("shell.originalEndnoteModuleStatus", { p0: uiDisplayParameter(receipt.status) }) : uiMessage("shell.noPendingEndnoteModule")); },
        retryEndnoteModule: async () => { await doc.authoringAssets.retryOriginal(); toast(uiMessage("shell.endnoteModuleConfirmedBodyUnsaved")); },
        restartEndnoteModule: async () => { await doc.authoringAssets.restartAfterCancelled(); toast(uiMessage("shell.cancelledModuleReprepared")); },
        settleEndnoteModule: async () => { const receipt = await doc.authoringAssets.settleOriginal(); toast(displayMessage("shell.originalModuleStatus", { p0: uiDisplayParameter(receipt.status) })); },
        pendingInput: value => { noteInputRecovery.add(localPersonalAudience(), value); toast(uiMessage("shell.unappliedNoteInputRetained")); }, onError: showError,
    };
}
async function openLocalNumbering(doc: LiveDocument): Promise<void> {
    await doc.writing.deferUntilCompositionEnds(() => doc.writing.flush());
    if (current !== doc)
        throw new UiError(uiMessage("shell.noteSwitchedNumberingNotOpened"));
    const keys = localPersonalNumbering(doc);
    await openNumberingPanel(numberingHost(localAuthoringEnvironment(doc), { ...keys, display: enabled => {
            if (current === doc && renderHost)
                displayPersonalHeadings(renderHost, enabled);
        } }));
}
function renderDocument(): void {
    if (!current)
        return;
    const doc = current;
    const comparisonMessage = documentComparisonMessage(doc);
    if (comparisonMessage)
        doc.session.error = comparisonMessage;
    const controls = el("div", "document-controls");
    const viewSwitch = el("div", "view-switch");
    viewSwitch.setAttribute("role", "group");
    setUiAttr(viewSwitch, "aria-label", uiMessage("shell.documentView"));
    for (const mode of modes) {
        const choice = button(mode.title, () => changeMode(mode.id), doc.mode === mode.id ? "view-choice active" : "view-choice");
        setUiAttr(choice, "title", () => `${uiTextValue(mode.short)} · ${uiTextValue(mode.hint)}`);
        choice.setAttribute("aria-pressed", String(doc.mode === mode.id));
        viewSwitch.append(choice);
    }
    controls.append(viewSwitch);
    const layout = el("select", "layout-choice");
    setUiAttr(layout, "aria-label", uiMessage("shell.documentLayout"));
    for (const [value, label] of [["continuous", uiMessage("shell.continuousLayout")], ["paged", uiMessage("shell.pagedPreview")]] as const) {
        const option = el("option", "", label);
        option.value = value;
        layout.append(option);
    }
    layout.value = doc.layout;
    layout.onchange = () => { void doc.writing.deferUntilCompositionEnds(async () => { await doc.writing.flush(); doc.layout = layout.value as LiveDocument["layout"]; renderContent(); }).catch(showError); };
    controls.append(layout);
    if (doc.mode === "source") {
        const preview = button(previewVisible ? uiMessage("shell.collapsePreview") : uiMessage("shell.showPreview"), () => { previewVisible = !previewVisible; localStorage.setItem("weftext-preview", String(previewVisible)); renderContent(); }, "text-button preview-toggle", "eye");
        controls.append(preview);
    }
    content.append(controls);
    const caption = el("div", "view-caption");
    caption.append(el("span", "", modes.find(m => m.id === doc.mode)!.hint));
    if (doc.mode === "read" || readOnlySource(doc))
        caption.append(el("span", "readonly-label", readOnlySource(doc) ? uiMessage("shell.recycledReadOnly") : uiMessage("shell.readOnlyCurrentDraft")));
    if (doc.session.recovered)
        caption.append(button(uiMessage("shell.recoveredPreviousDraftView"), showDraftComparison, "recovery-caption"));
    if (noteInputRecovery.count(localPersonalAudience()))
        caption.append(button(uiMessage("shell.retrieveUnappliedNoteInput"), showRecovery, "recovery-caption"));
    content.append(caption);
    doc.notesMount?.dispose();
    const authoring = el("section", "document-authoring-controls"), notes = el("div", "document-notes-summary");
    authoring.append(button(uiMessage("shell.numberingControls"), () => openLocalNumbering(doc), "text-button"), button(uiMessage("shell.outlineHeadingStructure"), async () => { await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("headings"); }, "text-button"), notes);
    content.append(authoring);
    doc.notesMount = mountDocumentNotes(notes, documentNotesHost(localAuthoringEnvironment(doc)));
    if (doc.session.opened.recoveredDraftConflict) {
        const conflict = el("div", "document-error document-conflict");
        conflict.append(icon("alert"), el("span", "", uiMessage("shell.recoveredDraftDiskDiffers")), button(uiMessage("shell.compareAndMerge"), showDraftComparison, "button-secondary compact"));
        content.append(conflict);
    }
    if (doc.session.error) {
        const error = el("div", "document-error session-error");
        error.append(icon("alert"), el("span", "", doc.session.error), doc.saveAsChangedTarget ? button(uiMessage("shell.openSaveAsTarget"), () => openDocument(doc.saveAsChangedTarget!), "text-button") : button(uiMessage("shell.compareDiskContent"), showDraftComparison, "text-button"));
        content.append(error);
    }
    if (doc.lastSave?.path && doc.lastSave.receipt) {
        const record = doc.lastSave;
        const delivery = el("div", "document-error save-as-delivery");
        delivery.append(el("span", "", joinUiText([displayMessage("shell.originalSaveAsCommit", {operationId:uiDisplayParameter(record.operationId)}), record.deliveryEnded ? uiMessage("shell.saveAsDeliveryEndedState") : uiMessage("shell.saveAsDeliveryPendingState")])), button(uiMessage("shell.inspectOriginalSaveAs"), () => inspectSaveAsDelivery(record.operationId), "text-button"));
        if (!record.deliveryEnded && !doc.session.savePending) delivery.append(button(uiMessage("shell.endSaveAsDelivery"), () => endSaveAsDelivery(doc, record), "text-button"));
        content.append(delivery);
    }
    const body = el("div", `document-body mode-${doc.mode} ${previewVisible ? "with-preview" : ""}`);
    content.append(body);
    if (doc.layout === "paged") {
        if (["source", "styled"].includes(doc.mode)) {
            const sourcePane = el("section", "source-pane");
            sourcePane.append(sourceToolbar(doc));
            const host = el("div", "source-host");
            sourcePane.append(host);
            doc.editor.mount(host);
            body.append(sourcePane);
        }
        const host = el("section", "paged-host");
        body.append(host);
        pagedPreview = new PagedPreview(host, { basis: () => current === doc && sourceConsumersAvailable(doc) ? { sessionId: doc.session.opened.sessionId, source: doc.session.source, serial: doc.session.serial } : null, request: basis => api("/api/paged", basis), fileName: () => basename(doc.session.path).replace(/\.typ$/i, ".pdf"), onError: showError });
        void pagedPreview.render();
        if (doc.queries.hasQueries) {
            const flow = el("section", "flow-pane paged-query-flow");
            flow.append(el("p", "embedded-query-help", uiMessage("shell.pagedPreviewQueryHint")));
            renderHost = el("div", "flow-document preview-document");
            setUiAttr(renderHost, "aria-label", uiMessage("shell.continuousDraftWithQueries"));
            flow.append(renderHost);
            body.append(flow);
            flowScroll = flow;
            renderProjection(doc);
        }
    }
    else if (["source", "styled"].includes(doc.mode)) {
        const sourcePane = el("section", "source-pane");
        setUiAttr(sourcePane, "aria-label", uiMessage("shell.sourceEditingArea"));
        sourcePane.append(sourceToolbar(doc));
        const host = el("div", "source-host");
        sourcePane.append(host);
        doc.editor.mount(host);
        body.append(sourcePane);
        if (previewVisible || doc.mode === "styled") {
            const pane = el("section", "preview-pane");
            const header = el("div", "pane-heading");
            header.append(el("span", "", uiMessage("shell.draftPreview")), el("span", "", uiMessage("shell.continuousReading")));
            pane.append(header);
            renderHost = el("div", "flow-document preview-document");
            setUiAttr(renderHost, "aria-label", uiMessage("shell.streamingDraftPreview"));
            pane.append(renderHost);
            body.append(pane);
            flowScroll = pane;
            renderProjection(doc);
        }
    }
    else {
        const flow = el("section", "flow-pane");
        renderHost = el("div", "flow-document");
        setUiAttr(renderHost, "aria-label", doc.mode === "read" ? uiMessage("shell.readOnlyDraft") : uiMessage("shell.visualDraft"));
        flow.append(renderHost);
        body.append(flow);
        flowScroll = flow;
        renderProjection(doc);
        if (doc.mode === "write" && !readOnlySource(doc)) {
            body.before(createWritingToolbar(doc.writing, { onError: showError, insertLink, revealSource: range => revealWritingSource(doc, range) }));
        }
        if (doc.mode === "focus" && activeSourceOrigin) {
            const source = el("section", "focus-source");
            const heading = el("div", "pane-heading");
            heading.append(el("span", "", uiMessage("shell.currentConstructSyntax")), button(uiMessage("shell.collapseSyntax"), () => { doc.editor.clearReveal(); activeSourceOrigin = null; renderContent(); }, "text-button", "close"));
            source.append(heading, sourceToolbar(doc));
            const host = el("div", "source-host");
            source.append(host);
            body.append(source);
            doc.editor.mount(host);
        }
    }
    if (doc.session.projectionCurrent && doc.session.projection!.diagnostics.length)
        renderDiagnostics(doc);
    if (doc.session.projectionCurrent && sourceConsumersAvailable(doc))
        doc.editor.tokens(doc.session.projection!.sourceInfo.syntaxTokens, doc.mode === "styled", doc.editor.syntaxBasis());
    else if (!sourceConsumersAvailable(doc) || doc.sourceTrashed)
        doc.editor.clearTokens();
    doc.editor.styleMode(doc.mode === "styled");
    doc.editor.readOnly(doc.mode === "read" || doc.mode === "write" || readOnlySource(doc));
    const scroller = flowScroll, mode = doc.mode;
    requestAnimationFrame(() => {
        if (current !== doc || doc.mode !== mode)
            return;
        if (scroller && flowScroll === scroller && doc.scroll[mode] != null)
            scroller.scrollTop = doc.scroll[mode]!;
        if (["source", "styled"].includes(mode) && doc.sourceScroll != null)
            doc.editor.view.scrollDOM.scrollTop = doc.sourceScroll;
    });
}
function sourceToolbar(doc: LiveDocument): HTMLElement {
    const toolbar = el("div", "source-toolbar");
    toolbar.setAttribute("role", "toolbar");
    setUiAttr(toolbar, "aria-label", uiMessage("shell.sourceFormattingTools"));
    const actions: [
        string,
        UiText,
        string,
        string,
        string
    ][] = [["bold", uiMessage("shell.formatBold"), "*", "*", "文字"], ["italic", uiMessage("shell.formatItalic"), "_", "_", "文字"], ["strike", uiMessage("shell.formatStrike"), "#strike[", "]", "文字"], ["heading", uiMessage("shell.formatHeading"), "= ", "", "章节标题"], ["list", uiMessage("queryView.list"), "- ", "", "列表内容"], ["equation", uiMessage("shell.formatEquation"), "$ ", " $", "x + y"]];
    for (const [role, label, before, after, placeholder] of actions) {
        const control = button(label, () => {
            if (!readOnlySource(doc) && !doc.editor.view.composing)
                doc.editor.insert(before, after, placeholder);
        }, `format-button format-${role}`);
        control.disabled = readOnlySource(doc);
        toolbar.append(control);
    }
    const link = button(uiMessage("shell.formatLink"), insertLink, "format-button", "link");
    link.disabled = readOnlySource(doc);
    toolbar.append(link);
    const spacer = el("span", "toolbar-spacer");
    toolbar.append(spacer, el("span", "keyboard-hint", uiMessage("shell.sourceKeyboardHint")));
    return toolbar;
}
function renderContent(): void {
    const recognitionCurrent = current?.session.opened.root ?? "";
    if (recognitionSession !== recognitionCurrent) {
        recognitionPanel.retireDisplay(true);
        recognitionSession = recognitionCurrent;
    }
    if (flowScroll && flowHasProjection && renderedDocument && renderedMode)
        renderedDocument.scroll[renderedMode] = flowScroll.scrollTop;
    if (renderedDocument && renderedMode && ["source", "styled"].includes(renderedMode))
        renderedDocument.sourceScroll = renderedDocument.editor.view.scrollDOM.scrollTop;
    renderedDocument = current;
    renderedMode = current?.mode ?? null;
    flowHasProjection = false;
    pagedPreview?.dispose();
    pagedPreview = null;
    workspaceTools.attachSurface(null);
    projectionMappings = [];
    for (const doc of documents) {
        doc.queries.detach();
        doc.deferredProjection.clear();
    }
    renderHost = null;
    flowScroll = null;
    content.replaceChildren();
    current ? renderDocument() : renderWelcome();
    renderTopbar();
    renderTabs();
    renderStatus();
    renderEverydayBar();
}
function refreshChrome(): void {
    const comparisonMessage = current && documentComparisonMessage(current);
    if (current && comparisonMessage) {
        current.session.error = comparisonMessage;
        let notice = content.querySelector<HTMLElement>(".document-error.session-error");
        if (!notice) {
            notice = el("div", "document-error session-error");
            notice.append(icon("alert"), el("span", "", comparisonMessage), button(uiMessage("shell.compareDiskContent"), showDraftComparison, "text-button"));
            content.querySelector(".document-body")?.before(notice);
        }
        else {
            const message = notice.querySelector("span");
            if (message)
                setUiText(message, comparisonMessage);
        }
    }
    renderTopbar();
    renderTabs();
    renderStatus();
    if (current)
        refreshProjectionNotice(current);
    if (current && !current.session.recovered)
        content.querySelector(".recovery-caption")?.remove();
    if (current && !current.session.error)
        content.querySelector(".document-error.session-error")?.remove();
    if (current && !current.session.opened.recoveredDraftConflict)
        content.querySelector(".document-conflict")?.remove();
}
function projectionStaleMessage(doc: LiveDocument): UiText {
    return doc.session.error
        ? displayMessage("shell.lastSuccessfulPreviewError", { p0: uiDisplayParameter(doc.session.error) }) : uiMessage("shell.lastPreviewUpdating");
}
function fillProjectionEmpty(host: HTMLElement, doc: LiveDocument): void {
    host.replaceChildren();
    if (doc.session.error)
        host.append(icon("alert"), el("h3", "", uiMessage("shell.draftPreviewUnavailable")), el("p", "projection-message", doc.session.error), button(uiMessage("shell.viewSourceAction"), () => changeMode("source"), "button-secondary", "code"));
    else
        host.append(el("div", "loading-ring"), el("p", "", uiMessage("shell.compilingDraft")));
}
// Update only the notice: retain the source editor, active input and scroll.
function refreshProjectionNotice(doc: LiveDocument): void {
    if (doc !== current || !renderHost || !sourceConsumersAvailable(doc) || doc.session.projectionCurrent)
        return;
    if (doc.session.projection) {
        let notice = renderHost.querySelector<HTMLElement>(":scope > .preview-stale");
        if (!notice) {
            notice = el("div", "preview-stale");
            renderHost.prepend(notice);
        }
        const message = projectionStaleMessage(doc);
        if (notice.textContent !== message)
            setUiText(notice, message);
    }
    else {
        const notice = renderHost.querySelector<HTMLElement>(":scope > .projection-empty");
        if (!notice)
            return;
        if (doc.session.error ? notice.querySelector(".projection-message")?.textContent !== doc.session.error : !notice.querySelector(".loading-ring"))
            fillProjectionEmpty(notice, doc);
    }
}
function projectionPresentation(doc: LiveDocument): readonly unknown[] { return [doc, doc.session.opened.sessionId, doc.session.source, doc.session.serial, doc.session.projection, doc.writing.map, doc.mode, doc.layout, renderHost, sourceConsumersAvailable(doc), readOnlySource(doc)]; }
function renderProjection(doc: LiveDocument): void {
    if (doc !== current || !renderHost)
        return;
    if (doc.writing.retainInput || doc.editor.composing) {
        doc.deferredProjection.defer(projectionPresentation(doc));
        renderStatus();
        return;
    }
    doc.deferredProjection.clear();
    const restoreQuerySourceFocus = doc.mode === "focus" && !!doc.editor.revealed && doc.editor.view.hasFocus && !!doc.editor.view.dom.closest(".embedded-query-source-host");
    const preservedScroll = flowHasProjection ? flowScroll?.scrollTop : doc.scroll[doc.mode];
    const host = renderHost;
    host.replaceChildren();
    flowHasProjection = false;
    if (!sourceConsumersAvailable(doc)) {
        const waiting = el("div", "projection-empty");
        waiting.append(el("p", "", doc.sourceRefreshing ? uiMessage("shell.syncCheckingCurrentDraftRetained") : uiMessage("shell.noteMovedCompareDisk")));
        host.append(waiting);
        return;
    }
    const projection = doc.session.projection;
    if (!projection) {
        const waiting = el("div", "projection-empty");
        fillProjectionEmpty(waiting, doc);
        host.append(waiting);
        return;
    }
    const projectionCurrent = doc.session.projectionCurrent;
    if (!projectionCurrent) {
        const stale = el("div", "preview-stale", projectionStaleMessage(doc));
        host.append(stale);
    }
    if (!projection.ok || !projection.tree) {
        const failed = el("div", "projection-empty");
        failed.append(icon("alert"), el("h3", "", uiMessage("shell.draftPreviewUnavailable")), el("p", "", uiMessage("shell.previewUnavailableInputRetained")), button(uiMessage("shell.viewSourceAction"), () => changeMode("source"), "button-secondary", "code"));
        host.append(failed);
        return;
    }
    let writableCount = 0, personalHeadingOrdinal = 0;
    const renderedTree = projection.tree, renderedSource = projection.source;
    projectionMappings = [];
    const writingMap = doc.mode === "write" && !readOnlySource(doc) && projectionCurrent ? doc.writing.map : null;
    const structures = writingStructureIndex(writingMap);
    const groups = writingMap ? groupWritingRenderNodes(projection.tree, writingMap, doc.session.source) : [];
    const grouped = new Map(groups.flatMap(group => group.memberKeys.map(key => [key, group] as const)));
    const emptySlots = new Map((writingMap ? groupWritingEmptySlots(projection.tree, writingMap, doc.session.source) : []).map(slot => [slot.nodeKey, slot.region]));
    doc.queries.beginProjection(projection.tree, { sessionId: doc.session.opened.sessionId, source: projection.source, serial: projection.serial });
    function draw(node: RenderNode): Node {
        const query = doc.queries.forNode(node);
        if (query)
            return query;
        if (node.kind === "tag")
            return document.createDocumentFragment();
        if (node.kind === "text") {
            const text = node.text ?? "";
            const group = grouped.get(node.key);
            const region = group?.region;
            if (doc.mode === "write" && !readOnlySource(doc) && group && region) {
                if (group.firstKey !== node.key)
                    return document.createDocumentFragment();
                const input = el("textarea", "literal-input");
                input.rows = 1;
                setUiAttr(input, "aria-label", uiMessage("shell.editParagraphText"));
                input.dataset.nodeKey = node.key;
                input.spellcheck = false;
                doc.writing.bindLiteral(input, region);
                writableCount++;
                return input;
            }
            if (doc.mode === "focus" && node.origin?.isMain) {
                const span = el("span", "focus-text", text);
                span.tabIndex = 0;
                span.setAttribute("role", "button");
                setUiAttr(span, "aria-label", displayMessage("shell.expandSourceExcerpt", { p0: uiDisplayParameter(text.slice(0, 30)) }));
                setUiAttr(span, "title", uiMessage("shell.expandSourceHint"));
                const reveal = () => revealSource(node.origin!);
                span.onclick = reveal;
                span.onkeydown = event => {
                    if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        reveal();
                    }
                };
                projectionMappings.push({ nodeKey: node.key, node: span, text, origin: node.origin });
                return span;
            }
            const leaf = uiTextNode(text);
            projectionMappings.push({ nodeKey: node.key, node: leaf, text, origin: node.origin });
            return leaf;
        }
        if (node.kind === "frame") {
            const island = el("div", "layout-island");
            island.append(el("span", "", uiMessage("shell.layoutNeedsPagination")));
            if (doc.mode !== "read")
                island.append(button(uiMessage("shell.viewSourceAction"), () => node.origin ? revealSource(node.origin) : changeMode("source"), "text-button", "code"));
            return island;
        }
        const tag = node.tag?.toLowerCase() ?? "span";
        const element = nativeElement(node);
        element.setAttribute("data-document-node-key", node.key);
        const semanticRole = node.attrs["data-weftext-role"], noteId = node.attrs["data-weftext-note-id"], targetId = node.attrs["data-weftext-target-id"];
        if (semanticRole === "heading")
            markPersonalHeading(element, ++personalHeadingOrdinal);
        if (projectionCurrent && typeof noteId === "string" && ["footnote-reference", "endnote-reference"].includes(String(semanticRole))) {
            element.setAttribute("role", "button");
            element.setAttribute("tabindex", "0");
            setUiAttr(element, "aria-label", displayMessage("shell.viewNoteBodyMarkers", { p0: uiDisplayParameter(semanticRole === "endnote-reference" ? uiMessage("shell.endnote") : uiMessage("shell.footnote")) }));
            const open = () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), semanticRole === "endnote-reference" ? "endnote" : "footnote", noteId);
            element.addEventListener("click", event => { event.preventDefault(); event.stopPropagation(); void open().catch(showError); }, true);
            element.addEventListener("keydown", event => {
                if (event instanceof KeyboardEvent && !event.isComposing && !doc.writing.state.composing && ["Enter", " "].includes(event.key)) {
                    event.preventDefault();
                    event.stopPropagation();
                    void open().catch(showError);
                }
            });
        }
        const target = node.attrs.href;
        if (tag === "a" && typeof target === "string") {
            element.setAttribute("href", "#");
            setUiAttr(element, "title", target);
            element.addEventListener("click", event => { event.preventDefault(); void navigateLink(target).catch(showError); });
        }
        for (const child of node.children ?? [])
            element.append(draw(child));
        if (node.text)
            element.append(uiTextNode(node.text));
        if (projectionCurrent && typeof targetId === "string" && ["figure", "equation"].includes(String(semanticRole))) {
            const actions = el("span", "document-notes-object-actions");
            actions.append(button(semanticRole === "equation" ? uiMessage("shell.equationNumberCaption") : uiMessage("shell.objectCaptionNumbering"), () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), "caption", targetId), "text-button"));
            element.append(actions);
        }
        if (projectionCurrent && typeof noteId === "string" && ["footnote-body", "endnote-body"].includes(String(semanticRole)))
            element.append(button(uiMessage("shell.viewBodyMarkersNotes"), () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), semanticRole === "endnote-body" ? "endnote" : "footnote", noteId), "text-button"));
        const empty = emptySlots.get(node.key);
        if (doc.mode === "write" && !readOnlySource(doc) && empty) {
            const input = el("textarea", "literal-input empty-slot-input");
            input.rows = 1;
            setUiAttr(input, "aria-label", uiMessage("shell.editEmptyContent"));
            doc.writing.bindLiteral(input, empty);
            element.append(input);
            writableCount++;
        }
        if (writingMap && node.origin?.isMain) {
            const structure = structures.get(`${node.origin.utf16From}:${node.origin.utf16To}`);
            if (structure) {
                const controls = createStructureControls(doc.writing, structure, { onError: showError, revealSource: slot => revealWritingSource(doc, slot), renderedTable: element });
                if (controls) {
                    const wrapper = el("div", "writing-structure");
                    if (controls.classList.contains("writing-table-surface"))
                        wrapper.append(controls);
                    else
                        wrapper.append(element, controls);
                    return wrapper;
                }
            }
        }
        if (doc.mode === "write" && node.origin?.isMain && !node.writable && ["pre", "table", "figure"].includes(tag)) {
            const wrapper = el("div", "readonly-construct");
            wrapper.append(element, button(uiMessage("shell.readOnlyPartViewSource"), () => revealSource(node.origin!), "source-entry", "code"));
            return wrapper;
        }
        if (tag === "math" && projectionCurrent) {
            const wrapper = el("div", "math-discussion-target");
            const isCurrent = (): boolean => doc === current && doc.session.projection === projection && doc.session.projectionCurrent && sourceConsumersAvailable(doc);
            wrapper.append(element, createMathSelectionControls({ math: node, tree: renderedTree, source: renderedSource,
                current: isCurrent, notify: message => toast(message, true), select: selection => {
                    if (!isCurrent())
                        return;
                    const structure = doc.writing.map?.structures.find(item => item.range.utf16From === selection.from && item.range.utf16To === selection.to || item.body?.utf16From === selection.from && item.body?.utf16To === selection.to);
                    readingSelection = { sessionId: doc.session.opened.sessionId, serial: doc.session.serial, selection: { from: selection.from, to: selection.to, visibleExcerpt: selection.visibleExcerpt, nodeKey: selection.nodeKey, elementRole: structure?.kind } };
                    doc.editor.select(selection.from, selection.to, false);
                    toast(uiMessage("shell.exactEquationSelected"));
                } }));
            return wrapper;
        }
        return element;
    }
    host.append(draw(projection.tree));
    displayPersonalHeadings(host, personalNumberingVisible(localPersonalNumbering(doc)));
    if (restoreQuerySourceFocus && doc.editor.view.dom.isConnected)
        doc.editor.view.focus();
    host.onmouseup = captureReadingSelection;
    host.onkeyup = captureReadingSelection;
    void workspaceTools.projectionChanged(host, projectionMappings).catch(showError);
    if (doc.mode === "write" && !readOnlySource(doc)) {
        if (doc.session.projectionCurrent && doc.writing.map?.bodyInsertion?.writable) {
            const input = el("textarea", "literal-input append-input");
            input.rows = 1;
            setUiAttr(input, "placeholder", writableCount ? uiMessage("shell.continueWritingPlaceholder") : uiMessage("shell.startWritingPlaceholder"));
            setUiAttr(input, "aria-label", uiMessage("shell.newParagraphInput"));
            doc.writing.bindBodyInput(input);
            host.append(input);
        }
        if (!writableCount && !(projection.sourceInfo.bodyInsertion?.writable)) {
            const fallback = el("div", "rich-readonly");
            fallback.append(icon("code"), el("p", "", uiMessage("shell.noSafeEditableRegion")), button(uiMessage("embeddedQuery.editSource"), () => changeMode("source"), "button-secondary"));
            host.append(fallback);
        }
        else
            host.append(el("p", "rich-scope-note", uiMessage("shell.visualEditingBoundaryHint")));
    }
    flowHasProjection = true;
    const scroller = flowScroll, mode = doc.mode;
    if (scroller && preservedScroll != null) {
        scroller.scrollTop = preservedScroll;
        const installed = scroller.scrollTop;
        requestAnimationFrame(() => {
            if (current !== doc || doc.mode !== mode || flowScroll !== scroller || renderHost !== host || !flowHasProjection || scroller.scrollTop !== installed)
                return;
            scroller.scrollTop = preservedScroll;
            doc.scroll[mode] = scroller.scrollTop;
        });
    }
}
function captureReadingSelection(): void {
    const doc = current;
    const selection = window.getSelection();
    if (!doc || !doc.session.projectionCurrent || !selection || selection.isCollapsed || selection.rangeCount !== 1)
        return;
    const range = selection.getRangeAt(0);
    // This UI path only supplies a direct character map. More complex generated
    // or escaped appearances remain source-selectable instead of guessing offsets.
    const endpoint = (node: Node, offset: number): number | null => {
        const matches = projectionMappings.filter(item => item.node === node || item.node.firstChild === node);
        if (matches.length !== 1)
            return null;
        const item = matches[0], origin = item.origin;
        if (!origin?.isMain || origin.raw !== item.text || doc.session.source.slice(origin.utf16From, origin.utf16To) !== item.text || offset < 0 || offset > item.text.length)
            return null;
        if (projectionMappings.filter(other => other.origin?.isMain && other.origin.utf16From === origin.utf16From && other.origin.utf16To === origin.utf16To).length !== 1)
            return null;
        return origin.utf16From + offset;
    };
    const from = endpoint(range.startContainer, range.startOffset), to = endpoint(range.endContainer, range.endOffset);
    if (from == null || to == null || from >= to)
        return;
    readingSelection = { sessionId: doc.session.opened.sessionId, serial: doc.session.serial, selection: { from, to, visibleExcerpt: selection.toString(), elementRole: "text" } };
}
function renderDiagnostics(doc: LiveDocument): void {
    content.querySelector(".diagnostics")?.remove();
    const details = el("details", "diagnostics");
    const diagnostics = doc.session.projection!.diagnostics;
    const errors = diagnostics.filter(d => d.severity === "error").length;
    details.open = errors > 0;
    details.append(el("summary", "", displayMessage("shell.diagnosticSummary", { p0: uiDisplayParameter(errors ? displayMessage("shell.errorCount", { p0: errors }) : displayMessage("shell.noticeCount", { p0: diagnostics.length })), p1: uiDisplayParameter(doc.session.projection!.compilerVersion) })));
    for (const diagnostic of diagnostics) {
        const item = button(engineDiagnosticUiText(diagnostic), () => diagnostic.origin ? revealSource(diagnostic.origin) : changeMode("source"), `diagnostic-item ${diagnostic.severity}`, diagnostic.severity === "error" ? "alert" : "code");
        details.append(item);
    }
    content.append(details);
}
async function changeMode(mode: ViewMode): Promise<void> {
    if (!current)
        return;
    if (current.writing.state.composing) {
        const doc = current;
        await doc.writing.deferUntilCompositionEnds(() => {
            if (current === doc)
                return changeMode(mode);
        });
        return;
    }
    if (mode === "source" || mode === "styled")
        await current.writing.settleForSessionChange();
    else
        await flushLiteral();
    if (flowScroll && flowHasProjection)
        current.scroll[current.mode] = flowScroll.scrollTop;
    current.editor.clearReveal();
    current.mode = mode;
    current.queries.modeChanged();
    activeSourceOrigin = null;
    localStorage.setItem("weftext-view", mode);
    renderContent();
}
function revealSource(origin: Origin): void {
    if (!current || !origin.isMain || !current.session.projectionCurrent)
        return;
    current.queries.closeExpansion();
    if (current.mode !== "focus")
        current.mode = "source";
    else
        activeSourceOrigin = origin;
    if (current.mode === "focus") {
        const map = current.writing.map;
        const construct = map?.structures.filter(item => item.range.utf16From <= origin.utf16From && item.range.utf16To >= origin.utf16To).sort((a, b) => (a.range.to - a.range.from) - (b.range.to - b.range.from))[0]?.range;
        const slot = construct ?? { ...origin, writable: true };
        if (!current.editor.reveal({ from: slot.utf16From, to: slot.utf16To, raw: slot.raw }, current.session.source))
            return;
    }
    else
        current.editor.clearReveal();
    renderContent();
    current.editor.select(Math.min(origin.utf16From, current.session.source.length), Math.min(origin.utf16To, current.session.source.length));
}
function revealWritingSource(doc: LiveDocument, range: WritingRange): void {
    if (doc !== current)
        return;
    revealSource({ ...range, path: doc.session.path, syntaxKind: "structure", isMain: true });
}
function scheduleDraft(doc: LiveDocument): void { clearTimeout(doc.draftTimer); doc.draftTimer = setTimeout(() => { void persistDraft(doc).catch(showError); }, 450); }
async function persistDraft(doc: LiveDocument, proposedSource?: string): Promise<void> {
    if (bodyCloseFrozen)
        throw new UiError(uiMessage("shell.closeRecoveryWriteFrozen"));
    clearTimeout(doc.draftTimer);
    const source = proposedSource ?? doc.writing.recoverySource() ?? doc.session.source;
    const serial = doc.session.serial;
    const request = doc.draftChain.catch(() => undefined).then(async () => {
        await api("/api/draft", { sessionId: doc.session.opened.sessionId, source });
        doc.session.acknowledgeDraft(source, serial);
        doc.draftFailure = null;
    });
    doc.draftChain = request;
    try {
        await request;
    }
    catch (error) {
        const failure = error instanceof ApiError && error.code === "tooLarge"
            ? new ApiError(error.code, uiMessage("shell.sourceSizeLimitExceeded"), error.diagnostics, error.details, error.causes)
            : error;
        doc.draftFailure = failure instanceof Error ? errorUiText(failure) : String(failure);
        throw failure;
    }
    finally {
        if (doc === current)
            renderStatus();
    }
}
function scheduleRender(doc: LiveDocument, delay = 300): void {
    doc.renderGeneration++;
    clearTimeout(doc.renderTimer);
    if (!sourceConsumersAvailable(doc)) {
        if (doc === current)
            refreshChrome();
        return;
    }
    doc.renderTimer = setTimeout(() => { void compile(doc); }, delay);
}
async function compile(doc: LiveDocument): Promise<void> {
    clearTimeout(doc.renderTimer);
    if (!sourceConsumersAvailable(doc))
        return;
    const source = doc.session.source;
    const serial = doc.session.serial;
    const generation = doc.renderGeneration;
    const sessionId = doc.session.opened.sessionId;
    const syntaxBasis = doc.editor.syntaxBasis();
    doc.rendering++;
    if (doc === current)
        renderStatus();
    if (doc === current && doc.layout === "paged")
        void pagedPreview?.render({ sessionId: doc.session.opened.sessionId, source, serial });
    try {
        const basis = { sessionId: doc.session.opened.sessionId, source, serial };
        const hadQueries = doc.queries.hasQueries;
        const [result, editorMap] = await Promise.all([
            api<RenderOutput>("/api/render", basis),
            api<WritingMap>("/api/editor-map", basis),
            doc.queries.discover(basis),
        ]);
        if (!sourceConsumersAvailable(doc) || generation !== doc.renderGeneration || sessionId !== doc.session.opened.sessionId)
            return;
        if (result.sourceInfo.notes?.sourceHash === editorMap.sourceHash)
            editorMap.notes = result.sourceInfo.notes;
        if (doc.session.acceptProjection(result)) {
            // Map observers consume the accepted projection as well as its source.
            // Publish it before notifying them so initial notes controls can enable.
            doc.writing.installMap(editorMap, source, serial);
            doc.editor.tokens(result.sourceInfo.syntaxTokens, doc.mode === "styled", syntaxBasis);
            if (doc === current) {
                if (doc.layout === "paged" && hadQueries !== doc.queries.hasQueries)
                    renderContent();
                else
                    renderProjection(doc);
                if (result.diagnostics.length)
                    renderDiagnostics(doc);
                else
                    content.querySelector(".diagnostics")?.remove();
                scrollFragment(doc);
            }
        }
    }
    catch (error) {
        if (sourceConsumersAvailable(doc) && doc.session.serial === serial && generation === doc.renderGeneration && sessionId === doc.session.opened.sessionId) {
            doc.session.error = displayMessage("shell.previewFailed", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) });
            if (doc === current)
                toast(doc.session.error, true);
        }
    }
    finally {
        doc.rendering--;
        if (doc === current)
            refreshChrome();
    }
}
async function flushLiteral(): Promise<void> {
    if (current)
        await current.writing.flush();
}
async function chooseLibrary(): Promise<void> {
    const path = await pathDialog("openLibrary");
    if (!path)
        return;
    await flushLiteral();
    inventory = await api<Inventory>("/api/library/open", { path });
    selectedFolder = null;
    collapsed.clear();
    query = "";
    renderSidebar();
    renderContent();
}
async function creation<T extends OriginalFact>(route: string, payload: Record<string, unknown>, deliver: (receipt: T) => Promise<void>): Promise<T> {
  if (pendingCreation && pendingCreation.route !== route) throw new UiError(uiMessage("shell.previousCreationUnconfirmedRetry"));
  const retry = pendingCreation !== null;
  const request = pendingCreation ?? { route, payload: { ...payload, operationId: operationId() }, receipt: undefined as OriginalFact | undefined };
  pendingCreation = request;
  try { const result = await submitOriginal<T>(api, request.route, request.payload, "newNode", retry); request.receipt = result; await deliver(result); pendingCreation = null; return result; }
  catch (error) { if (error instanceof TerminatedAttempt && !request.receipt) pendingCreation = null; else toast(uiMessage("shell.creationUnconfirmedRetrySameOperation"), true); throw error; }
}
async function createLibrary(): Promise<void> {
  if (pendingCreation) throw new UiError(uiMessage("shell.originalNoteCreationPending"));
  await openLibraryCreation({ defaults: () => api("/api/library/create-defaults"), create: request => api("/api/library/create", request), delivery: operationId => api("/api/operations/status", { operationId }), deliver: async receipt => {
    if (!receipt.library || receipt.library.workspaceId !== receipt.workspaceId || normalize(receipt.library.root) !== normalize(receipt.root) || receipt.rootNotePath && !receipt.rootDocument) throw new UiError(uiMessage("shell.libraryExactDeliveryPending"));
    workspaceTools.invalidateQueryEnvironment(receipt.root);
    inventory = receipt.library; query = ""; selectedFolder = null; renderSidebar(); renderContent();
    if (receipt.rootDocument) await acceptCreatedDocument(receipt.rootDocument);
    else await refreshInventory();
    toast(uiMessage("shell.libraryCreated"));
  }, inspect: operationId => api("/api/recovery/inspect", { operationId }), resolve: (operationId, action) => api("/api/recovery/resolve", { operationId, action }), cancel: request => api("/api/library/create-cancel", request) });
}
async function openOutsideFile(): Promise<void> {
    const path = await pathDialog("file");
    if (path)
        await openDocument(path);
}
async function createNote(): Promise<void> {
    if (!inventory) {
        await chooseLibrary();
        return;
    }
    if (!inventory.workspaceId) {
        toast(uiMessage("shell.ordinaryFolderCannotCreateNote"), true);
        return;
    }
    const name = pendingCreation?.route === "/api/note/create" ? String(pendingCreation.payload.name) : await modal<string>(uiMessage("shell.newNote"), displayMessage("shell.saveToFolder", { p0: uiDisplayParameter(selectedFolder ?? inventory.root) }), (form, close) => {
        const label = el("label", "field-label", uiMessage("shell.noteName"));
        const input = el("input", "text-input");
        setUiAttr(input, "placeholder", uiMessage("shell.untitledNotePlaceholder"));
        input.required = true;
        label.append(input);
        form.append(label);
        form.append(el("p", "field-help", uiMessage("shell.noteNameDescription")));
        const actions = el("div", "modal-actions");
        actions.append(button(uiMessage("common.cancel"), () => close(null), "button-secondary"));
        const submit = el("button", "button-primary", uiMessage("shell.createAndOpen"));
        submit.type = "submit";
        actions.append(submit);
        form.append(actions);
        form.onsubmit = event => {
            event.preventDefault();
            if (input.value.trim())
                close(input.value.trim());
        };
    });
    if (!name)
        return;
    await prepareSwitch();
    await creation<OriginalFact & { document?: OpenedDocument; documentError?: ApiErrorBody }>("/api/note/create", { root: inventory.root, parent: selectedFolder ?? inventory.root, name }, async receipt => {
        if (receipt.status !== "committed" || !receipt.document || receipt.documentError) throw new UiError(joinUiText([uiMessage("shell.noteOriginalDeliveryPending"), receipt.documentError ? apiErrorUiText(receipt.documentError) : uiMessage("shell.checkOriginalOperationSentence")]));
        await acceptCreatedDocument(receipt.document);
    });
}
async function prepareSwitch(): Promise<void> {
    if (bodyCloseFrozen)
        throw new UiError(uiMessage("shell.closeDocumentSwitchFrozen"));
    captureNavigation();
    if (current)
        await current.writing.settleForSessionChange();
    if (current)
        await persistDraft(current);
}
async function openDocument(path: string, root?: string): Promise<void> {
    const existing = documents.find(doc => normalize(doc.session.path) === normalize(path));
    if (existing && !existing.sourceTrashed) {
        await activateDocument(existing);
        return;
    }
    const generation = ++navigationGeneration;
    await prepareSwitch();
    if (generation !== navigationGeneration)
        return;
    const opened = await api<OpenedDocument>("/api/open", { path, root });
    if (generation !== navigationGeneration)
        return;
    if (opened.kind === "managed" && documents.some(doc => doc.sourceTrashed && doc.session.opened.nodeId === opened.nodeId && doc.session.opened.workspaceId === opened.workspaceId))
        await acceptResolvedDocument(opened);
    else
        addDocument(opened);
    shell.classList.remove("sidebar-open");
}
function readOnlySource(doc: LiveDocument): boolean { return doc.session.opened.readOnly === true || doc.session.opened.kind === "trash" || doc.session.opened.kind === "managedTrash"; }
function addDocument(opened: OpenedDocument): void {
    if (bodyCloseFrozen)
        throw new UiError(uiMessage("shell.closeLateOpenFrozen"));
    const session = new DocumentSession(opened);
    let doc!: LiveDocument;
    const editor = new SourceEditor(session.source, source => {
        session.edit(source);
        doc.queries.invalidate();
        readingSelection = null;
        workspaceTools.documentChanged();
        if (current === doc)
            pagedPreview?.invalidate();
        scheduleDraft(doc);
        scheduleRender(doc);
        if (current === doc)
            refreshChrome();
    }, () => { void saveDocument(doc).catch(showError); });
    editor.observe(event => {
        if (event.kind === "reveal" && !editor.revealed && doc?.mode === "focus") {
            activeSourceOrigin = null;
            doc.mode = "source";
            doc.queries.modeChanged();
            queueMicrotask(() => {
                if (current === doc)
                    renderContent();
            });
        }
        if (event.kind === "composition" && !event.composing && doc?.mode === "focus" && current === doc)
            queueMicrotask(() => {
                if (current === doc)
                    renderProjection(doc);
            });
    });
    const preference = localStorage.getItem("weftext-view");
    const mode = modes.some(m => m.id === preference) ? preference as ViewMode : "styled";
    const writing = new WritingController({
        actionAvailable: () => current === doc,
        refreshMap: basis => api("/api/editor-map", basis),
        editor, basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }),
        // A temporary consumer pause cannot revoke pending literal settlement.
        // Only the reviewed placement conflict blocks new native visual actions.
        editable: () => !bodyCloseFrozen && !sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget) && doc.mode !== "read" && !readOnlySource(doc), request: request => api("/api/edit", request),
        onState: () => {
            if (!doc)
                return;
            doc.deferredProjection.flush(() => current === doc && renderHost ? projectionPresentation(doc) : undefined, () => doc.writing.retainInput || doc.editor.composing, () => renderProjection(doc));
            if (current === doc)
                refreshChrome();
        }, onError: showError,
        onPendingSource: source => { clearTimeout(doc.draftTimer); doc.draftTimer = setTimeout(() => { void persistDraft(doc, source).catch(showError); }, 220); },
        onApplied: () => { scheduleDraft(doc); scheduleRender(doc, 100); },
    });
    const queries = new EmbeddedQueryController({
        queryResultScope: "local",
        preferenceIdentity: () => session.opened.nodeId && session.opened.workspaceId ? `weftext-node://${session.opened.workspaceId}/${session.opened.nodeId}` : null,
        catalog: async () => (await api<{
            schema: import("./data-panel").DataSchema;
        }>("/api/data/schema", { root: session.opened.root })).schema,
        openDefinition: async (owner) => { await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: session.opened.root, entityKey: owner, lifecycle: "live" })); },
        basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }), mode: () => doc.mode,
        editable: () => sourceConsumersAvailable(doc) && current === doc && doc.mode !== "read" && !readOnlySource(doc) && !doc.sourceTrashed && !writing.state.composing,
        request: (path, basis) => {
            if (!sourceConsumersAvailable(doc))
                return Promise.reject(new UiError(uiMessage("shell.queryNoteLocationNeedsComparison")));
            return api(path, basis);
        }, prepareEdit: () => writing.flush(), notify: toast, output: query => workspaceTools.openQueryOutput(query),
        apply: async (patches, basis, event) => {
            if (current !== doc || doc.mode === "read" || readOnlySource(doc) || writing.state.composing || session.opened.sessionId !== basis.sessionId || session.serial !== basis.serial || session.source !== basis.source)
                throw new UiError(uiMessage("shell.queryProposalBasisChanged"));
            editor.applyPatches(basis.source, patches, undefined, event);
            scheduleDraft(doc);
            scheduleRender(doc, 0);
        },
        reveal: (occurrence, host) => {
            if (current !== doc || doc.mode !== "focus" || readOnlySource(doc))
                return false;
            activeSourceOrigin = null;
            const range = occurrence.range;
            const revealed = editor.revealed;
            if (!(revealed?.from === range.utf16From && revealed.to === range.utf16To && revealed.raw === range.raw) && !editor.reveal({ from: range.utf16From, to: range.utf16To, raw: range.raw }, session.source))
                return false;
            editor.readOnly(false);
            editor.mount(host);
            return true;
        },
        closeReveal: () => editor.clearReveal(),
        openOwner: async (origin) => {
            if (origin.isMain)
                return;
            await openDocument(origin.path, session.opened.root);
        },
        openRow: async (row, plan) => {
            const lifecycle = row.lifecycle ?? (plan?.advanced?.call || plan?.advanced?.related || plan?.advanced?.unionAll ? null : plan?.scope.lifecycle);
            if (!lifecycle || !row.path)
                throw new UiError(uiMessage("shell.queryRowExactNavigationMissing"));
            if (row.entityKey)
                await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: session.opened.root, entityKey: row.entityKey, lifecycle, ...(lifecycle === "trash" ? { revision: row.revision } : {}) }));
            else if (lifecycle === "live")
                await openDocument(row.path, session.opened.root);
            else
                throw new UiError(uiMessage("shell.recycleResultIdentityMissing"));
        },
    });
    const authoringAssets = new AuthoringAssetsAdapter({ request: api, prefix: "/api/editor/endnotes-module", context: () => doc && current === doc && sourceConsumersAvailable(doc) && !readOnlySource(doc) && !doc.sourceTrashed && doc.mode !== "read" ? { sessionId: doc.session.opened.sessionId, audience: localPersonalAudience(), root: doc.session.opened.root } : null, committed: context => {
            if (context.root)
                workspaceTools.invalidateQueryEnvironment(context.root);
        }, changed: async () => { doc.session.projection = null; doc.renderGeneration++; doc.queries.invalidate(); await compile(doc); } });
    doc = { navigationKey: opened.sessionId, session, editor, writing, queries, mode, authoringAssets, draftChain: Promise.resolve(), draftFailure: null, rendering: 0, renderGeneration: 0, scroll: {}, layout: "continuous", deferredProjection: new DeferredPaint() };
    documents.push(doc);
    current = doc;
    activeSourceOrigin = null;
    renderSidebar();
    renderContent();
    scheduleRender(doc, 0);
    if (session.recovered)
        toast(uiMessage("shell.lastUnsavedDraftRecovered"));
}
async function activateDocument(doc: LiveDocument): Promise<void> {
    const generation = ++navigationGeneration;
    const activeLibrary = inventory;
    if (doc.sourceTrashed && activeLibrary && activeLibrary.workspaceId === doc.session.opened.workspaceId && activeLibrary.entries.some(entry => entry.identityStatus === "admitted" && entry.nodeId === doc.session.opened.nodeId)) {
        await prepareSwitch();
        await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: activeLibrary.root, entityKey: `weftext-node://${doc.session.opened.workspaceId}/${doc.session.opened.nodeId}` }));
        return;
    }
    if (current === doc)
        return;
    await prepareSwitch();
    if (generation !== navigationGeneration)
        return;
    const prior = doc.session.opened;
    let fresh: OpenedDocument | null = null;
    try {
        fresh = prior.nodeId && prior.workspaceId && !readOnlySource(doc) ? await api<OpenedDocument>("/api/data/open", { root: prior.root, entityKey: `weftext-node://${prior.workspaceId}/${prior.nodeId}` }) : await api<OpenedDocument>("/api/open", { sessionId: prior.sessionId });
    } catch (error) {
        doc.session.error = joinUiText([uiMessage("shell.originalFileSessionUnavailable"), error instanceof Error ? errorUiText(error) : String(error)]);
    }
    if (generation !== navigationGeneration)
        return;
    const saved = navigationContexts.get(doc);
    if (saved?.inventory) {
        const before = saved.inventory, latest = await api<Inventory>("/api/library/open", { path: before.root });
        if (generation !== navigationGeneration)
            return;
        const relocate = (path: string): string | null => {
            const entry = before.entries.find(item => normalize(item.path) === normalize(path));
            const primary = entry?.nodeId ? entry : before.entries.find(item => item.kind === "node" && normalize(item.parentPath) === normalize(path));
            if (!primary?.nodeId)
                return null;
            const next = latest.entries.find(item => item.kind === "node" && item.nodeId === primary.nodeId && item.identityStatus === "admitted");
            return next ? (entry?.kind === "directory" ? next.parentPath : next.path) : null;
        };
        saved.selection = saved.selection.flatMap(path => { const next = relocate(path); return next ? [next] : []; });
        if (saved.folder) {
            const next = relocate(saved.folder);
            if (next)
                saved.folder = next;
            else if (!latest.entries.some(item => item.kind === "directory" && normalize(item.path) === normalize(saved.folder!))) {
                saved.folder = null;
                toast(uiMessage("shell.oldDirectoryScopeUnavailable"));
            }
        }
        saved.inventory = latest;
    }
    if (fresh?.source === doc.session.baseSource) {
        doc.session.opened = fresh;
        doc.session.acknowledgeSave(fresh.source, fresh.baseId, fresh.path);
        doc.sourceTrashed = false;
        doc.editor.clearTokens();
        doc.queries.invalidate();
        doc.session.projection = null;
        await persistDraft(doc);
    }
    else if (fresh) {
        doc.externalSource = fresh.source;
        doc.comparisonTarget = fresh;
        doc.session.error = uiMessage("shell.returnTargetDiskChanged");
    }
    if (generation !== navigationGeneration)
        return;
    if (flowScroll && flowHasProjection && current)
        current.scroll[current.mode] = flowScroll.scrollTop;
    current = doc;
    restoreNavigation(doc);
    if (doc.mode === "write")
        doc.writing.restoreNavigationFocus();
    activeSourceOrigin = null;
    renderSidebar();
    renderContent();
    if (saved) {
        const tree = sidebar.querySelector(".file-tree");
        if (tree)
            tree.scrollTop = saved.scroll;
        const contents = sidebar.querySelector(".node-contents");
        if (contents)
            contents.scrollTop = saved.contentScroll;
    }
    if (["source", "styled"].includes(doc.mode))
        doc.editor.view.focus();
    if (!doc.session.projectionCurrent)
        scheduleRender(doc, 0);
}
async function saveDocument(doc: LiveDocument, path?: string): Promise<void> {
    if (bodyCloseFrozen)
        throw new UiError(uiMessage("shell.closeSaveFrozen"));
    if (historyPanel.hasPending(historyContext(doc))) { toast(uiMessage("shell.historyRestoreBeforeSave"), true); return; }
    if (doc.session.savePending)
        return;
    if ((doc.sourceTrashed || readOnlySource(doc)) && !path) {
        toast(uiMessage("shell.noteRecycledRestoreOrSaveAs"), true);
        return;
    }
    if (doc.writing.state.composing) {
        await doc.writing.deferUntilCompositionEnds(() => saveDocument(doc, path));
        return;
    }
    doc.session.savePending = true;
    refreshChrome();
    try {
        if (!pendingSave(doc)) {
            await flushLiteral();
            await persistDraft(doc);
        }
    }
    catch (error) {
        doc.session.savePending = false;
        refreshChrome();
        throw error;
    }
    if (doc.session.opened.recoveredDraftConflict && !path) {
        doc.session.savePending = false;
        refreshChrome();
        await showDraftComparison(doc);
        return;
    }
    let reservedOperation: string | null = null;
    let querySaveNotice = false;
    if (!pendingSave(doc) && !path && doc.session.opened.nodeId && doc.session.source.includes('"weftext.query"')) {
        const source = doc.session.source, serial = doc.session.serial;
        try {
            const pending = await api<{
                operationId: string | null;
                queryState?: string;
            }>("/api/query/definition/pending", { sessionId: doc.session.opened.sessionId, source, serial });
            reservedOperation = pending.operationId;
            querySaveNotice = ["unparsed", "unadmitted", "multiplePending"].includes(pending.queryState ?? "");
        }
        catch (error) {
            doc.session.savePending = false;
            refreshChrome();
            throw error;
        }
        if (doc.session.source !== source || doc.session.serial !== serial) {
            doc.session.savePending = false;
            throw new UiError(uiMessage("shell.saveQueryReservationInputChanged"));
        }
    }
    const retry = pendingSave(doc) !== undefined;
    const request: NonNullable<LiveDocument["lastSave"]> = pendingSave(doc) ?? { source: doc.session.source, sourcePath: doc.session.path, baseId: doc.session.baseId, operationId: reservedOperation ?? operationId(), sessionId: doc.session.opened.sessionId, path };
    doc.lastSave = request;
    doc.session.savePending = true;
    refreshChrome();
    let receiptReceived = false;
    try {
        const payload = request.path
            ? { sessionId: request.sessionId, path: request.path, source: request.source, operationId: request.operationId }
            : { sessionId: request.sessionId, baseId: request.baseId, source: request.source, operationId: request.operationId };
        const receipt = fileSaveReceipt(await submitOriginal(api, request.path ? "/api/save-as" : "/api/save", payload, request.path ? "saveAs" : "save", retry), request);
        if (!originalCommitted(receipt)) throw new ApiError(receipt.status, uiMessage("shell.saveUnconfirmedDraftRetained"));
        receiptReceived = true;
        request.receipt = receipt;
        if (doc.lastSave !== request || doc.session.opened.sessionId !== (request.deliverySessionId ?? request.sessionId)) {
            doc.lastSave = request;
            throw new ApiError("committedDeliveryPending", uiMessage("shell.commitSessionChangedAwaitOriginalResult"));
        }
        if (receipt.recoveryIssue)
            toast(uiMessage("shell.savedRecoveryUpdateUnavailable"), true);
        workspaceTools.invalidateQueryEnvironment(doc.session.opened.root);
        if (request.path) {
            // The original physical creation must deliver its actual fresh Core Session.
            const opened = receipt.document;
            if (!opened || receipt.documentError || receipt.documentSourceMatchesCommit !== true || opened.path !== receipt.path || !canRebindSaveAs(request.source, opened)) {
                if (opened) { doc.comparisonTarget = opened; doc.externalSource = opened.source; }
                throw new UiError(uiMessage("shell.saveAsOriginalPhysicalDeliveryPending"));
            }
            await doc.writing.settleForSessionChange();
            if (doc.session.opened.sessionId !== (request.deliverySessionId ?? request.sessionId)) throw new UiError(uiMessage("shell.saveAsDeliverySessionChanged"));
            doc.session.opened = opened;
            request.deliverySessionId = opened.sessionId;
            doc.editor.clearTokens();
            doc.session.acknowledgeSave(request.source, opened.baseId, opened.path);
            doc.session.projection = null;
            scheduleRender(doc, 0);
        }
        else {
            if (receipt.document) await doc.writing.settleForSessionChange();
            if (!receipt.document || receipt.documentError || receipt.documentSourceMatchesCommit !== true || !rebindCommittedSave(doc.session, { ...request, sessionId: request.deliverySessionId ?? request.sessionId }, receipt.document)) {
                if (receipt.document) {
                    doc.externalSource = receipt.document.source;
                    doc.comparisonTarget = receipt.document;
                }
                throw new ApiError("committedDeliveryPending", uiMessage("shell.savedBasisChangedCheckDisk"));
            }
            request.deliverySessionId = receipt.document.sessionId;
            doc.editor.clearTokens();
            doc.session.projection = null;
            scheduleRender(doc, 0);
        }
        await persistDraft(doc);
        if (inventory) { inventory = await api<Inventory>("/api/library/open", { path: inventory.root }); renderSidebar(); }
        if (doc.lastSave !== request || doc.session.opened.sessionId !== (request.deliverySessionId ?? request.sessionId)) {
            doc.lastSave = request;
            throw new UiError(uiMessage("shell.laterDeliverySessionChanged"));
        }
        doc.saveAsChangedTarget = undefined;
        doc.lastSave = undefined;
        if (doc === current) { renderSidebar(); refreshChrome(); }
        toast(doc.session.dirty ? uiMessage("shell.savedLaterInputInDraft") : uiMessage("shell.savedSentence"), false);
        if (querySaveNotice) toast(uiMessage("shell.savedQueriesNotRunnable"), true);
    }
    catch (error) {
        if (!receiptReceived && error instanceof TerminatedAttempt)
            doc.lastSave = undefined;
        doc.session.error = error instanceof Error ? errorUiText(error) : String(error);
        if (error instanceof ApiError && error.currentSource != null)
            doc.externalSource = error.currentSource;
        if (error instanceof ApiError && error.code === "saveAsTargetChanged")
            doc.session.error = errorUiText(error);
        else if (receiptReceived)
            doc.session.error = doc.lastSave ? uiMessage("shell.committedDeliveryIncomplete") : displayMessage("shell.savedRecoveryInformationFailed", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) });
        else if (!(error instanceof ApiError))
            doc.session.error = uiMessage("shell.saveUnknownConfirmSameOperation");
        if (doc === current)
            renderContent();
        throw error;
    }
    finally {
        doc.session.savePending = false;
        if (doc === current) { if (request.path || retry) renderContent(); else refreshChrome(); }
    }
}
async function saveAs(): Promise<void> {
    if (!current)
        return;
    const path = await pathDialog("saveAs");
    if (path)
        await saveDocument(current, path);
}
async function inspectSaveAsDelivery(operationId: string): Promise<void> {
  const fact = await api<Receipt & OriginalFact>("/api/operations/status", { operationId });
  if (fact.operationId !== operationId || fact.status !== "committed") throw new UiError(uiMessage("shell.originalCommitReadUnavailable"));
  await modal<void>(uiMessage("shell.originalSaveAsFactTitle"), uiMessage("shell.originalSaveAsFactHelp"), (form, close) => {
    form.append(el("pre", "", JSON.stringify({ operationId: fact.operationId, status: fact.status, path: fact.path, baseId: fact.baseId, documentError: fact.documentError }, null, 2)), button(uiMessage("common.close"), () => close(), "button-primary"));
  });
}
async function endSaveAsDelivery(doc: LiveDocument, request: NonNullable<LiveDocument["lastSave"]>): Promise<void> {
  if (doc.session.savePending || doc.lastSave !== request || !request.path || !request.receipt || !["saved", "committed"].includes(request.receipt.status)) throw new UiError(uiMessage("shell.confirmedSaveAsRequiredToEnd"));
  await doc.writing.settleForSessionChange(); await persistDraft(doc);
  if (doc.lastSave !== request || doc.session.savePending) throw new UiError(uiMessage("shell.saveStateChangedOriginalRetained"));
  request.deliveryEnded = true; doc.session.error = null; doc.comparisonTarget = undefined; doc.externalSource = undefined;
  if (doc === current) renderContent();
  toast(uiMessage("shell.saveAsDeliveryEndedNotice"));
}

async function showDraftComparison(doc = current): Promise<void> {
    if (!doc)
        return;
    await doc.writing.settleForSessionChange();
    await persistDraft(doc);
    scheduleRender(doc, 0);
    const target = doc.comparisonTarget ?? doc.session.opened;
    const fresh = target.nodeId && target.workspaceId && target.kind === "managed"
        ? await api<OpenedDocument>("/api/data/open", { root: target.root, entityKey: `weftext-node://${target.workspaceId}/${target.nodeId}` })
        : await api<OpenedDocument>("/api/open", { path: target.path, root: target.root });
    await doc.writing.settleForSessionChange();
    const initialMergeSource = doc.session.source;
    const confirmed = { sessionId: doc.session.opened.sessionId, source: doc.session.source, serial: doc.session.serial, generation: doc.editor.generation };
    const result = await modal<{
        action: "discardRecovered" | "mergeRecovered";
        source: string;
    }>(uiMessage("shell.compareDraftAndDisk"), uiMessage("shell.compareDraftAndDiskDescription"), (form, close) => {
        const columns = el("div", "comparison");
        for (const [label, source] of [[uiMessage("shell.draftOriginalVersion"), doc.session.opened.recoveredDraftBaseSource ?? doc.session.baseSource], [uiMessage("shell.currentDiskContent"), fresh.source]] as const) {
            const pane = el("label", "field-label", label);
            const text = el("textarea", "text-input comparison-source");
            text.readOnly = true;
            text.value = source;
            text.rows = 10;
            pane.append(text);
            columns.append(pane);
        }
        form.append(columns);
        const variants = [...(doc.session.opened.recoveredDraftVariants ?? []), ...(fresh.recoveredDraftVariants ?? [])];
        for (const candidate of [doc.session.opened, fresh]) {
            if (candidate.recoveredDraft != null && candidate.recoveredDraft !== initialMergeSource)
                variants.push({ path: candidate.path, baseSource: candidate.recoveredDraftBaseSource ?? candidate.source, source: candidate.recoveredDraft, baseKnown: candidate.recoveredDraftBaseSource != null });
        }
        const seen = new Set<string>();
        for (const variant of variants) {
            const key = JSON.stringify(variant);
            if (seen.has(key))
                continue;
            seen.add(key);
            const details = el("details", "recovery-member");
            details.open = true;
            details.append(el("summary", "", displayMessage("shell.anotherRetainedDraft", { p0: uiDisplayParameter(variant.path) })));
            const draft = el("textarea", "text-input comparison-source");
            draft.readOnly = true;
            draft.rows = 8;
            draft.value = variant.source;
            setUiAttr(draft, "aria-label", displayMessage("shell.retainedDraft", { p0: uiDisplayParameter(variant.path) }));
            details.append(draft);
            if (variant.baseKnown) {
                const base = el("textarea", "text-input comparison-source");
                base.readOnly = true;
                base.rows = 5;
                base.value = variant.baseSource;
                setUiAttr(base, "aria-label", displayMessage("shell.draftOriginalVersionPath", { p0: uiDisplayParameter(variant.path) }));
                details.append(el("p", "field-help", uiMessage("shell.thisDraftOriginalVersion")), base);
            }
            else
                details.append(el("p", "field-help", uiMessage("shell.draftOriginalVersionUnconfirmed")));
            form.append(details);
        }
        const mergeLabel = el("label", "field-label", uiMessage("shell.mergedDraftEditable"));
        const merged = el("textarea", "text-input comparison-source");
        merged.value = initialMergeSource;
        merged.rows = 10;
        mergeLabel.append(merged);
        form.append(mergeLabel);
        const actions = el("div", "modal-actions");
        actions.append(button(uiMessage("shell.closeKeepDraft"), () => close(null), "button-secondary"), button(uiMessage("shell.useDiskVersion"), () => close({ action: "discardRecovered", source: fresh.source }), "button-secondary"), button(uiMessage("shell.applyMergedDraft"), () => close({ action: "mergeRecovered", source: textareaSourceDraft(initialMergeSource, merged.value) }), "button-primary"));
        form.append(actions);
    });
    if (!result) {
        if (doc === current)
            renderContent();
        return;
    }
    await api("/api/draft/resolve", { sessionId: fresh.sessionId, action: result.action, source: result.action === "mergeRecovered" ? result.source : undefined });
    await doc.writing.settleForSessionChange();
    if (doc.session.opened.sessionId !== confirmed.sessionId || doc.session.source !== confirmed.source || doc.session.serial !== confirmed.serial || doc.editor.generation !== confirmed.generation) {
        doc.session.opened.recoveredDraftVariants = [...(doc.session.opened.recoveredDraftVariants ?? []), { path: fresh.path, baseSource: fresh.source, source: result.source, baseKnown: true }];
        doc.comparisonTarget = fresh;
        await persistDraft(doc);
        doc.session.error = uiMessage("shell.mergeConfirmationNewInput");
        if (doc === current)
            renderContent();
        return;
    }
    fresh.recoveredDraftConflict = false;
    fresh.recoveredDraft = null;
    doc.session.opened = fresh;
    doc.editor.clearTokens();
    doc.sourceTrashed = fresh.kind === "managedTrash";
    doc.session.acknowledgeSave(fresh.source, fresh.baseId, fresh.path);
    doc.session.projection = null;
    doc.externalSource = undefined;
    doc.comparisonTarget = undefined;
    doc.lastSave = undefined;
    doc.editor.apply(0, doc.editor.source.length, result.source);
    await persistDraft(doc);
    workspaceTools.documentChanged();
    scheduleRender(doc, 0);
    renderContent();
    toast(result.action === "discardRecovered" ? uiMessage("shell.diskVersionAppliedRecoveryKept") : uiMessage("shell.explicitMergeApplied"));
}
async function showRecovery(): Promise<void> {
    const result = await api<RecoveryInfo[] | {
        recoveries: RecoveryInfo[];
    }>("/api/recovery");
    recoveries = Array.isArray(result) ? result : result.recoveries;
    await modal<void>(uiMessage("shell.recoveryRecords"), uiMessage("shell.recoveryRecordsDescription"), (form, close) => {
        const notes = el("section");
        noteInputRecovery.render(notes, localPersonalAudience());
        form.append(notes);
        if (!recoveries.length)
            form.append(el("p", "recovery-empty", uiMessage("shell.noPendingFileOperations")));
        for (const record of recoveries) {
            const item = el("div", "recovery-item");
            item.append(el("strong", "", record.path), el("p", "", coreDescriptionUiText(record.description)), el("span", "", displayMessage("shell.operationStatus", { p0: uiDisplayParameter(record.status) })));
            const inspect = button(uiMessage("shell.compareAndResolve"), async () => { await inspectRecovery(record.operationId); setUiText(inspect, uiMessage("shell.viewAgain")); }, "button-secondary compact");
            item.append(inspect);
            form.append(item);
        }
        form.append(button(uiMessage("common.close"), () => close(null), "button-primary"));
    });
}
async function inspectRecovery(id: string): Promise<void> {
    const record = await api<RecoveryInfo>("/api/recovery/inspect", { operationId: id });
    const decision = await modal<string>(uiMessage("shell.inspectFileRecovery"), uiMessage("shell.inspectFileRecoveryDescription"), (form, close) => {
        form.append(el("p", "field-help", () => `${record.path}\n${uiTextValue(coreDescriptionUiText(record.description))}`));
        const states: Record<string, UiText> = { knownBefore: uiMessage("shell.recoveryCompleteBefore"), knownAfter: uiMessage("shell.recoveryCompletePlanned"), knownIntermediate: uiMessage("shell.recoveryConfirmedIntermediate"), mixedKnown: uiMessage("shell.recoveryPartialAllFilesConfirmed"), thirdState: uiMessage("shell.recoveryOtherChanges"), unavailable: uiMessage("shell.recoveryUnreadable") };
        if (record.coherence)
            form.append(el("p", "field-help", states[record.coherence] ?? record.coherence));
        if (record.members?.length) {
            for (const member of record.members) {
                const details = el("details", "recovery-member");
                details.open = true;
                details.append(el("summary", "", () => `${uiTextValue(member.path)}${uiTextValue(member.targetPath ? ` → ${member.targetPath}` : "")} · ${uiTextValue(states[member.state] ?? member.state)}`));
                const columns = el("div", "comparison recovery-comparison");
                for (const [label, image] of [[uiMessage("shell.preOperationContent"), member.before], [uiMessage("shell.plannedContent"), member.after], [uiMessage("shell.currentContent"), member.current]] as const) {
                    const pane = el("div", "field-label");
                    pane.append(el("strong", "", label));
                    if (!image?.present)
                        pane.append(el("p", "field-help", uiMessage("shell.fileMissingOrUnreadable")));
                    else if (image.source !== undefined) {
                        const field = el("textarea", "text-input comparison-source");
                        field.readOnly = true;
                        field.rows = 8;
                        field.value = image.source;
                        setUiAttr(field, "aria-label", () => `${uiTextValue(label)}：${uiTextValue(member.path)}`);
                        pane.append(field);
                    }
                    else {
                        pane.append(el("p", "field-help", displayMessage("shell.binaryFileSize", { p0: uiDisplayParameter(image.byteLength.toLocaleString()) })));
                        if (image.base64)
                            pane.append(button(uiMessage("shell.downloadVersionToCompare"), () => {
                                const bytes = Uint8Array.from(atob(image.base64!), value => value.charCodeAt(0));
                                const url = URL.createObjectURL(new Blob([bytes]));
                                const link = el("a");
                                link.href = url;
                                link.download = `${uiTextValue(label)}-${uiTextValue(basename(member.path))}`;
                                link.click();
                                setTimeout(() => URL.revokeObjectURL(url), 1000);
                            }, "button-secondary compact"));
                    }
                    columns.append(pane);
                }
                details.append(columns);
                form.append(details);
            }
        }
        else {
            const columns = el("div", "comparison recovery-comparison");
            for (const [label, source] of [[uiMessage("shell.preOperationContent"), record.beforeSource], [uiMessage("shell.proposedContent"), record.afterSource], [uiMessage("shell.currentDiskContent"), record.currentSource]] as const) {
                const pane = el("label", "field-label", label!);
                const text = el("textarea", "text-input comparison-source");
                text.readOnly = true;
                text.rows = 10;
                text.value = source ?? "";
                if (source === undefined)
                    setUiAttr(text, "placeholder", uiMessage("shell.noReadableFileContent"));
                pane.append(text);
                columns.append(pane);
            }
            form.append(columns);
        }
        const actions = el("div", "modal-actions");
        actions.append(button(uiMessage("common.close"), () => close(null), "button-secondary"));
        const allowed = record.allowedActions ?? (!record.members?.length && !["resolved", "committed"].includes(record.status) ? ["keepCurrent"] : []);
        const labels: Record<string, UiText> = { keepCurrent: uiMessage("shell.keepOriginalStateUnblock"), adoptAfterImages: uiMessage("shell.acceptAllPlannedContent"), resumeOriginalPlan: uiMessage("shell.continueRemainingOriginalFiles") };
        for (const action of allowed)
            if (labels[action])
                actions.append(button(labels[action], () => close(action), "button-primary"));
        form.append(actions);
    });
    if (decision) {
        await api("/api/recovery/resolve", { operationId: id, action: decision });
        workspaceTools.invalidateQueryEnvironment();
        toast(uiMessage("shell.recoveryResolutionConfirmed"));
        await refreshInventory();
    }
}
async function insertLink(): Promise<void> {
    if (!current || readOnlySource(current) || current.mode === "read")
        return;
    const doc = current;
    const target = await modal<{
        target: string;
        label: string;
    }>(uiMessage("shell.insertLink"), uiMessage("shell.insertLinkDescription"), (form, close) => {
        const targetLabel = el("label", "field-label", uiMessage("shell.linkTargetAddress"));
        const input = el("input", "text-input");
        input.required = true;
        setUiAttr(input, "placeholder", uiMessage("shell.linkTargetPlaceholder"));
        targetLabel.append(input);
        form.append(targetLabel);
        if (inventory) {
            const options = el("select", "text-input link-targets");
            const empty = el("option", "", uiMessage("shell.chooseFromLibrary"));
            empty.value = "";
            options.append(empty);
            for (const entry of inventory.entries.filter(e => e.path.endsWith(".typ"))) {
                const option = el("option", "", entry.title || entry.relativePath);
                option.value = entry.path;
                options.append(option);
            }
            options.onchange = () => {
                if (options.value)
                    input.value = relativeFileTarget(doc.session.path, options.value);
            };
            form.append(options);
        }
        const textLabel = el("label", "field-label", uiMessage("shell.linkDisplayText"));
        const label = el("input", "text-input");
        setUiAttr(label, "placeholder", uiMessage("shell.linkTextPlaceholder"));
        const selection = doc.editor.view.state.selection.main;
        label.value = doc.editor.view.state.sliceDoc(selection.from, selection.to);
        textLabel.append(label);
        form.append(textLabel);
        const actions = el("div", "modal-actions");
        actions.append(button(uiMessage("common.cancel"), () => close(null), "button-secondary"));
        const submit = el("button", "button-primary", uiMessage("shell.insert"));
        submit.type = "submit";
        actions.append(submit);
        form.append(actions);
        form.onsubmit = event => {
            event.preventDefault();
            if (input.value.trim())
                close({ target: absoluteFileTarget(input.value.trim()), label: label.value || input.value.trim() });
        };
    });
    if (!target)
        return;
    const quoted = quoteTypstString(target.target);
    const selection = doc.editor.view.state.selection.main;
    doc.editor.apply(selection.from, selection.to, `#link(${quoted})[${encodeLiteral(target.label)}]`);
    doc.editor.view.focus();
}
async function navigateLink(target: string): Promise<void> {
    if (!current)
        return;
    if (target.startsWith("#")) {
        const anchor = renderHost?.querySelector(`#${CSS.escape(`native-${target.slice(1)}`)}`);
        if (anchor) {
            anchor.scrollIntoView({ block: "center", behavior: "smooth" });
            return;
        }
        throw new UiError(uiMessage("shell.previewAnchorMissing"));
    }
    const result = await api<{
        path?: string;
        root?: string;
        fragment?: string;
        externalUrl?: string;
        entityKey?: string;
        resourceRef?: string;
        ownerEntityKey?: string;
        revision?: string;
        resourceFragment?: {
            status: string;
            pageIndex?: number;
        };
    }>("/api/link/resolve", { sessionId: current.session.opened.sessionId, target });
    if (result.externalUrl) {
        await modal<void>(uiMessage("shell.openExternalLink"), result.externalUrl, (form, close) => { const link = el("a", "button-primary", uiMessage("shell.openInBrowser")); link.href = result.externalUrl!; link.target = "_blank"; link.rel = "noopener noreferrer"; form.append(link, button(uiMessage("common.cancel"), () => close(null), "button-secondary")); });
    }
    else if (result.resourceRef) {
        if (!result.root || !result.ownerEntityKey || !result.revision)
            throw new UiError(uiMessage("shell.resourceLinkExactIdentityMissing"));
        const owner = await api<OpenedDocument>("/api/data/open", { root: result.root, entityKey: result.ownerEntityKey });
        await acceptResolvedDocument(owner);
        workspaceTools.openResource(owner, { resource: result.resourceRef, revision: result.revision, pageIndex: result.resourceFragment?.status === "found" ? result.resourceFragment.pageIndex : undefined });
        if (result.fragment && result.resourceFragment?.status !== "found")
            toast(uiMessage("shell.resourceOpenedPositionUnconfirmed"), true);
    }
    else if (result.entityKey) {
        const opened = await api<OpenedDocument>("/api/data/open", { root: result.root, entityKey: result.entityKey });
        await acceptResolvedDocument(opened);
        if (current && result.fragment) {
            current.pendingFragment = result.fragment;
            scrollFragment(current);
        }
    }
    else if (result.path) {
        await openDocument(result.path, result.root);
        if (current && result.fragment) {
            current.pendingFragment = result.fragment;
            scrollFragment(current);
        }
    }
}
function scrollFragment(doc: LiveDocument): void {
    if (doc !== current || !doc.pendingFragment || !doc.session.projectionCurrent || !renderHost)
        return;
    const fragment = doc.pendingFragment;
    let decoded = fragment;
    try {
        decoded = decodeURIComponent(fragment);
    }
    catch { /* Exact invalid escape remains identifiable. */ }
    const target = renderHost.querySelector(`#${CSS.escape(`native-${fragment}`)}`) ?? renderHost.querySelector(`#${CSS.escape(`native-${decoded}`)}`);
    if (target) {
        target.scrollIntoView({ block: "center" });
        doc.pendingFragment = undefined;
    }
}
async function bootstrap(): Promise<void> {
    try {
        const result = await api<{
            library?: Inventory | null;
            inventory?: Inventory | null;
            initialDocument?: OpenedDocument | null;
            initialDocumentError?: ApiErrorBody | null;
            recoveries?: RecoveryInfo[];
            recentLibraries?: string[];
        }>("/api/bootstrap");
        inventory = result.inventory ?? result.library ?? inventory;
        recoveries = result.recoveries ?? [];
        startupError = result.initialDocumentError ? apiErrorUiText(result.initialDocumentError) : null;
        if (result.initialDocument && !documents.length) {
            addDocument(result.initialDocument);
            try {
                inventory = await api<Inventory>("/api/library/open", { path: result.initialDocument.root });
            }
            catch { /* The file session remains usable without an inventory. */ }
        }
    }
    catch (error) {
        startupError = error instanceof Error ? errorUiText(error) : uiMessage("shell.localServiceUnavailable");
    }
    renderSidebar();
    renderContent();
    if (!startupError)
        void ensureOfflineFonts().catch(error => toast(displayMessage("shell.offlineFontsIncomplete", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) }), true));
}
window.addEventListener("beforeunload", event => {
    if (documents.some(doc => doc.session.dirty || pendingSave(doc) || doc.draftFailure || doc.writing.state.pending)) {
        event.preventDefault();
        event.returnValue = "";
    }
});
content.addEventListener("contextmenu", event => {
    if (!(event.target instanceof Element) || !event.target.closest(".flow-document") || event.target.closest("input,textarea,a,button,[contenteditable=true]"))
        return;
    event.preventDefault();
    openEverydayMenu(uiMessage("shell.currentBodyActions"), everydayWritingCommands(current?.writing, !!current && sourceConsumersAvailable(current) && current.mode !== "read" && !readOnlySource(current) && !current.editor.composing, insertEverydayQuery), toast, false);
});
document.addEventListener("keydown", event => {
    if (event.isComposing || event.keyCode === 229 || current?.editor.composing || current?.writing.state.composing)
        return;
    if (event.target instanceof Element && event.target.closest("dialog"))
        return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "p") {
        event.preventDefault();
        openEverydayMenu(event.shiftKey ? uiMessage("cli.commandsHeading") : uiMessage("shell.quickOpen"), event.shiftKey ? everydayCommands() : everydayEntries().map(entry => ({ id: entry.key, label: entry.title, detail: entry.detail, unavailable: entry.unavailable, run: entry.open })), toast);
        return;
    }
    if (event.altKey && ["ArrowLeft", "ArrowRight"].includes(event.key)) {
        event.preventDefault();
        void stepNavigation(event.key === "ArrowLeft" ? -1 : 1).catch(showError);
        return;
    }
    if (event.target instanceof Element && event.target.closest('.embedded-query-dialog, .embedded-query-builder, .embedded-query-parameters, .document-notes-dialog, .numbering-dialog'))
        return;
    if (event.defaultPrevented)
        return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (current)
            void saveDocument(current).catch(showError);
    }
    if ((event.ctrlKey || event.metaKey) && ["z", "y"].includes(event.key.toLowerCase()) && current?.mode === "write" && !event.isComposing && !(event.target instanceof Element && event.target.closest("input,textarea,[contenteditable=true]"))) {
        event.preventDefault();
        const doc = current;
        const redo = event.shiftKey || event.key.toLowerCase() === "y";
        void flushLiteral().then(() => redo ? doc.editor.redo() : doc.editor.undo()).catch(showError);
    }
});
renderSidebar();
renderContent();
void bootstrap();
import { coreDescriptionUiText, engineDiagnosticUiText } from "./ui-diagnostics";
