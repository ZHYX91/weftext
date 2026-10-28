import { OrderedDraft, type OrderedDraftTransmission } from "./ordered-draft";
import { WorkspaceRibbon, type RibbonTab } from "./workspace-chrome";
import "./workspace-chrome.css";
import { parseUiMessage, disposeUiTree } from "./ui-locale";
import { decodeRawDiagnostics, decodeErrorCauses, descriptionUiText } from "./ui-diagnostics";
import { ApiError, decodeApiError } from "./api-error";
import { joinUiText } from "./ui-locale";
import { InterfaceLocaleControl, hostedLocalePreferences, publicLocalePreferences } from "./ui-locale-settings";
import { locale } from "./ui-locale";
import "./ui-locale.css";
import { bindDeclaredUi } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError } from "./ui-locale";
import { setUiText, type UiText } from "./ui-locale";
import "./hosted.css";
import { HostedDraftCache } from "./hosted-draft-cache";
import { connectHostedPanels } from "./hosted-panel-connection";
import { EverydayNavigation, type EverydayCommand } from "./everyday-navigation";
import { openEverydayMenu, openEverydayPicker } from "./everyday-ui";
import { HostedFields } from "./hosted-fields";
import { HostedReview } from "./hosted-review";
import { HostedOffice } from "./hosted-office";
import { HostedEditor } from "./hosted-editor";
import { HostedQuery } from "./hosted-query";
import { HostedHistory } from "./hosted-history";
import { PersonalInputRecovery, PERSONAL_COPY_ONLY_SLOT, isPersonalCopy, personalInputText, type BrowserProposalAck, type PersonalInputRecord, type PersonalOriginalRouteProof } from "./personal-input-recovery";
import { renderPersonalInputs } from "./personal-input-panel";
import { replayPersonalInput } from "./source-input-origin";
import { PersonalControlInput } from "./personal-control-input";
import type { AuthorInputFactory, AuthorInputSession, AuthorProducerState } from "./author-input";
import { freezeOriginalAttempt } from "./original-operation";
import { HostedLibrary } from "./hosted-library";
import { HostedMedia } from "./hosted-media";
import { HostedStructure } from "./hosted-structure";
import { mountHostedCapture, type MountedHostedCapture } from "./capture-hosted-mount";
import { decodeStructureCatalog, type StructureCatalog } from "./hosted-structure-state";
import { action as captureAction, ui as captureUi } from "./native-value-form";
import "./capture.css";
import { HostedKnowledgeLinks } from "./hosted-knowledge-links";
import { HostedCitations } from "./hosted-citations";
import { HostedNavigationRegistry } from "./hosted-navigation-registry";
import { HostedContentImport } from "./hosted-content-import";
import { HostedOwnerBackup } from "./hosted-owner-backup";
import { LatexInsertionPanel } from "./latex-insertion";
import { DiagramPanel } from "./diagram-ui";
import type { PanelHost } from "./data-panel";
import { decodeProcessingDocument } from "./processing-settings-panel";
import type { StructureReceipt, StructureRequest } from "./hosted-structure-state";
import { nodeFragmentOrigin, nodeReferenceParts, nodeSourceHash, nodeTargetDraft, sameHostedNodeAudience, type HostedNodeAudience, type HostedNodeBasis, type HostedNodeFragmentResult, type HostedNodeResolution } from "./hosted-navigation-state";
import { currentHostedHistoryResult, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, type HistoryContext } from "./history-state";
import type { HostedQueryRow } from "./hosted-query-state";
import { RecognitionPanel } from "./recognition-panel";
import { bindHostedDraftReference, draftDisposition, applySaveResult, freezeHostedSave, hostedOpenedDraft, hostedDocumentDraft, qualifyHostedDraft, mergeDefinitionReservations, hostedCachePrefix, sameHostedQueryAudience, sameHostedQueryIdentity, type HostedDraft } from "./hosted-state";
type Rights = {
    read: boolean;
    write: boolean;
    history: boolean;
    download: boolean;
};
type Doc = {
    viewInputId?: string;
    resource: string;
    title: string;
    sessionId: string;
    baseId: string;
    source: string;
    recoveredDraft?: string;
    recoveredDraftBaseSource?: string;
    recoveredDraftWitness?: string;
    recoveredDraftConflict: boolean;
    rights: Rights;
};
type Me = {
    actor: string;
    deployment: string;
    generation: number;
    version: string;
};
type SaveStatus = {
    status: string;
    operationId: string;
    resource?: string;
    baseId?: string;
};
// Only an actual install establishes adoption of the reviewed recovery primary.
const recoveryContinuations = new WeakMap<Doc, Promise<void> | null>();
function checkedSaveStatus(result: SaveStatus, operationId: string, resource: string): SaveStatus {
    if (!result || typeof result !== "object" || result.operationId !== operationId || !["committed", "cancelled", "resolved", "recoveryPending", "notCommitted", "notRegistered"].includes(result.status) || result.status !== "notRegistered" && result.resource !== resource || result.status === "committed" && (typeof result.baseId !== "string" || !result.baseId))
        throw new UiError(uiMessage("hostedShell.saveReceiptBasisMismatch"));
    return result;
}
const root = document.querySelector<HTMLElement>("#hosted")!;
root.classList.add("hosted-shell");
root.innerHTML = `<header><span class="brand">Weftext</span><span><span data-ui-message="hostedShell.remotePreviewEdition"></span></span><button id="logout" hidden><span data-ui-message="hostedShell.logoutClearCache"></span></button></header>
<form id="login"><h1><span data-ui-message="hostedShell.connectSpace"></span></h1><p><span data-ui-message="hostedShell.credentialsDescription"></span></p><label><span data-ui-message="hostedShell.accessCredentials"></span><input id="token" type="password" autocomplete="off" required minlength="32"></label><button><span data-ui-message="hostedShell.connect"></span></button></form>
<p id="status" role="status" aria-live="polite"><span data-ui-message="hostedShell.notConnected"></span></p>
<section id="workspace" hidden><aside><h2><span data-ui-message="hostedShell.accessibleDocuments"></span></h2><form id="search"><input id="term" data-ui-aria-label="hostedShell.searchAuthorizedDocuments" aria-label="" data-ui-placeholder="hostedShell.searchDocumentsPlaceholder" placeholder=""><button><span data-ui-message="hostedShell.search"></span></button></form><button id="refresh"><span data-ui-message="hostedShell.allDocuments"></span></button><nav id="documents" data-ui-aria-label="hostedShell.documents" aria-label=""></nav></aside>
<article><h1 id="title"><span data-ui-message="hostedShell.selectDocument"></span></h1><p class="hint"><span data-ui-message="hostedShell.sharedDraftUndoHint"></span></p>
<div class="actions"><button id="save" disabled><span data-ui-message="common.save"></span></button><button id="preview" disabled><span data-ui-message="hostedShell.preview"></span></button><button id="download" disabled><span data-ui-message="hostedShell.downloadSavedSource"></span></button><button id="history" disabled><span data-ui-message="hostedShell.saveHistory"></span></button><button id="personal-input-recovery" hidden><span data-ui-message="hostedShell.retrievePersonalUnuploadedInput"></span></button><button id="reopen" disabled><span data-ui-message="hostedShell.rereadAndCheck"></span></button><button id="latex" disabled><span data-ui-message="hostedShell.latexEquation"></span></button><button id="diagram" disabled><span data-ui-message="hostedShell.mermaidDiagram"></span></button></div>
<p id="stable-link" hidden><label><span data-ui-message="hostedShell.authorizedStableNoteLink"></span><input id="node-reference" data-ui-aria-label="hostedShell.authorizedStableNoteLink" aria-label="" readonly></label><button id="copy-reference" disabled><span data-ui-message="hostedShell.copyStableNoteLink"></span></button></p>
<section id="node-navigation" data-ui-aria-label="hostedShell.stableFragmentNavigation" aria-label="" hidden></section>
<section id="source" data-ui-aria-label="hostedShell.hostedDocumentEditor" aria-label=""></section>
<section id="comparison" hidden><h2><span data-ui-message="hostedShell.compareKeepBothDrafts"></span></h2><p><span data-ui-message="hostedShell.comparisonDescription"></span></p><pre id="current"></pre><button id="rebase"><span data-ui-message="hostedShell.confirmCurrentSaveBasis"></span></button></section>
<section id="history-list" data-ui-aria-label="hostedShell.hostedNoteTimeline" aria-label="" hidden></section>

<section id="personal-inputs" data-ui-aria-label="hostedShell.personalUnuploadedInputOperations" aria-label="" hidden></section>
<section id="library" data-ui-aria-label="hostedShell.hostedLibraryConfigurationStyles" aria-label="" hidden></section>
<section id="structure" data-ui-aria-label="hostedShell.hostedStructureOperations" aria-label="" hidden></section>
<section id="fields" data-ui-aria-label="hostedShell.authorizedFieldsQueries" aria-label="" hidden></section>
<section id="query" data-ui-aria-label="hostedShell.hostedAdvancedQuery" aria-label="" hidden></section>
<section id="knowledge-links" data-ui-aria-label="hostedShell.hostedKnowledgeLinksMentions" aria-label="" hidden></section>
<section id="citations" data-ui-aria-label="hostedShell.hostedFormalCitations" aria-label="" hidden></section>
<section id="navigation-registry" data-ui-aria-label="hostedShell.hostedContentsTagsNavigation" aria-label="" hidden></section>
<section id="review" data-ui-aria-label="hostedShell.hostedAnnotationsProofreading" aria-label="" hidden></section>
<section id="media" data-ui-aria-label="hostedShell.hostedAuthorizedMediaResources" aria-label="" hidden></section>
<section id="office" data-ui-aria-label="hostedShell.officeImportOutput" aria-label="" hidden></section>
<section id="content-import" data-ui-aria-label="hostedShell.hostedNativeContentLiteratureImport" aria-label="" hidden></section>
<section id="owner-backup" data-ui-aria-label="hostedShell.hostedFullRecoveryBackup" aria-label="" hidden></section>
<iframe id="render" data-ui-title="hostedShell.authorizedContentPreview" title="" sandbox="" hidden></iframe></article></section>`;
bindDeclaredUi(root);
const interfaceLocaleHost = document.createElement("div");
interfaceLocaleHost.className = "interface-locale-host";
root.querySelector("header")!.classList.add("hosted-app-header");
root.querySelector("header")!.append(interfaceLocaleHost);
let interfaceLocale: InterfaceLocaleControl | undefined;
function mountInterfaceLocale(audience?: {
    deployment: string;
    actor: string;
}): void {
    interfaceLocale?.dispose();
    interfaceLocale = new InterfaceLocaleControl(audience ? hostedLocalePreferences(localStorage, audience.deployment, audience.actor) : publicLocalePreferences(localStorage, location.origin));
    interfaceLocaleHost.replaceChildren(interfaceLocale.element);
}
mountInterfaceLocale();
document.documentElement.lang = locale.current;
locale.subscribe(value => { document.documentElement.lang = value; });
window.addEventListener("pagehide", event => {
    if (!event.persisted)
        interfaceLocale?.dispose();
});
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const everydayNavigation = new EverydayNavigation();
let navigatingHistory = false;
const everydayBar = document.createElement("nav");
everydayBar.className = "everyday-bar";
setUiAttr(everydayBar, "aria-label", uiMessage("shell.everydayNavigation"));
const ribbon = new WorkspaceRibbon();
const documentTabs = document.createElement("nav");
documentTabs.className = "hosted-document-tabs";
documentTabs.setAttribute("role", "tablist");
setUiAttr(documentTabs, "aria-label", uiMessage("shell.openDocuments"));
const openDocumentTitles = new Map<string, string>();
const openDocumentButtons = new Map<string, HTMLButtonElement>();
const inspector = document.createElement("aside");
inspector.className = "hosted-inspector";
inspector.hidden = true;
setUiAttr(inspector, "aria-label", uiMessage("hostedChrome.inspector"));
const inspectorHeader = document.createElement("header");
inspectorHeader.className = "hosted-inspector-header";
const inspectorTitle = document.createElement("strong");
const inspectorBody = document.createElement("div");
inspectorBody.className = "hosted-inspector-body";
const inspectorClose = document.createElement("button");
inspectorClose.type = "button";
setUiText(inspectorClose, uiMessage("common.close"));
inspectorClose.onclick = () => closeHostedInspector();
inspectorHeader.append(inspectorTitle, inspectorClose);
inspector.append(inspectorHeader, inspectorBody);
type HostedPanel = { host: HTMLElement; slot: HTMLElement; button: HTMLButtonElement; trigger: HTMLButtonElement; triggerTab: RibbonTab; tab: RibbonTab; title: UiText; wide: boolean };
const hostedPanels = new Map<string, HostedPanel>();
let selectedHostedPanel: string | null = null;
const panelObservers: MutationObserver[] = [];
function closeHostedInspector(): void {
    const previous = selectedHostedPanel ? hostedPanels.get(selectedHostedPanel) : undefined;
    selectedHostedPanel = null;
    for (const panel of hostedPanels.values()) {
        panel.slot.hidden = true;
        panel.button.setAttribute("aria-pressed", "false");
    }
    inspector.hidden = true;
    if (previous) {
        ribbon.select(previous.triggerTab, true);
        ribbon.setCollapsed(false);
        if (previous.trigger.isConnected && !previous.trigger.disabled && !previous.trigger.hidden) previous.trigger.focus();
    }
}
function selectHostedPanel(id: string, origin?: { button: HTMLButtonElement; tab: RibbonTab }): boolean {
    const selected = hostedPanels.get(id);
    // The original controller, not layout selection, owns authorization/availability.
    if (!selected || selected.host.hidden) return false;
    if (origin) { selected.trigger = origin.button; selected.triggerTab = origin.tab; }
    selectedHostedPanel = id;
    for (const [key, panel] of hostedPanels) {
        panel.slot.hidden = key !== id;
        panel.button.setAttribute("aria-pressed", String(key === id));
    }
    setUiText(inspectorTitle, selected.title);
    inspector.classList.toggle("wide", selected.wide);
    inspector.hidden = false;
    return true;
}
function syncHostedPanels(): void {
    for (const panel of hostedPanels.values()) panel.button.disabled = busy || panel.host.hidden;
    if (selectedHostedPanel && hostedPanels.get(selectedHostedPanel)?.host.hidden) closeHostedInspector();
}
function renderHostedDocumentTabs(): void {
    const retained = new Set(source.openResources);
    for (const resource of openDocumentTitles.keys()) {
        if (retained.has(resource)) continue;
        openDocumentTitles.delete(resource);
        const button = openDocumentButtons.get(resource);
        if (button) { disposeUiTree(button); button.remove(); }
        openDocumentButtons.delete(resource);
    }
    if (doc && retained.has(doc.resource)) openDocumentTitles.set(doc.resource, doc.title);
    for (const [resource, title] of openDocumentTitles) {
        let button = openDocumentButtons.get(resource);
        if (!button) {
            button = document.createElement("button");
            button.type = "button";
            button.setAttribute("role", "tab");
            button.setAttribute("aria-controls", "source");
            button.onclick = () => { void run(() => open(resource)); };
            openDocumentButtons.set(resource, button);
            documentTabs.append(button);
        }
        if (button.textContent !== title) setUiText(button, title);
        button.disabled = busy;
        button.tabIndex = doc?.resource === resource ? 0 : -1;
        button.setAttribute("aria-selected", String(doc?.resource === resource));
    }
}
documentTabs.addEventListener("keydown", event => {
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const buttons = [...openDocumentButtons.values()].filter(button => !button.disabled);
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (index < 0 || !buttons.length) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next].focus();
});
function everydayEntries() { return structureDocuments.map(item => ({ key: item.resource, title: item.title, detail: item.resource, open: () => run(() => open(item.resource)) })); }
function everydayCommands(): EverydayCommand[] {
    return [
        { id: "quick-open", label: uiMessage("shell.quickOpen"), run: () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), status) },
        { id: "relationship-graph", label: uiMessage("shell.openRelationshipGraph"), run: async () => { await knowledgeLinks.showStructure("graph"); selectHostedPanel("knowledge-links"); } },
        { id: "heading-structure", label: uiMessage("shell.openHeadingStructure"), run: async () => { await knowledgeLinks.showStructure("headings"); selectHostedPanel("knowledge-links"); } },
        { id: "recent", label: uiMessage("shell.recentVisits"), run: showRecent },
        { id: "save", label: uiMessage("shell.saveCurrentNote"), unavailable: !authoringEditable() ? uiMessage("hostedShell.bodyCannotSave") : undefined, run: () => run(save) },
        ...source.commands(async () => { el("query").hidden = false; selectHostedPanel("query"); el("query").scrollIntoView(); el("query").querySelector<HTMLInputElement>('input[data-control-role="definition-label"]')?.focus(); status(uiMessage("hostedShell.queryPromotePlanHint")); }),
    ];
}
function showRecent() { const entries = everydayEntries(); openEverydayPicker(uiMessage("shell.recentVisits"), everydayNavigation.recent.flatMap(key => entries.filter(item => item.key === key)), status); }
async function stepNavigation(direction: -1 | 1) {
    const target = everydayNavigation.target(direction);
    if (!target)
        return;
    navigatingHistory = true;
    try {
        await open(target);
        if (doc?.resource === target)
            everydayNavigation.arrive(direction, target);
    }
    finally {
        navigatingHistory = false;
        renderEverydayBar();
    }
}
function hostedChromeIcon(name: string): SVGSVGElement {
    const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("fill", "none");
    icon.setAttribute("stroke", "currentColor");
    icon.setAttribute("stroke-width", "1.6");
    icon.setAttribute("stroke-linecap", "round");
    icon.setAttribute("stroke-linejoin", "round");
    icon.setAttribute("aria-hidden", "true");
    icon.setAttribute("focusable", "false");
    const shapes: Record<string, string> = { back: "M14 5l-7 7 7 7", forward: "M10 5l7 7-7 7", "quick-open": "M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0", commands: "M4 6h16M4 12h16M4 18h16", recent: "M3 11a9 9 0 1 1 3 8M3 4v7h7", insert: "M12 4v16M4 12h16", navigation: "M3 4h18v16H3zM9 4v16" };
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", shapes[name] ?? shapes.commands);
    icon.append(path);
    return icon;
}
const everydayControls = new Map<string, HTMLButtonElement>();
function renderEverydayBar() {
    if (doc && !navigatingHistory) everydayNavigation.visit(doc.resource);
    const add = (id: string, label: UiText, action: () => void | Promise<void>, disabled = false) => {
        let button = everydayControls.get(id);
        if (!button) {
            button = document.createElement("button");
            button.type = "button";
            button.dataset.command = id;
            setUiAttr(button, "aria-label", label);
            setUiAttr(button, "title", label);
            const caption = document.createElement("span");
            caption.className = "hosted-command-label";
            setUiText(caption, label);
            button.append(hostedChromeIcon(id), caption);
            button.onclick = () => { void Promise.resolve().then(action).catch(error => status(errorUiText(error))); };
            everydayControls.set(id, button);
        }
        button.disabled = disabled;
        if (["back", "forward", "quick-open", "commands"].includes(id) && button.parentElement !== everydayBar) everydayBar.append(button);
    };
    add("back", uiMessage("shell.back"), () => run(() => stepNavigation(-1)), !everydayNavigation.target(-1) || busy);
    add("forward", uiMessage("shell.forward"), () => run(() => stepNavigation(1)), !everydayNavigation.target(1) || busy);
    add("quick-open", uiMessage("shell.quickOpen"), () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), status), busy);
    add("recent", uiMessage("shell.recentVisits"), showRecent, busy);
    add("commands", uiMessage("cli.commandsHeading"), () => openEverydayMenu(uiMessage("cli.commandsHeading"), everydayCommands(), status), busy);
    add("insert", uiMessage("shell.insertBlockActions"), () => openEverydayMenu(uiMessage("shell.insertBlockActions"), source.commands(), status), busy || !doc);
}
const personalInputs = new PersonalInputRecovery();
const personalControls = new PersonalControlInput(document, context => context.audience === personalAudience() && context.epoch === authorizationEpoch, ({ context, id, revision, text, producerId, producerRevision }) => {
    if (context.owner.kind === "capture")
        throw new UiError(uiMessage("hostedShell.captureBodyRecoveryRejected"));
    try {
        observePersonalWrite(personalInputs.update(context.audience, [{ id, owner: context.owner, slot: context.slot, path: context.path, producer: { producerId, revision: producerRevision }, origin: { baseLength: 0, pieces: [], detached: text ? [{ id, revision, text }] : [], revision } }]), context.audience);
        controls();
    }
    catch (error) {
        status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.personalFormInputMemoryOnly"));
    }
});
const authorInputs: AuthorInputFactory = (owner, current, sink, leaseRevision) => { const audience = personalAudience(); return audience ? personalControls.session(audience, authorizationEpoch, owner, current, sink, leaseRevision) : undefined; };
const personalInputList = document.createElement("div"), personalCopyMount = document.createElement("section");
personalCopyMount.className = "personal-copy-editor";
interface HostedPersonalCopy {
    audience: string;
    epoch: number;
    credential: string;
    version: number;
    originalId: string;
    record: PersonalInputRecord;
    session: AuthorInputSession;
    textarea: HTMLTextAreaElement;
    status: HTMLElement;
    finish: HTMLButtonElement;
    finishing: boolean;
    retentionFailure?: boolean;
    lastWrite?: Promise<BrowserProposalAck>;
}
let personalCopyVersion = 0, personalCopyEditor: HostedPersonalCopy | undefined;
let hostedCapture: MountedHostedCapture | undefined;
let captureVersion = 0;
const capturePanel = document.createElement("section");
capturePanel.id = "capture-panel";
capturePanel.hidden = true;
el("structure").before(capturePanel);
const captureButton = document.createElement("button");
captureButton.type = "button";
setUiText(captureButton, uiMessage("capture.title"));
el("refresh").after(captureButton);
captureButton.onclick = () => void showCapturePicker().catch(error => status(errorUiText(error)));
function captureAudienceCurrent(captured: {
    actor: string;
    deployment: string;
    generation: number;
    epoch: number;
    credential: string;
}): boolean { return authorizationEpoch === captured.epoch && token === captured.credential && identity?.actor === captured.actor && identity.deployment === captured.deployment && identity.generation === captured.generation; }
async function showCapturePicker(): Promise<void> {
    if (!identity || !personalAudience())
        throw new UiError(uiMessage("hostedShell.reauthenticateOriginalAccount"));
    capturePanel.hidden = false;
    selectHostedPanel("capture-panel");
    if (hostedCapture) {
        capturePanel.scrollIntoView();
        return;
    }
    const captured = { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, epoch: authorizationEpoch, credential: token }, version = ++captureVersion;
    const current = () => version === captureVersion && captureAudienceCurrent(captured);
    const [catalog, views] = await Promise.all([request<StructureCatalog>("structure").then(decodeStructureCatalog), request<{
            recovery: {
                draftId: string;
            };
        }[]>("capture/draft/list", {})]);
    if (!current())
        return;
    const existing = captureUi("select", "data-input"), slots = captureUi("select", "data-input"), diagnostic = captureUi("p", "data-warning");
    for (const view of views) {
        const id = view.recovery?.draftId;
        if (!/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(id))
            throw new UiError(uiMessage("capture.rawRetained"));
        const option = captureUi("option", "", id);
        option.value = id;
        existing.append(option);
    }
    for (const slot of catalog.slots.filter(slot => slot.create && slot.status === "ready")) {
        const option = captureUi("option", "", slot.title);
        option.value = slot.resource;
        slots.append(option);
    }
    const activate = async (choice: {
        draftId?: string;
        slot?: string;
    }) => {
        if (!current())
            return;
        const mounted = await mountHostedCapture(capturePanel, { request, inputs: personalControls, authorInputs, catalog,
            options: { deployment: captured.deployment, actor: captured.actor, ...choice, recovery: personalInputs, current },
            deliverCommitted: async (resource) => {
                if (!current())
                    throw new UiError(uiMessage("capture.rawRetained"));
                await open(resource);
                if (!current() || doc?.resource !== resource || source.pausedSession)
                    throw new UiError(uiMessage("capture.pending"));
            },
            requestClose: async () => {
                if (!current() || !hostedCapture)
                    throw new UiError(uiMessage("desktop.unsavedClose"));
                const closing = hostedCapture;
                hostedCapture = undefined;
                closing.dispose();
                capturePanel.replaceChildren();
                capturePanel.hidden = true;
            },
        });
        if (!current()) {
            mounted.retireAuthorization();
            mounted.dispose();
            return;
        }
        hostedCapture = mounted;
        capturePanel.scrollIntoView();
    };
    const run = (work: () => Promise<void>) => async () => {
        try {
            await work();
        }
        catch (error) {
            if (current())
                setUiText(diagnostic, errorUiText(error));
        }
    };
    const restore = captureAction(uiMessage("capture.restoreDraft"), run(() => activate({ draftId: existing.value })));
    restore.disabled = existing.options.length === 0;
    const create = captureAction(uiMessage("capture.newDraft"), run(() => activate({ slot: slots.value })));
    create.disabled = slots.options.length === 0;
    capturePanel.replaceChildren(captureUi("h2", "", uiMessage("capture.title")), captureUi("p", "", uiMessage("capture.restoreDraft")), existing, restore, captureUi("p", "", uiMessage("capture.target")), slots, create, diagnostic);
}
const fields = new HostedFields(el("fields"), request, (id, metadata) => showOfficeQuery(id, metadata), () => queries.environmentChanged(uiMessage("hostedShell.fieldsChangedQueryMustRerun")), authorInputs);
const source: HostedEditor = new HostedEditor(el("source"), request, sourceChanged, () => { void run(save); }, error => status(errorUiText(error)), {
    canOpenRow: row => (row as HostedQueryRow).canOpen === true && !!(row as HostedQueryRow).resource,
    openRow: row => {
        const target = row as HostedQueryRow;
        if (target.canOpen !== true || !target.resource)
            return Promise.reject(new UiError(uiMessage("hostedShell.queryRowNoBodyReadRight")));
        return openQueryResource(target.resource);
    },
    openOwner: async () => { throw new UiError(uiMessage("hostedShell.querySourceNoAuthorizedNavigation")); },
    output: (query, result) => showOfficeQuery(query.queryId, { columns: result.columns ?? [], rowCount: result.rows.length, coverage: result.coverage }),
    modeChanged: () => { queries.authorChanged(); knowledgeAuthorChanged(); },
    showHeadingStructure: async () => { await knowledgeLinks.showStructure("headings"); selectHostedPanel("knowledge-links"); },
    navigateReference,
    navigateResource,
    dependencies: () => queries.dependencies(),
    personalInputChanged: () => {
        try {
            persistPersonalInputs();
            controls();
        }
        catch (error) {
            status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.personalInputNotDurable"));
        }
    },
    authorInputs,
});
const review: HostedReview = new HostedReview(el("review"), request, () => doc ? {
    resource: doc.resource, sessionId: doc.sessionId, path: doc.resource, root: "", source: source.value,
    serial: source.serial, inputId: source.authorInputId, baseId: doc.baseId, dirty: source.value !== doc.source || compareRequired,
    pending: !!draft?.pending, selection: source.selection,
} : null, () => source.flush(), (from, to) => source.select(from, to), (resource, changed) => {
    if (changed && doc?.resource === resource) {
        compareRequired = true;
        knowledgeAuthorChanged();
        controls();
        status(uiMessage("hostedShell.suggestionCommittedReopenBody"));
    }
}, { context: () => media.context(), open: (asset, revision, initial) => media.openAsset(asset, revision, initial) }, readSharedProcessingDefaults, authorInputs);
let token = sessionStorage.getItem("weftext-hosted-token") ?? "";
let identity: Me | undefined;
let verifiedPersonalAudience = "";
let doc: Doc | undefined;
let draft: HostedDraft | undefined;
let cachePrefix = "";
const draftCache = new HostedDraftCache(sessionStorage, () => cachePrefix);
let busy = false;
let actionVersion = 0;
let requestEpoch = 0;
let authorizationEpoch = 0;
let compareRequired = false;
let installingDocument = false;
let draftTimer: ReturnType<typeof setTimeout> | undefined;
let expiredSessionId: string | undefined;
const authoringEditable = (): boolean => !!doc?.rights.write && !busy && !source.pausedSession && source.viewMode !== "read" && !compareRequired && !draft?.pending && !history.hasPending(doc.resource);
const authoringHost: PanelHost = {
    authorInputs: basis => { const selected = doc; return selected ? authorInputs({ kind: "body", resource: selected.resource, reference: draft?.reference }, () => doc === selected && doc.sessionId === basis.sessionId && !source.pausedSession, undefined, () => source.serial) : undefined; },
    request: <T>(path: string, body?: unknown): Promise<T> => {
        if (path !== "/api/editor/preview")
            return Promise.reject(new UiError(uiMessage("hostedShell.writingControlReadEndpointUnavailable")));
        return request<T>("editor/preview", body);
    },
    getDocument: () => doc && identity && !source.pausedSession ? { sessionId: doc.sessionId, source: source.value, serial: source.serial, inputId: source.authorInputId, baseId: doc.baseId, path: `resource:${doc.resource}`, root: "", dirty: source.value !== doc.source } : null,
    applyPatch: async (patch, basis) => {
        await source.flush();
        if (!authoringEditable() || doc?.sessionId !== basis.sessionId || doc.baseId !== basis.baseId || source.value !== basis.source || source.serial !== basis.serial || source.authorInputId !== basis.inputId)
            throw new UiError(uiMessage("hostedShell.hostedProposalBasisChanged"));
        retain();
        await source.applyQueryPatches([{ from: patch.from, to: patch.to, insert: patch.text, expected: basis.source.slice(patch.from, patch.to) }], basis, "input.authoring.proposal");
    },
    openDocument: async () => { throw new UiError(uiMessage("hostedShell.cannotOpenHostPath")); },
    notify: message => status(message), prepareAction: async () => {
        await source.flush();
        if (!authoringEditable())
            throw new UiError(uiMessage("hostedShell.checkDraftSaveBeforeEditing"));
        retain();
    },
};
const latexAuthoring = new LatexInsertionPanel({ ...authoringHost, selection: () => source.selection, editable: authoringEditable });
const diagramAuthoring = new DiagramPanel(authoringHost, authoringEditable);
el<HTMLButtonElement>("latex").onclick = () => { void latexAuthoring.open().catch(error => status(error instanceof Error ? errorUiText(error) : String(error))); };
el<HTMLButtonElement>("diagram").onclick = () => { void diagramAuthoring.open().catch(error => status(error instanceof Error ? errorUiText(error) : String(error))); };
const retiredStructureSessions = new Set<string>();
const retiredStructureSources = new Set<string>();
function sessionAudience() { return { authorization: authorizationEpoch, credential: token, actor: identity?.actor, generation: identity?.generation, deployment: identity?.deployment }; }
type HostedSessionAudience = Pick<HostedNodeAudience, "authorization" | "credential" | "actor" | "generation" | "deployment">;
async function closeOwnedSession(sessionId: string, captured: HostedSessionAudience): Promise<void> {
    const entry = hostedWriterRetirements.get(sessionId) ?? { sessionId, audience: captured };
    hostedWriterRetirements.set(sessionId, entry);
    if (entry.closing) return entry.closing;
    const closing = (async () => {
        if (Array.from(hostedWriterRetirements.values()).some(item => item.waitFor?.target.sessionId === sessionId))
            await confirmHostedRetirementCuts(sessionId);
        if (entry.waitFor) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
        if (!entry.audience.credential) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
        const response = await fetch("/api/hosted/close", { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${entry.audience.credential}` }, body: JSON.stringify({ sessionId }), cache: "no-store", keepalive: true, signal: AbortSignal.timeout(10000) });
        if (response.status === 410) await response.arrayBuffer();
        else if (!response.ok || (await response.json()).closed !== true) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
        if (hostedWriterRetirements.get(sessionId) === entry) hostedWriterRetirements.delete(sessionId);
    })();
    entry.closing = closing;
    try { await closing; } finally { if (entry.closing === closing) entry.closing = undefined; }
}
function releaseViewSessions() {
    const captured = sessionAudience(), selected = doc;
    const waiting = selected && Array.from(hostedWriterRetirements.values()).some(entry => entry.waitFor?.target.sessionId === selected.sessionId);
    const input = selected && waiting ? hostedInput(selected) : undefined;
    const transmission = selected && input ? draftOrder(selected).issued(input) : undefined;
    const sessions = new Set(retiredStructureSessions);
    if (selected) sessions.add(selected.sessionId);
    for (const id of sessions) {
        const entry = hostedWriterRetirements.get(id) ?? {sessionId:id,audience:captured};
        if (selected && input) entry.waitFor = {target:selected,input,audience:captured,transmission};
        hostedWriterRetirements.set(id,entry);
    }
    if (selected && input) {
        if (transmission) bindHostedTransmission(selected,transmission);
        else if (selected.rights.write) {
            // Departure is an actual current-input checkpoint. Capture it once;
            // all subsequent cleanup reuses this original transmission.
            const issued = issueHostedInput(selected,input,wire=>capturedDraftAck(wire,captured));
            void issued.result.then(()=>retireHostedWriters(selected,issued.transmission)).catch(()=>{});
        }
    }
    void retryHostedRetirements().catch(()=>{});
}
async function confirmHostedRetirementCuts(sessionId: string): Promise<void> {
    for (const entry of Array.from(hostedWriterRetirements.values())) {
        while (entry.waitFor?.target.sessionId === sessionId) {
            const pending = entry.waitFor, transmission = pending.transmission;
            if (!transmission) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
            const order = draftOrder(pending.target);
            await order.retry(transmission,wire=>capturedDraftAck(wire,pending.audience));
            await order.settled();
            if (entry.waitFor === pending && pending.transmission === transmission) entry.waitFor = undefined;
        }
    }
    if (Array.from(hostedWriterRetirements.values()).some(entry=>entry.waitFor?.target.sessionId===sessionId))
        throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
}
async function retryHostedRetirements(): Promise<void> {
    const waiting = new Set(Array.from(hostedWriterRetirements.values()).flatMap(entry => entry.waitFor ? [entry.waitFor.target.sessionId] : []));
    for (const id of waiting) {
        try { await confirmHostedRetirementCuts(id); } catch { /* Keep the exact target cut and both handles. */ }
    }
    for (const entry of Array.from(hostedWriterRetirements.values())) if (!entry.waitFor) {
        try { await closeOwnedSession(entry.sessionId, entry.audience); } catch { /* retain for the next owned cleanup */ }
    }
}
async function capturedDraftAck(wire: Parameters<Parameters<OrderedDraft["persist"]>[1]>[0], captured: HostedSessionAudience): Promise<{ inputRevision: number }> {
    const response = await fetch("/api/hosted/draft", { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${captured.credential}` }, body: JSON.stringify(wire), cache: "no-store", keepalive: true, signal: AbortSignal.timeout(10000) });
    if (!response.ok) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    const ack = await response.json();
    if (ack.status !== "inputAcknowledged" || !["authorDraft","savedView"].includes(ack.evidence) || ack.inputRevision !== wire.inputRevision) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    return ack;
}
async function refreshOpenSession() {
    const selected = doc, captured = sessionAudience();
    if (!selected || !identity || selected.sessionId === expiredSessionId)
        return;
    try {
        await request("session/keepalive", { sessionId: selected.sessionId }, false, 0, AbortSignal.timeout(10000));
    }
    catch (error) {
        const current = sessionAudience();
        if (doc !== selected || (["authorization", "credential", "actor", "generation", "deployment"] as const).some(key => captured[key] !== current[key]))
            return;
        if (error instanceof HostedError && error.http === 410 && error.code === "sessionExpired") {
            expiredSessionId = selected.sessionId;
            compareRequired = true;
            clearTimeout(draftTimer);
            try {
                retain();
                status(uiMessage("hostedShell.sessionExpiredDraftUndoRetained"));
            }
            catch {
                status(uiMessage("hostedShell.sessionExpiredRecoveryUnavailable"));
            }
            controls();
        }
    }
}
const status = (text: UiText) => { setUiText(el("status"), text); };
const office: HostedOffice = new HostedOffice(el("office"), request, () => doc ? {
    resource: doc.resource, source: source.value, sessionId: doc.sessionId, baseId: doc.baseId, serial: source.serial, paused: source.pausedSession, dirty: source.value !== doc.source || compareRequired, pending: !!draft?.pending || history.hasPending(doc.resource),
} : null, () => source.flush(), () => { void list().catch(e => status(e instanceof Error ? errorUiText(e) : uiMessage("hostedShell.refreshDocumentListFailed"))); }, {
    identity: hostedIdentity, basis: () => source.historyBasis(true), embeddedQueries: () => source.outputSelections(), authorizationChanged: authorizationsChanged,
});
const queries = new HostedQuery(el("query"), request, openQueryResource, (id, metadata) => showOfficeQuery(id, metadata), {
    context: () => doc ? { resource: doc.resource, sessionId: doc.sessionId, baseId: doc.baseId, source: source.value, serial: source.serial, inputId: source.authorInputId, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending, compare: compareRequired, definitionReservations: draft?.definitionReservations } : null,
    prepareBasis: () => source.prepareQueryBasis(),
    apply: (patches, basis) => source.applyQueryPatches(patches, basis, "input.query.definition.proposal"),
    authorInputs,
    reservations: (resource, receipts, includeReserved = false) => {
        if (!doc || !draft || resource !== doc.resource)
            return;
        draft.definitionReservations = mergeDefinitionReservations(draft.definitionReservations ?? [], receipts, includeReserved);
        retain();
    },
}, () => source.queryInputsChanged(), navigateResource, { canEdit: (resource, fieldId) => fields.canEdit(resource, fieldId), open: async (resource, fieldId) => { await fields.openGranted(resource, fieldId); selectHostedPanel("fields"); } });
const knowledgeLinks = new HostedKnowledgeLinks(el("knowledge-links"), (path, body, signal) => request(path, body, false, 0, signal), {
    identity: hostedIdentity,
    context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, inputId: source.authorInputId, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
    flush: async () => { await source.historyBasis(true); },
    apply: async (patches, basis, event) => { retain(); await source.applyQueryPatches(patches, basis, event); },
    navigate: reference => reference.startsWith("weftext-resource://") ? navigateResource(reference) : navigateReference(reference),
    select: (from, to) => source.selectSourceRange(from, to),
    revealRendered: heading => source.revealRendered(heading.key, heading),
    authorizationChanged: (message, current) => { clearDisplay(message); void confirmIdentity({ ...identity!, ...current }).catch(error => status(errorUiText(error))); },
});
function hostedIdentity() { return identity ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, authorization: authorizationEpoch } : null; }
async function readSharedProcessingDefaults() {
    const original = sessionAudience();
    const profile = await request<{
        source: string;
        basis: string;
    }>("library/read", { role: "settings" });
    const current = sessionAudience();
    if ((["authorization", "credential", "actor", "generation", "deployment"] as const).some(key => original[key] !== current[key]))
        throw new UiError(uiMessage("hostedShell.sharedSettingsReadPermissionChanged"));
    const settings = JSON.parse(profile.source) as {
        version?: unknown;
        processing?: unknown;
    };
    if (settings.version !== 1)
        throw new UiError(uiMessage("hostedShell.sharedSettingsVersionUnknown"));
    return { processing: settings.processing == null ? null : decodeProcessingDocument(settings.processing), source: uiMessage("hostedShell.sharedProcessingDefaults"), baseId: profile.basis };
}
function authorizationsChanged(message: UiText, current: {
    actor: string;
    deployment: string;
    generation: number;
}) {
    clearDisplay(message);
    if (identity)
        void confirmIdentity({ ...identity, ...current }).catch(error => status(errorUiText(error)));
}
const citations = new HostedCitations(el("citations"), request, {
    identity: hostedIdentity,
    context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, inputId: source.authorInputId, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
    basis: () => source.historyBasis(true), apply: async (patches, basis) => { retain(); await source.applyQueryPatches(patches, basis, "input.citation.proposal"); },
    authorInputs, selection: () => source.selection, select: (from, to) => source.selectSourceRange(from, to), navigate: navigateReference, authorizationChanged: authorizationsChanged,
});
function knowledgeAuthorChanged() { knowledgeLinks.documentChanged(); citations.documentChanged(); }
const navigationRegistry = new HostedNavigationRegistry(el("navigation-registry"), request, { authorInputs, identity: hostedIdentity, authorizationChanged: authorizationsChanged, registryChanged: () => queries.environmentChanged(uiMessage("hostedShell.tagStructureChangedQueryMustRerun")) });
const contentImport = new HostedContentImport(el("content-import"), request, { identity: hostedIdentity, authorizationChanged: authorizationsChanged, open: openQueryResource });
const recognitionPanel = new RecognitionPanel({
    request: (path, body) => request(`recognition${path}`, body),
    identity: () => identity ? JSON.stringify([identity.deployment, identity.actor, identity.generation]) : null,
    target: () => null,
    notify: message => status(message),
    created: async (receipt) => {
        const ref = receipt.nodeRefs[0];
        if (!ref)
            throw new UiError(uiMessage("hostedShell.newNoteNoIndependentRead"));
        await navigateReference(`weftext-node://${ref.workspaceId}/${ref.nodeId}`);
    },
});
const recognitionButton = document.createElement("button");
recognitionButton.type = "button";
setUiText(recognitionButton, uiMessage("shell.importRecognition"));
recognitionButton.onclick = () => { void recognitionPanel.open().catch(error => status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.recognitionUnavailable"))); };
el("workspace").querySelector("article > .actions")!.append(recognitionButton);
const ownerBackup = new HostedOwnerBackup(el("owner-backup"), request, { identity: hostedIdentity, authorizationChanged: authorizationsChanged });
const history = new HostedHistory(el("history-list"), request, {
    context: () => doc && draft ? { resource: doc.resource, sessionId: doc.sessionId, path: `resource:${doc.resource}`, root: "", baseId: doc.baseId, baseSource: doc.source, source: source.value, serial: source.serial, inputId: source.authorInputId, inputRevision: draftOrder(doc).acknowledged(hostedInput(doc)), dirty: source.value !== doc.source, readOnly: !doc.rights.write || source.viewMode === "read", savePending: !!draft.pending, unresolvedSave: compareRequired } : null,
    basis: hostedHistoryBasis, pendingChanged: () => { controls(); queries.authorChanged(); knowledgeAuthorChanged(); scheduleCurrentDraft(); }, restored: acknowledgeHistory, finishUnavailable: finishUnavailableHistory,
});
const library = new HostedLibrary(el("library"), request, async (role) => {
    const epoch = requestEpoch;
    await source.flush();
    if (epoch !== requestEpoch)
        return;
    source.refreshEnvironment();
    queries.environmentChanged();
    knowledgeLinks.environmentChanged();
    el<HTMLIFrameElement>("render").srcdoc = "";
    el("render").hidden = true;
    if (role === "schema" || role === "rules")
        await queries.refreshCatalog();
    if (epoch === requestEpoch)
        status(uiMessage("hostedShell.libraryConfigurationCommitted"));
}, authorInputs);
const media: HostedMedia = new HostedMedia(el("media"), (path, body, signal) => request(path, body, false, 0, signal), context => review.mediaChanged(context), context => review.openMedia(context));
let structureDocuments: {
    resource: string;
    title: string;
}[] = [];
const structures = new HostedStructure(el("structure"), request, {
    protect: async (intent, phase) => {
        const epoch = requestEpoch, selected = doc;
        await source.flush();
        if (epoch !== requestEpoch || doc !== selected)
            throw new UiError(uiMessage("hostedShell.structureProtectionBodyOrUserChanged"));
        retain();
        if (selected && draft && intent.source === selected.resource) {
            if (draft.pending || history.hasPending(selected.resource))
                throw new UiError(uiMessage("hostedShell.originalSaveRestoreUnverified"));
            if (source.pausedSession) {
                if (phase === "commit" || intent.kind === "restore")
                    return;
                throw new UiError(uiMessage("hostedShell.structureSessionPaused"));
            }
            const ref = await request<{
                resource: string;
                reference: string;
            }>("navigation/ref", { sessionId: selected.sessionId });
            if (epoch !== requestEpoch || doc !== selected)
                throw new UiError(uiMessage("hostedShell.bodySessionChangedDuringCheck"));
            draft = bindHostedDraftReference(draft, ref.reference);
            retain();
            await persistHostedDraft(selected);
            if (epoch !== requestEpoch || doc !== selected)
                throw new UiError(uiMessage("hostedShell.draftProtectionUserOrObjectChanged"));
            retain();
            if (phase === "commit" && ["rename", "move", "trash", "restore"].includes(intent.kind)) {
                retiredStructureSessions.add(selected.sessionId);
                retiredStructureSources.add(selected.resource);
                clearTimeout(draftTimer);
                clearNodeNavigation();
                source.pauseSession();
                knowledgeAuthorChanged();
                compareRequired = true;
                controls();
            }
        }
    },
    changed: async (receipt, intent) => {
        const epoch = requestEpoch;
        if (doc && draft && intent.source === doc.resource && ["rename", "move", "trash", "restore"].includes(intent.kind)) {
            retiredStructureSessions.add(doc.sessionId);
            retiredStructureSources.add(doc.resource);
            retain();
            clearTimeout(draftTimer);
            source.pauseSession();
            knowledgeAuthorChanged();
            compareRequired = true;
            controls();
        }
        await list();
        if (epoch !== requestEpoch)
            return;
        await queries.refreshCatalog();
        if (epoch !== requestEpoch)
            return;
        await media.refresh();
        if (epoch === requestEpoch)
            status(uiMessage("hostedShell.structureCommittedDraftRetained"));
    },
    open: openStructureResult,
});
async function openStructureResult(result: NonNullable<StructureReceipt["result"]>, intent: StructureRequest): Promise<void> {
    if (!result.canOpen)
        throw new UiError(uiMessage("hostedShell.structureResultNoBodyRead"));
    if (!["rename", "move", "restore"].includes(intent.kind)) {
        await openQueryResource(result.resource);
        return;
    }
    const epoch = requestEpoch, audience = sessionAudience(), selected = doc;
    await source.flush();
    if (epoch !== requestEpoch || audience.credential !== token)
        return;
    retain();
    const expected = `weftext-node://${result.nodeRef.workspaceId}/${result.nodeRef.nodeId}`;
    const original = selected?.resource === intent.source && draft ? { ...draft, source: source.value } : intent.source ? cached(intent.source) : undefined;
    const fresh = await request<Doc>("open", { resource: result.resource });
    let adopted = false;
    try {
        if (epoch !== requestEpoch || doc !== selected)
            return;
        const reference = await readDocumentReference(fresh);
        if (epoch !== requestEpoch || doc !== selected || reference !== expected)
            throw new UiError(uiMessage("hostedShell.structureNewLocationEligibilityChanged"));
        await retain();
        if (epoch !== requestEpoch || doc !== selected)
            return;
        if (original?.reference === expected) {
            const retained = selected?.resource === intent.source && draft ? { ...draft, source: source.value } : original;
            installDocument(fresh, retained, reference, false, false, selected?.resource === intent.source && source.pausedSession);
            adopted = true;
            if (fresh.rights.write && retained.source !== fresh.source) {
                await persistHostedDraft(fresh, retained.source);
                if (doc === fresh)
                    scheduleCurrentDraft();
            }
        }
        else {
            installDocument(fresh, cached(fresh.resource) ?? hostedDocumentDraft(fresh), reference);
            adopted = true;
        }
        if (selected)
            await closeOwnedSession(selected.sessionId, audience);
    }
    finally {
        if (!adopted)
            await closeOwnedSession(fresh.sessionId, audience);
    }
}
let stableReference: {
    resource: string;
    sessionId: string;
    reference: string;
} | undefined;
let nodeNavigationVersion = 0;
function clearNodeNavigation() { nodeNavigationVersion++; el("node-navigation").replaceChildren(); el("node-navigation").hidden = true; }
function nodeAudience(): HostedNodeAudience { return { epoch: requestEpoch, authorization: authorizationEpoch, credential: token, actor: identity?.actor, deployment: identity?.deployment, generation: identity?.generation, sessionId: doc?.sessionId, source: source.value, serial: source.serial, navigation: nodeNavigationVersion }; }
function nodeNavigationMessage(reference: string, message: UiText, onlyOpen = false) {
    const host = el("node-navigation");
    host.hidden = false;
    host.replaceChildren();
    const original = document.createElement("input");
    original.readOnly = true;
    original.value = reference;
    setUiAttr(original, "aria-label", uiMessage("hostedShell.originalStableFragmentLink"));
    const text = document.createElement("p");
    setUiText(text, message);
    text.setAttribute("role", "status");
    setUiAttr(text, "aria-label", uiMessage("hostedShell.stableFragmentLocationResult"));
    host.append(original, text);
    if (onlyOpen) {
        const base = reference.split("#", 1)[0], button = document.createElement("button");
        setUiText(button, uiMessage("hostedShell.openNoteWithoutFragment"));
        button.type = "button";
        button.onclick = () => { void navigateReference(base).catch(error => status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.targetUnavailable"))); };
        host.append(button);
    }
}
async function navigateResource(reference: string) {
    const epoch = authorizationEpoch;
    if (doc) {
        await source.flush();
        if (epoch !== authorizationEpoch)
            return;
        retain();
    }
    if (epoch === authorizationEpoch)
        await media.openReference(reference);
}
async function navigateReference(reference: string) {
    if (busy)
        throw new UiError(uiMessage("hostedShell.operationCheckingKeepLinkDraft"));
    const epoch = requestEpoch;
    await source.flush();
    retain();
    if (epoch !== requestEpoch)
        return;
    clearNodeNavigation();
    const captured = nodeAudience();
    let staged: Doc | undefined, adopted = false, adoptedAudience: ReturnType<typeof nodeAudience> | undefined;
    nodeNavigationMessage(reference, uiMessage("hostedShell.resolvingAuthorizedLink"));
    try {
        const parts = nodeReferenceParts(reference), resolved = await request<HostedNodeResolution>("navigation/resolve", { reference });
        if (!sameHostedNodeAudience(captured, nodeAudience()))
            return;
        if (!resolved.resource || resolved.reference !== reference)
            throw new UiError(uiMessage("hostedShell.stableLinkReceiptMismatch"));
        if (parts.fragment === undefined) {
            if (resolved.canOpen !== true)
                throw new UiError(uiMessage("hostedShell.stableLinkTargetUnavailable"));
            await run(() => open(resolved.resource));
            return;
        }
        source.checkNavigationCapacity(resolved.resource);
        // The saved label result cannot qualify an actor's recovered or local Draft.
        const currentTarget = doc?.resource === resolved.resource;
        if (currentTarget && draft?.pending)
            throw new UiError(uiMessage("hostedShell.targetSaveUnverified"));
        const prior = doc;
        let target: Doc, targetDraft: HostedDraft, targetReference: string | undefined;
        if (currentTarget) {
            target = doc!;
            targetDraft = draft!;
        }
        else {
            staged = await request<Doc>("open", { resource: resolved.resource });
            target = staged;
            if (!sameHostedNodeAudience(captured, nodeAudience()))
                return;
            if (target.resource !== resolved.resource)
                throw new UiError(uiMessage("hostedShell.stagedTargetAuthorizedMismatch"));
            targetReference = await readDocumentReference(target);
            if (!sameHostedNodeAudience(captured, nodeAudience()))
                return;
            if (targetReference !== parts.base)
                throw new UiError(uiMessage("hostedShell.stagedTargetAuthorizedMismatch"));
            let local: HostedDraft | undefined;
            try {
                local = cached(target.resource);
            }
            catch {
                throw new UiError(uiMessage("hostedShell.targetRecoveryUnparseable"));
            }
            const retained = source.pausedSession && draft?.reference === parts.base ? { ...draft, source: source.value } : local;
            targetDraft = retained === undefined && target.recoveredDraft == null ? hostedDocumentDraft(target) : nodeTargetDraft(target.source, target.recoveredDraft, target.recoveredDraftBaseSource, retained);
        }
        const retainedAlias = !currentTarget && source.pausedSession && draft?.reference === parts.base;
        if (!currentTarget) {
            targetDraft = bindHostedDraftReference(qualifyHostedDraft(target, targetDraft), targetReference!);
            targetDraft.inputId ??= retainedAlias ? source.authorInputId : crypto.randomUUID();
        }
        const inputId = currentTarget ? source.authorInputId : targetDraft.inputId!;
        const basis: HostedNodeBasis = { sessionId: target.sessionId, source: currentTarget ? source.value : targetDraft.source, serial: currentTarget || retainedAlias ? source.serial : source.retainedSourceSerial(target.resource, targetDraft.source), sourceHash: await nodeSourceHash(currentTarget ? source.value : targetDraft.source) };
        if (!sameHostedNodeAudience(captured, nodeAudience()))
            return;
        await request("editor/map", basis);
        if (!sameHostedNodeAudience(captured, nodeAudience()))
            return;
        const result = await request<HostedNodeFragmentResult>("navigation/fragment", { ...basis, reference });
        if (!sameHostedNodeAudience(captured, nodeAudience()))
            return;
        const origin = nodeFragmentOrigin(result, target.resource, reference, basis);
        if (!origin) {
            const reason = { missing: uiMessage("hostedShell.fragmentLabelMissingInDraft"), ambiguous: uiMessage("hostedShell.fragmentLabelAmbiguousInDraft"), unavailable: uiMessage("hostedShell.fragmentLabelNotExactlyLocatable"), found: uiMessage("hostedShell.fragmentLabelExactRangeMissing") }[result.fragment.status];
            nodeNavigationMessage(reference, displayMessage("hostedShell.fragmentFailureBodyUndoRetained", { p0: uiDisplayParameter(reason) }), true);
            return;
        }
        if (busy) {
            nodeNavigationMessage(reference, uiMessage("hostedShell.fragmentLateRangeNotAdopted"), true);
            return;
        }
        if (!currentTarget) {
            await retain();
            if (!sameHostedNodeAudience(captured, nodeAudience()))
                return;
            installDocument(target, targetDraft, targetReference!, false, false, retainedAlias);
            adopted = true;
            if (prior)
                void closeOwnedSession(prior.sessionId, captured).catch(() => {});
            scheduleCurrentDraft();
        }
        adoptedAudience = nodeAudience();
        source.locateNodeFragment(origin, { sessionId: target.sessionId, source: basis.source, serial: basis.serial, inputId });
        nodeNavigationMessage(reference, displayMessage("hostedShell.fragmentExactlyLocatedInDraft", { p0: uiDisplayParameter(result.fragment.label) }));
        status(uiMessage("hostedShell.fragmentLocatedBodyUnsaved"));
    }
    catch (error) {
        if (sameHostedNodeAudience(adoptedAudience ?? captured, nodeAudience())) {
            const message = displayMessage("hostedShell.fragmentFailureCurrentBodyRetained", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.fragmentLocationIncomplete")) });
            nodeNavigationMessage(reference, message, reference.includes("#"));
            status(message);
        }
    }
    finally {
        // Never send an old actor's temporary session to the newly connected actor.
        if (staged && !adopted)
            await closeOwnedSession(staged.sessionId, captured);
    }
}
/** Read only the staged session. No producer, draft or visible owner is changed. */
async function readDocumentReference(selected: Doc): Promise<string> {
    const result = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: selected.sessionId }, false, 0, AbortSignal.timeout(10000), "stagedTarget");
    if (result.resource !== selected.resource || typeof result.reference !== "string" || !/^weftext-node:\/\//.test(result.reference))
        throw new UiError(uiMessage("hostedShell.noteAuthorizedStableIdentityMissing"));
    return result.reference;
}
function publishDocumentReference(selected: Doc, reference: string): void {
    stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference };
    el<HTMLInputElement>("node-reference").value = reference;
    el("stable-link").hidden = false;
    controls();
}
async function acknowledgeHistory(record: HostedHistoryRecord): Promise<"applied" | "superseded"> {
    const selected = doc, epoch = requestEpoch, audience = sessionAudience();
    if (!selected || !draft || selected.resource !== record.request.resource || !record.status?.result)
        throw new UiError(uiMessage("hostedShell.openOriginalForRestoreReceipt"));
    await source.flush();
    if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>audience[key] !== sessionAudience()[key]))
        throw new UiError(uiMessage("hostedShell.objectChangedRestoreReceiptRetained"));
    const fresh = await request<Doc>("open", { resource: selected.resource, sessionId: selected.sessionId });
    let adopted = false;
    try {
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>audience[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.restoreObjectOrUserChanged"));
        const target = fresh.sessionId === selected.sessionId ? fresh : selected;
        const reference = await readDocumentReference(target);
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>audience[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.restoreObjectOrUserChanged"));
        const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>audience[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.restoreObjectOrUserChanged"));
        const accepted = history.installStatus(record, confirmed);
        if (!accepted.result)
            throw new UiError(uiMessage("hostedShell.restoreResultNoCurrentDeliveryRight"));
        await retain();
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>audience[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.restoreObjectOrUserChanged"));
        source.personalInputReference(target.resource, reference);
        const installed = currentHostedHistoryResult(record, fresh.source, accepted), buffer = { sessionId: selected.sessionId, source: source.value, serial: source.serial, inputId: source.authorInputId };
        const current: HistoryContext = { ...buffer, path: `resource:${selected.resource}`, root: "", baseId: selected.baseId, baseSource: selected.source, dirty: buffer.source !== selected.source, nodeId: record.basis.nodeId, workspaceId: record.basis.workspaceId };
        const replaceClean = installed && fresh.sessionId === selected.sessionId && !fresh.recoveredDraftConflict && restoreBufferDisposition(record.basis, current) === "replaceClean";
        if (replaceClean) source.applyHistorySource(accepted.result.source, buffer);
        const retainedSource = source.value, originalDraftBase = draft.baseSource;
        // Keep the initiating producer. A returned comparison never grants
        // a different writer permission to adopt this editor's later input.
        if (target !== selected) orderedDrafts.set(target, draftOrder(selected));
        doc = target;
        source.open(target.sessionId, target.resource, retainedSource);
        source.disabled = !target.rights.write;
        draft = { ...draft, source: retainedSource, baseSource: replaceClean ? fresh.source : originalDraftBase,
            basis: replaceClean ? { sessionId: target.sessionId, baseId: target.baseId } : draft.basis };
        compareRequired = !replaceClean;
        el("comparison").hidden = replaceClean;
        setUiText(el("current"), replaceClean ? "" : fresh.source);
        setUiText(el("title"), fresh.title);
        adopted = target === fresh;
        retain();
        history.documentChanged();
        controls();
        queries.authorChanged(true);
        knowledgeAuthorChanged();
        void queries.refreshCatalog();
        publishDocumentReference(target, reference);
        if (target.rights.write) await persistHostedDraft(target,retainedSource);
        return replaceClean ? "applied" : "superseded";
    }
    finally {
        if (!adopted && fresh.sessionId !== selected.sessionId) {
            try {
                await closeOwnedSession(fresh.sessionId, audience);
            }
            catch { /* Keep the original receipt even if session cleanup became unavailable. */ }
        }
    }
}
async function finishUnavailableHistory(record: HostedHistoryRecord): Promise<void> {
    const selected = doc, epoch = requestEpoch, captured = sessionAudience();
    if (!selected || !draft || selected.resource !== record.request.resource || !unavailableCommittedHistory(record))
        throw new UiError(uiMessage("hostedShell.openOriginalConfigurationQueryFact"));
    await source.flush();
    if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>captured[key] !== sessionAudience()[key]))
        throw new UiError(uiMessage("hostedShell.objectChangedOriginalFactRetained"));
    const fresh = await request<Doc>("open", { resource: selected.resource });
    let adopted = false;
    try {
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>captured[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.rereadObjectOrUserChanged"));
        const reference = await readDocumentReference(fresh);
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>captured[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.rereadObjectOrUserChanged"));
        const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>captured[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.rereadObjectOrUserChanged"));
        history.installStatus(record, confirmed);
        if (!unavailableCommittedHistory(record))
            throw new UiError(uiMessage("hostedShell.originalDeliveryStatusChanged"));
        await retain();
        if (epoch !== requestEpoch || doc !== selected || !draft || (["authorization","credential","actor","generation","deployment"] as const).some(key=>captured[key] !== sessionAudience()[key]))
            throw new UiError(uiMessage("hostedShell.rereadObjectOrUserChanged"));
        source.personalInputReference(fresh.resource, reference);
        const retainedSource = source.value;
        doc = fresh;
        draft = { ...draft, source: retainedSource };
        source.open(fresh.sessionId, fresh.resource, retainedSource);
        source.disabled = !fresh.rights.write;
        compareRequired = true;
        el("comparison").hidden = false;
        setUiText(el("current"), fresh.source);
        setUiText(el("title"), fresh.title);
        adopted = true;
        retain();
        history.documentChanged();
        knowledgeAuthorChanged();
        controls();
        publishDocumentReference(fresh, reference);
        queueHostedWriterRetirement(selected,fresh);
        if (fresh.rights.write) await persistHostedDraft(fresh,retainedSource);
    }
    finally {
        if (!adopted) {
            try {
                await closeOwnedSession(fresh.sessionId,captured);
            }
            catch { /* Preserve original evidence when cleanup is unavailable. */ }
        }
    }
}
async function openQueryResource(resource: string) {
    const epoch = requestEpoch;
    await source.flush();
    if (epoch !== requestEpoch)
        return;
    await run(() => open(resource));
}
class HostedError extends ApiError {
    constructor(error: ApiError, public http: number) { super(error.code, error.explanation, error.diagnostics, error.details, error.causes, http); }
}
function clearDisplay(message: UiText, clearToken = false) {
    let captureFailure: unknown;
    try {
        hostedCapture?.retireAuthorization();
    }
    catch (error) {
        captureFailure = error;
    }
    try {
        hostedCapture?.dispose();
    }
    catch (error) {
        captureFailure ??= error;
    }
    hostedCapture = undefined;
    captureVersion++;
    capturePanel.replaceChildren();
    capturePanel.hidden = true;
    interfaceLocale?.dispose();
    interfaceLocale = undefined;
    let personalFailure = false;
    try {
        persistPersonalInputs();
    }
    catch {
        personalFailure = true;
    }
    verifiedPersonalAudience = "";
    retirePersonalCopy();
    personalInputList.replaceChildren();
    el("personal-inputs").replaceChildren();
    el("personal-inputs").hidden = true;
    personalControls.clear();
    releaseViewSessions();
    clearTimeout(draftTimer);
    expiredSessionId = undefined;
    everydayNavigation.clear();
    everydayBar.replaceChildren();
    openDocumentTitles.clear();
    openDocumentButtons.clear();
    documentTabs.replaceChildren();
    closeHostedInspector();
    document.querySelectorAll<HTMLDialogElement>(".everyday-dialog").forEach(dialog => { dialog.close(); dialog.remove(); });
    // Logout retires its pending action. A new login may start immediately;
    // the former action's catch/finally cannot change the new account's UI.
    if (clearToken) {
        actionVersion++;
        busy = false;
    }
    retiredStructureSessions.clear();
    retiredStructureSources.clear();
    structureDocuments = [];
    clearNodeNavigation();
    requestEpoch++;
    authorizationEpoch++;
    doc = undefined;
    draft = undefined;
    compareRequired = false;
    cachePrefix = "";
    stableReference = undefined;
    draftCache.clearMemory();
    const clearErrors: unknown[] = captureFailure ? [captureFailure] : [];
    // A blocked storage provider must never stop the remaining protected views
    // from disposing their in-memory capabilities and visible content.
    for (const clear of [() => latexAuthoring.clear(), () => diagramAuthoring.clear(), () => source.clear(), () => knowledgeLinks.clear(), () => citations.clear(), () => navigationRegistry.clear(), () => contentImport.clear(), () => recognitionPanel.revoke(), () => ownerBackup.clear(), () => fields.clear(), () => review.clear(), () => queries.clear(), () => office.clear(), () => history.clear(), () => library.clear(), () => media.clear(), () => structures.clear()]) {
        try {
            clear();
        }
        catch (error) {
            clearErrors.push(error);
        }
    }
    el("documents").replaceChildren();
    setUiText(el("title"), uiMessage("hostedShell.selectDocument"));
    setUiText(el("current"), "");
    el("stable-link").hidden = true;
    el<HTMLInputElement>("node-reference").value = "";
    el<HTMLIFrameElement>("render").srcdoc = "";
    el("render").hidden = true;
    el("comparison").hidden = true;
    el("history-list").hidden = true;
    const remove = (key: string) => {
        try {
            sessionStorage.removeItem(key);
        }
        catch (error) {
            clearErrors.push(error);
        }
    };
    try {
        for (const key of Object.keys(sessionStorage))
            if (key.startsWith("weftext-hosted-draft:") || key.startsWith("weftext-hosted-draft-text:"))
                remove(key);
    }
    catch (error) {
        clearErrors.push(error);
    }
    if (clearToken) {
        token = "";
        identity = undefined;
        remove("weftext-hosted-token");
        remove("weftext-hosted-identity");
        el("workspace").hidden = true;
        el("login").hidden = false;
        el("logout").hidden = true;
    }
    controls();
    locale.set("zh-CN");
    mountInterfaceLocale();
    status(() => uiTextValue(clearErrors.length ? displayMessage("hostedShell.protectedDisplayClearStorageFailure", { p0: uiDisplayParameter(message) }) : message) + uiTextValue(personalFailure ? uiMessage("hostedShell.personalInputMemoryAfterClearSuffix") : ""));
}
async function request<T>(path: string, data?: unknown, bytes = false, attempt = 0, signal?: AbortSignal, failureScope?: "stagedTarget"): Promise<T> {
    const epoch = requestEpoch, credential = token;
    const target = data as {
        sessionId?: string;
    } | undefined, installed = doc;
    if (path === "draft" && installed && installed.sessionId === target?.sessionId && recoveryContinuations.has(installed)) {
        let pending = recoveryContinuations.get(installed);
        if (!pending) {
            pending = request("draft/resume", { sessionId: installed.sessionId, recordWitness: installed.recoveredDraftWitness, recoveredSource: installed.recoveredDraft }).then(() => undefined).catch(error => {
                if (!(error instanceof HostedError) || !["draftChanged", "draftProducerActive", "draftProducerUnavailable"].includes(error.code))
                    throw error;
            });
            recoveryContinuations.set(installed, pending);
        }
        try {
            await pending;
            recoveryContinuations.delete(installed);
        }
        catch (error) {
            recoveryContinuations.set(installed, null);
            throw error;
        }
    }
    if (epoch !== requestEpoch || credential !== token)
        throw new UiError(uiMessage("hostedShell.authorizationChangedRefreshDocuments"));
    const audience = () => ({ epoch: authorizationEpoch, credential: token, actor: identity?.actor, deployment: identity?.deployment, generation: identity?.generation, sessionId: doc?.sessionId, queryVersion: queries.version }), captured = audience();
    const response = await fetch(`/api/hosted/${path}`, { method: data === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` }, body: data === undefined ? undefined : JSON.stringify(data), cache: "no-store", signal });
    if (!response.ok) {
        const result = await readStrictJsonResponse(response) as {
            error?: {
                code: string;
                explanation: unknown;
            };
        };
        const requestTarget = data as {
            sessionId?: string;
            resource?: string;
        } | undefined, retired = !!requestTarget && (typeof requestTarget.sessionId === "string" && retiredStructureSessions.has(requestTarget.sessionId) || typeof requestTarget.resource === "string" && retiredStructureSources.has(requestTarget.resource));
        // Only a read-only preview can wait for the shared compiler. Author save
        // requests keep their explicit original-operation recovery path.
        if (path === "preview" && response.status === 429 && attempt < 5) {
            await new Promise(resolve => setTimeout(resolve, 500));
            if (epoch === requestEpoch && credential === token)
                return request(path, data, bytes, attempt + 1, signal);
        }
        if (response.status === 401 && sameHostedQueryIdentity(captured, audience()))
            clearDisplay(uiMessage("hostedShell.credentialsInvalidCacheCleared"), true);
        if ([404, 409, 410].includes(response.status) && retired && sameHostedQueryIdentity(captured, audience())) {
            try {
                const current = await request<Me>("me");
                if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) {
                    clearDisplay(uiMessage("hostedShell.authorizationChangedViewsCacheCleared"));
                    await confirmIdentity(current);
                }
            }
            catch { /* A retired structural coordinate never clears a newer buffer or actor. */ }
        }
        else if (response.status === 404 && path === "query" && sameHostedQueryIdentity(captured, audience())) {
            try {
                const current = await request<Me>("me");
                if (sameHostedQueryIdentity(captured, audience())) {
                    if (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation) {
                        clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshDocuments"));
                        await confirmIdentity(current);
                    }
                    else if (sameHostedQueryAudience(captured, audience()))
                        queries.refuseResult(uiMessage("hostedShell.queryObjectOrScopeUnavailable"));
                }
            }
            catch {
                if (sameHostedQueryAudience(captured, audience()))
                    queries.refuseResult(uiMessage("hostedShell.queryEligibilityUnverified"));
            }
        }
        else if ([404, 410].includes(response.status) && path.startsWith("media/") && sameHostedQueryIdentity(captured, audience())) {
            // A hidden/unknown asset is not a revocation of the independently open
            // note. Only a current identity's changed actor epoch clears all views.
            try {
                const current = await request<Me>("me");
                if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) {
                    clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshMedia"));
                    await confirmIdentity(current);
                }
            }
            catch { /* Caller stops this resource delivery; personal source remains protected. */ }
        }
        else if ([404, 409, 410].includes(response.status) && (failureScope === "stagedTarget" || path === "office/legacy-capabilities" || path === "open" || path.startsWith("navigation/") || path.startsWith("structure/") || path.startsWith("knowledge-links/") || path.startsWith("citations/") || path.startsWith("export/") || path.startsWith("import/") || path.startsWith("owner-backup/") || path.startsWith("recognition/")) && sameHostedQueryIdentity(captured, audience())) {
            try {
                const current = await request<Me>("me");
                if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) {
                    clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshLinks"));
                    await confirmIdentity(current);
                }
            }
            catch { /* Stop this link delivery without treating a hidden same-epoch target as source revocation. */ }
        }
        else if (response.status === 404 && failureScope !== "stagedTarget" && !retired && path !== "query" && path !== "open" && !path.startsWith("navigation/") && !path.startsWith("structure/") && !path.startsWith("knowledge-links/") && !path.startsWith("citations/") && !path.startsWith("export/") && !path.startsWith("import/") && !path.startsWith("owner-backup/") && !path.startsWith("media/") && epoch === requestEpoch && credential === token)
            clearDisplay(uiMessage("hostedShell.objectAuthorizationChangedRefreshDocuments"));
        throw new HostedError(decodeApiError(result.error) ?? new ApiError("invalidResponse", uiMessage("error.invalidResponse")), response.status);
    }
    return (bytes ? response.arrayBuffer() : readStrictJsonResponse(response)) as Promise<T>;
}
function controls() {
    renderEverydayBar();
    renderHostedDocumentTabs();
    syncHostedPanels();
    for (const button of el("documents").querySelectorAll<HTMLButtonElement>("button"))
        button.disabled = busy;
    try {
        const audience = personalAudience();
        el("personal-input-recovery").hidden = !audience || !personalInputs.list(audience).length;
    }
    catch {
        el("personal-input-recovery").hidden = !personalAudience();
    }
    for (const id of ["latex", "diagram"])
        el<HTMLButtonElement>(id).disabled = !authoringEditable();
    el<HTMLButtonElement>("save").disabled = busy || !doc?.rights.write || source.pausedSession || compareRequired || !!draft?.pending || !!doc && (history.hasPending(doc.resource) || unresolvedPersonalSave(doc));
    for (const id of ["preview", "reopen"])
        el<HTMLButtonElement>(id).disabled = busy || !doc || source.pausedSession && id === "preview";
    el<HTMLButtonElement>("download").disabled = busy || !doc?.rights.download || source.pausedSession;
    el<HTMLButtonElement>("history").disabled = busy || !doc?.rights.history || source.pausedSession;
    el<HTMLButtonElement>("rebase").disabled = busy || !doc?.rights.write || source.pausedSession || !!draft?.pending;
    el<HTMLButtonElement>("copy-reference").disabled = busy || !stableReference || stableReference.sessionId !== doc?.sessionId;
}
function retain(): Promise<BrowserProposalAck | undefined> {
    if (!doc || !draft)
        return Promise.resolve(undefined);
    draft.source = source.value;
    draft.inputId = source.authorInputId;
    // Storage failure is visible and prevents a save request that could lose an
    // unknown operation receipt on reload.
    const write = persistPersonalInputs();
    draftCache.write(doc.resource, draft);
    return write;
}
async function confirmIdentity(next: Me): Promise<void> {
    identity = next;
    verifiedPersonalAudience = "";
    const epoch = authorizationEpoch, credential = token, audience = JSON.stringify([next.deployment, next.actor]);
    try {
        await personalInputs.open(audience);
    }
    catch (error) {
        if (identity === next && epoch === authorizationEpoch && credential === token)
            status(errorUiText(error));
    }
    if (identity !== next || epoch !== authorizationEpoch || credential !== token)
        return;
    verifiedPersonalAudience = audience;
    sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(next));
    controls();
}
function personalAudience(): string | null { const current = identity ? JSON.stringify([identity.deployment, identity.actor]) : ""; return current && current === verifiedPersonalAudience ? current : null; }
/** A known route is not a terminal operation result. A fresh session identity
 * can exclude a separately proved node, never guess an alias. */
function unresolvedPersonalSave(selected: Doc, preparedOperationId?: string): boolean {
    const audience = personalAudience();
    if (!audience)
        return false;
    const reference = stableReference?.resource === selected.resource && stableReference.sessionId === selected.sessionId ? stableReference.reference : undefined;
    return personalInputs.list(audience).some(record => {
        const pending = record.pending;
        if (!pending || pending.status !== "unresolved")
            return false;
        // This exact newly prepared request has not been sent yet. Every older
        // operation remains a barrier, including one recovered after logout.
        if (pending.operationId === preparedOperationId && pending.route === "known" && pending.resource === selected.resource)
            return false;
        return record.owner.kind !== "body" || !record.owner.reference || !reference || record.owner.resource === selected.resource || record.owner.reference === reference || pending.route === "known" && pending.resource === selected.resource;
    });
}
function observePersonalWrite(write: Promise<BrowserProposalAck>, audience: string): Promise<BrowserProposalAck> {
    void write.catch(error => {
        if (personalAudience() === audience) {
            status(errorUiText(error));
            controls();
        }
    });
    return write;
}
function persistPersonalInputs(): Promise<BrowserProposalAck | undefined> {
    const audience = personalAudience();
    if (!audience)
        return Promise.resolve(undefined);
    const updates = source.personalInputRecords();
    return observePersonalWrite(personalInputs.update(audience, updates), audience);
}
async function qualifyPersonalOriginalRoute(proof: PersonalOriginalRouteProof): Promise<void> {
    const audience = personalAudience(), epoch = authorizationEpoch, credential = token;
    if (!audience || proof.audience !== audience)
        throw new UiError(uiMessage("hostedShell.reauthenticateOriginalAccount"));
    const operationId = proof.kind === "hostedDraft" ? proof.pending.operationId : proof.receipt.operationId;
    const original = personalInputs.list(audience).find(record => record.pending?.operationId === operationId && record.pending.route === "unknownOriginalRoute");
    if (!original)
        return;
    await observePersonalWrite(personalInputs.proveOriginalRoute(audience, original, proof), audience);
    if (personalAudience() !== audience || epoch !== authorizationEpoch || credential !== token)
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
}
async function finishPersonalOperation(operationId: string, terminal: "committed" | "cancelled" | "resolved", receipt: SaveStatus): Promise<void> {
    const audience = personalAudience(), epoch = authorizationEpoch, credential = token;
    if (!audience || typeof receipt.resource !== "string" || receipt.status !== terminal)
        throw new UiError(uiMessage("hostedShell.saveReceiptBasisMismatch"));
    checkedSaveStatus(receipt, operationId, receipt.resource);
    await qualifyPersonalOriginalRoute({ kind: "coreReceipt", audience, receipt: { operationId, resource: receipt.resource, status: terminal } });
    if (personalAudience() !== audience || epoch !== authorizationEpoch || credential !== token)
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
    source.settlePersonalSave(operationId, terminal === "committed");
    await Promise.all(personalInputs.list(audience).filter(record => record.pending?.operationId === operationId).map(record => observePersonalWrite(personalInputs.settle(audience, record, terminal), audience)));
}
function showPersonalInputs(): void {
    const audience = personalAudience();
    if (!audience)
        throw new UiError(uiMessage("hostedShell.reauthenticateOriginalAccount"));
    const panel = el("personal-inputs");
    panel.hidden = false;
    if (personalInputList.parentElement !== panel)
        panel.replaceChildren(personalInputList, personalCopyMount);
    renderPersonalInputs(personalInputList, personalInputs, audience, {
        restore: record => restorePersonalInput(record, audience), operation: record => recoverPersonalOperation(record, audience),
        editCopy: record => editPersonalCopy(record, audience),
        editingCopy: record => personalCopyEditor?.audience === audience && personalCopyEditor.record.id === record.id,
        producers: () => personalControls.inspect().producers.filter(producer => producer.audience === audience),
        discardProducer: producer => discardRetiredPersonalProducer(producer, audience),
        changed: () => {
            if (personalAudience() === audience) {
                controls();
                showPersonalInputs();
            }
        },
    });
}
function personalCopyCurrent(editor: HostedPersonalCopy | undefined): editor is HostedPersonalCopy {
    return !!editor && personalCopyEditor === editor && editor.version === personalCopyVersion
        && editor.audience === personalAudience() && editor.epoch === authorizationEpoch && editor.credential === token;
}
function retirePersonalCopy(): void {
    personalCopyVersion++;
    const editor = personalCopyEditor;
    personalCopyEditor = undefined;
    editor?.session.retire();
    personalCopyMount.replaceChildren();
}
async function editPersonalCopy(record: PersonalInputRecord, audience: string): Promise<void> {
    if (personalCopyEditor) {
        if (personalCopyCurrent(personalCopyEditor) && (personalCopyEditor.record.id === record.id || personalCopyEditor.originalId === record.id)) {
            personalCopyEditor.textarea.focus();
            return;
        }
        throw new UiError(uiMessage("personalInput.copyActiveFinishFirst"));
    }
    record = personalInputs.current(audience, record);
    const epoch = authorizationEpoch, credential = token, version = ++personalCopyVersion, originalId = record.id;
    const current = () => version === personalCopyVersion && epoch === authorizationEpoch && credential === token && personalAudience() === audience && !personalCopyEditor;
    await verifyPersonalIdentity(audience, epoch);
    if (!current())
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
    record = personalInputs.current(audience, record);
    let copy: PersonalInputRecord;
    if (isPersonalCopy(record)) {
        // Reopening this personal target is also an explicit retry of its own
        // RAM cut after a refused browser write; seeding still requires ACK.
        await observePersonalWrite(personalInputs.retry(audience), audience);
        copy = personalInputs.current(audience, record);
    }
    else
        copy = await personalInputs.createPersonalCopy(audience, record);
    if (!current())
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
    const seed = personalInputs.personalCopySeed(audience, copy);
    const title = document.createElement("h3"), textarea = document.createElement("textarea"), diagnostic = document.createElement("p"), finish = document.createElement("button");
    setUiText(title, uiMessage("personalInput.copyEditorTitle"));
    textarea.value = personalInputText(copy);
    textarea.rows = 5;
    setUiAttr(textarea, "aria-label", uiMessage("personalInput.copyEditorLabel"));
    diagnostic.setAttribute("role", "status");
    finish.type = "button";
    setUiText(finish, uiMessage("personalInput.finishCopy"));
    let editor: HostedPersonalCopy;
    const session = personalControls.session(audience, epoch, copy.owner, () => personalCopyCurrent(editor), input => {
        if (!personalCopyCurrent(editor))
            return;
        if (input.kind !== "text" || input.id !== editor.record.id || input.context.slot !== PERSONAL_COPY_ONLY_SLOT || input.context.path.length !== 0 || JSON.stringify(input.context.owner) !== JSON.stringify(editor.record.owner))
            throw new UiError(uiMessage("personalInput.copyChanged"));
        try {
            const previous = personalInputs.current(audience, editor.record);
            const write = personalInputs.update(audience, [{ id: previous.id, owner: previous.owner, slot: PERSONAL_COPY_ONLY_SLOT,
                producer: { producerId: input.producerId, revision: input.producerRevision },
                origin: { baseLength: 0, pieces: [], detached: [], copyOnly: [{ id: input.id, revision: input.revision, text: input.text }], revision: input.revision } }]);
            editor.record = personalInputs.current(audience, { ...previous, sequence: previous.sequence + 1 });
            editor.retentionFailure = false;
            editor.lastWrite = observePersonalWrite(write, audience);
            void write.catch(error => { if (personalCopyCurrent(editor)) setUiText(editor.status, errorUiText(error)); });
            controls();
        }
        catch (error) {
            editor.retentionFailure = true;
            setUiText(editor.status, errorUiText(error));
        }
    }, () => editor.record.sequence);
    editor = { audience, epoch, credential, version, originalId, record: copy, session, textarea, status: diagnostic, finish, finishing: false };
    personalCopyEditor = editor;
    personalCopyMount.replaceChildren(title, textarea, finish, diagnostic);
    try {
        session.bind(textarea, PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed });
    }
    catch (error) {
        retirePersonalCopy();
        throw error;
    }
    finish.onclick = () => { void finishPersonalCopy(editor).catch(error => { if (personalCopyCurrent(editor)) setUiText(editor.status, errorUiText(error)); }); };
    textarea.focus();
}
async function finishPersonalCopy(editor: HostedPersonalCopy): Promise<void> {
    if (!personalCopyCurrent(editor) || editor.finishing)
        throw new UiError(uiMessage("personalInput.copyChanged"));
    await verifyPersonalIdentity(editor.audience, editor.epoch);
    if (!personalCopyCurrent(editor))
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
    const record = personalInputs.current(editor.audience, editor.record), checkpoint = editor.session.inspect();
    if (!isPersonalCopy(record) || editor.retentionFailure || !editor.session.freeze(checkpoint))
        throw new UiError(uiMessage("personalInput.copyChanged"));
    editor.finishing = true;
    editor.finish.disabled = true;
    try {
        // Finish explicitly accepts this whole personal-copy cut. A transport
        // ACK on ordinary input never acknowledges this or the original owner.
        await observePersonalWrite(personalInputs.retry(editor.audience), editor.audience);
        if (!personalCopyCurrent(editor) || editor.session.inspect().revision !== checkpoint.revision || JSON.stringify(personalInputs.current(editor.audience, record)) !== JSON.stringify(record))
            throw new UiError(uiMessage("personalInput.copyChanged"));
        if (!editor.session.acknowledge(checkpoint, "applied"))
            throw new UiError(uiMessage("personalInput.copyChanged"));
        retirePersonalCopy();
        showPersonalInputs();
        status(uiMessage("personalInput.copyFinished"));
    }
    catch (error) {
        if (personalCopyCurrent(editor)) {
            editor.session.thaw();
            editor.finishing = false;
            editor.finish.disabled = false;
        }
        throw error;
    }
}
async function discardRetiredPersonalProducer(producer: AuthorProducerState, audience: string): Promise<void> {
    const epoch = authorizationEpoch;
    await verifyPersonalIdentity(audience, epoch);
    const current = () => personalControls.inspect().producers.find(item => item.producerId === producer.producerId);
    if (producer.audience !== audience || !producer.retired || JSON.stringify(current()) !== JSON.stringify(producer))
        throw new Error("作者表单已变化，未放弃另一代输入。");
    const records = personalInputs.list(audience).filter(record => record.producer?.producerId === producer.producerId);
    if (records.some(record => record.pending?.status === "unresolved"))
        throw new Error("原保存结果尚未核定，先查询同一编号；未放弃其输入。");
    if (personalCopyEditor?.audience === audience && records.some(record => record.id === personalCopyEditor!.record.id))
        throw new UiError(uiMessage("personalInput.copyActiveFinishFirst"));
    await Promise.all(records.map(record => observePersonalWrite(personalInputs.remove(audience, record), audience)));
    if (authorizationEpoch !== epoch || personalAudience() !== audience || JSON.stringify(current()) !== JSON.stringify(producer))
        throw new Error("处置期间作者身份已变化，未放弃新的输入。");
    if (!personalControls.acknowledge(producer, "discarded"))
        throw new Error("原退休表单已变化，未采用较早的处置。");
}
async function verifyPersonalIdentity(audience: string, epoch: number): Promise<void> {
    const next = await request<Me>("me");
    if (authorizationEpoch !== epoch || personalAudience() !== audience)
        throw new UiError(uiMessage("hostedShell.recoveryAccountOrAuthorizationChanged"));
    if (!identity || next.actor !== identity.actor || next.deployment !== identity.deployment || next.generation !== identity.generation) {
        authorizationsChanged(uiMessage("hostedShell.authorizationChangedNewInputRetained"), next);
        throw new UiError(uiMessage("hostedShell.reopenOriginalUnderNewAuthorization"));
    }
}
async function recoverPersonalOperation(record: PersonalInputRecord, audience: string): Promise<void> {
    const epoch = authorizationEpoch;
    await verifyPersonalIdentity(audience, epoch);
    record = personalInputs.current(audience, record);
    if (!record.pending || record.owner.kind !== "body")
        throw new UiError(uiMessage("hostedShell.bodySaveOperationMissing"));
    if (record.pending.route !== "known")
        throw new UiError(uiMessage("personalInput.unknownOriginalRoute"));
    const resource = record.pending.resource;
    let outcome = await request<SaveStatus>("operation", { resource, operationId: record.pending.operationId });
    if (authorizationEpoch !== epoch || personalAudience() !== audience)
        return;
    checkedSaveStatus(outcome, record.pending.operationId, resource);
    if (["notRegistered", "notCommitted"].includes(outcome.status))
        outcome = await request<SaveStatus>("operation/settle", { resource, operationId: record.pending.operationId });
    if (authorizationEpoch !== epoch || personalAudience() !== audience)
        return;
    personalInputs.current(audience, record);
    checkedSaveStatus(outcome, record.pending.operationId, resource);
    if (!["committed", "cancelled", "resolved"].includes(outcome.status))
        throw new UiError(uiMessage("hostedShell.originalSaveUnknownIdRetained"));
    await finishPersonalOperation(record.pending.operationId, outcome.status as "committed" | "cancelled" | "resolved", outcome);
}
async function restorePersonalInput(record: PersonalInputRecord, audience: string): Promise<void> {
    record = personalInputs.current(audience, record);
    if (isPersonalCopy(record))
        throw new UiError(uiMessage("personalInput.copyOnlyCannotRestore"));
    const selected = doc, epoch = authorizationEpoch;
    if (!selected || !draft || !selected.rights.write || compareRequired || draft.pending)
        throw new UiError(uiMessage("hostedShell.openOriginalCheckDraftAndOperation"));
    await source.flush();
    await verifyPersonalIdentity(audience, epoch);
    record = personalInputs.current(audience, record);
    if (isPersonalCopy(record))
        throw new UiError(uiMessage("personalInput.copyOnlyCannotRestore"));
    if (record.owner.kind !== "body")
        throw new UiError(uiMessage("hostedShell.formCopyOnly"));
    const owner = record.owner;
    if (doc !== selected || !draft || source.value !== selected.source || draft.source !== selected.source)
        throw new UiError(uiMessage("hostedShell.existingDraftInputNotOverwritten"));
    if (personalInputs.list(audience).some(other => other.pending?.status === "unresolved" && other.owner.kind === "body" && (other.owner.reference && other.owner.reference === owner.reference || other.owner.resource === owner.resource)))
        throw new UiError(uiMessage("hostedShell.originalSaveCheckSameId"));
    const basis = { sessionId: selected.sessionId, source: source.value, serial: source.serial, inputId: source.authorInputId };
    const reference = await request<{
        resource: string;
        reference: string;
    }>("navigation/ref", { sessionId: selected.sessionId });
    const hash = await nodeSourceHash(basis.source);
    if (authorizationEpoch !== epoch || personalAudience() !== audience || doc !== selected || source.value !== basis.source || source.serial !== basis.serial || source.authorInputId !== basis.inputId)
        throw new UiError(uiMessage("hostedShell.recoveryCheckNewInputOrObjectChanged"));
    personalInputs.current(audience, record);
    if (!owner.reference || reference.reference !== owner.reference || reference.resource !== selected.resource || !record.baseHash || hash !== record.baseHash)
        throw new UiError(uiMessage("hostedShell.recoveryTargetOrBaselineMismatch"));
    const restored = replayPersonalInput(record.origin, basis.source);
    source.restorePersonalInput(restored, record.origin, record.baseHash, basis);
    await retain();
    status(uiMessage("hostedShell.personalInputRestoredAsUnsavedDraft"));
}
function cached(resource: string): HostedDraft | undefined {
    return draftCache.read(resource);
}
async function connect() {
    const epoch = requestEpoch, credential = token;
    const previous = identity ?? JSON.parse(sessionStorage.getItem("weftext-hosted-identity") ?? "null") as Me | undefined;
    const next = await request<Me>("me");
    if (epoch !== requestEpoch || credential !== token)
        return;
    if (previous && (next.actor !== previous.actor || next.deployment !== previous.deployment || next.generation !== previous.generation))
        clearDisplay(uiMessage("hostedShell.authorizationUpdatedReopen"));
    await confirmIdentity(next);
    if (identity !== next || credential !== token || !personalAudience())
        return;
    mountInterfaceLocale(next);
    cachePrefix = hostedCachePrefix("draft", next.deployment, next.actor, next.generation);
    sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(next));
    sessionStorage.setItem("weftext-hosted-token", token);
    el("login").hidden = true;
    el("workspace").hidden = false;
    el("logout").hidden = false;
    controls();
    status(displayMessage("hostedShell.connectedIdentity", { p0: uiDisplayParameter(next.actor), p1: uiDisplayParameter(next.version) }));
    await connectPanels(next, requestEpoch, credential);
}
async function connectPanels(selected: Me, epoch: number, credential: string): Promise<void> {
    const current = () => requestEpoch === epoch && token === credential && identity === selected;
    const failures = await connectHostedPanels(current, [
        { label: uiMessage("hostedShell.panelDocumentDirectory"), connect: () => list() },
        { label: uiMessage("hostedShell.panelProperties"), connect: () => fields.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelAnnotations"), connect: () => review.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelQueries"), connect: () => queries.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelOfficeImportOutput"), connect: () => office.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("shell.timeline"), connect: () => history.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelLibraryConfiguration"), connect: () => library.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelMedia"), connect: () => media.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelStructure"), connect: () => structures.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelKnowledgeLinks"), connect: () => { structures.setDocuments(structureDocuments); knowledgeLinks.connect(); } },
        { label: uiMessage("hostedShell.panelCitations"), connect: () => citations.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelTags"), connect: () => navigationRegistry.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("shell.contentImport"), connect: () => contentImport.connect(selected.deployment, selected.actor, selected.generation) },
        { label: uiMessage("hostedShell.panelRecoveryMaterials"), connect: () => ownerBackup.connect(selected.deployment, selected.actor, selected.generation) },
    ]);
    if (failures && current())
        status(failures.length
            ? displayMessage("hostedShell.panelsNotReadyRetry", { p0: uiDisplayParameter(joinUiText(failures.map(item => displayMessage("hostedShell.panelNotReady", { p0: uiDisplayParameter(item.label), p1: uiDisplayParameter(item.message) })), "；")) }) : displayMessage("hostedShell.connectedIdentity", { p0: uiDisplayParameter(selected.actor), p1: uiDisplayParameter(selected.version) }));
}
async function list(term?: string) {
    const epoch = requestEpoch;
    const rows = term ? (await request<{
        rows: {
            resource: string;
            title: string;
        }[];
        complete: boolean;
    }>("search", { text: term })) : undefined;
    const items = rows?.rows ?? (await request<{
        documents: {
            resource: string;
            title: string;
        }[];
    }>("documents")).documents;
    if (epoch !== requestEpoch)
        return;
    structureDocuments = items;
    structures.setDocuments(items);
    el("documents").replaceChildren(...items.map(item => { const button = document.createElement("button"); setUiText(button, item.title); button.disabled = busy; button.onclick = () => run(() => open(item.resource)); return button; }));
    el("documents").querySelectorAll<HTMLButtonElement>("button").forEach((button, index) => {
        const item = items[index];
        const menu = () => openEverydayMenu(displayMessage("shell.entryActions", { p0: uiDisplayParameter(item.title) }), [{ id: "node-open", label: uiMessage("graph.openNote"), run: () => run(() => open(item.resource)) }], status, false);
        button.oncontextmenu = event => { event.preventDefault(); button.focus(); menu(); };
        button.onkeydown = event => {
            if (!event.isComposing && (event.key === "ContextMenu" || event.shiftKey && event.key === "F10")) {
                event.preventDefault();
                menu();
            }
        };
    });
    if (rows)
        status(rows.complete ? displayMessage("hostedShell.accessibleSearchCount", { p0: items.length }) : displayMessage("hostedShell.searchIncompleteCount", { p0: items.length }));
}
async function open(resource: string) {
    source.checkNavigationCapacity(resource);
    clearNodeNavigation();
    const prior = doc, captured = nodeAudience();
    const current = () => doc === prior && sameHostedNodeAudience(captured, nodeAudience());
    const sameIdentity = () => { const now = sessionAudience(); return (["authorization", "credential", "actor", "generation", "deployment"] as const).every(key => captured[key] === now[key]); };
    const authority = async () => {
        if (!sameIdentity())
            return false;
        const next = await request<Me>("me");
        if (!sameIdentity())
            return false;
        if (next.actor !== captured.actor || next.deployment !== captured.deployment || next.generation !== captured.generation) {
            authorizationsChanged(uiMessage("hostedShell.authorizationUpdatedReselectDocument"), next);
            return false;
        }
        return true;
    };
    await source.flush();
    if (!current())
        return;
    retain();
    status(uiMessage("hostedShell.verifyingTargetKeepDraftUndo"));
    const currentIdentity = await request<Me>("me");
    if (!sameIdentity())
        return;
    if (identity && (currentIdentity.generation !== identity.generation || currentIdentity.actor !== identity.actor || currentIdentity.deployment !== identity.deployment)) {
        authorizationsChanged(uiMessage("hostedShell.authorizationUpdatedReselectDocument"), currentIdentity);
        await list();
        return;
    }
    if (!current())
        return;
    let opened: Doc | undefined, adopted = false;
    try {
        opened = await request<Doc>("open", { resource });
        if (!await authority() || !current())
            return;
        if (opened.resource !== resource)
            throw new UiError(uiMessage("hostedShell.openReceiptTargetMismatch"));
        const reference = await readDocumentReference(opened);
        if (!await authority() || !current())
            return;
        let targetDraft = cached(resource) ?? hostedDocumentDraft(opened);
        let alias = false, recoveredSave = false;
        if (prior && draft?.reference && source.pausedSession && prior.resource !== resource) {
            if (reference === draft.reference) {
                targetDraft = { ...draft, source: source.value };
                alias = true;
            }
        }
        if (targetDraft.pending) {
            const originalResource = targetDraft.pending.resource;
            const audience = personalAudience();
            if (targetDraft.pending.request && audience) {
                await qualifyPersonalOriginalRoute({ kind: "hostedDraft", audience, pending: { operationId: targetDraft.pending.operationId, resource: originalResource, request: targetDraft.pending.request } });
                if (!current())
                    return;
            }
            let outcome = await request<SaveStatus>("operation", { resource: originalResource, operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget");
            if (!current())
                return;
            checkedSaveStatus(outcome, targetDraft.pending.operationId, originalResource);
            if (["notRegistered", "notCommitted"].includes(outcome.status))
                outcome = await request<SaveStatus>("operation/settle", { resource: originalResource, operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget");
            if (!current())
                return;
            checkedSaveStatus(outcome, targetDraft.pending.operationId, originalResource);
            if (outcome.status === "committed") {
                await finishPersonalOperation(targetDraft.pending.operationId, "committed", outcome);
                targetDraft = applySaveResult(targetDraft, targetDraft.pending.source, null);
                recoveredSave = true;
            }
            else if (["cancelled", "resolved"].includes(outcome.status)) {
                await finishPersonalOperation(targetDraft.pending.operationId, outcome.status as "cancelled" | "resolved", outcome);
                delete targetDraft.pending;
            }
        }
        if (alias && targetDraft.pending)
            throw new Error("原别名保存尚未终结；原编号、resource、输入和 Undo 保留，没有将待核对提案重绑到新别名。");
        if (!await authority() || !current())
            return;
        await retain();
        if (!current())
            return;
        targetDraft = bindHostedDraftReference(hostedOpenedDraft(opened.source, qualifyHostedDraft(opened, targetDraft)), reference);
        // A quota failure must happen while the original editor and session still
        // exist. This exact prepared cache is not written again during install.
        try {
            draftCache.write(opened.resource, targetDraft);
        }
        catch {
            throw new UiError(uiMessage("hostedShell.targetRecoveryNotDurableNoSwitch"));
        }
        // Install only after every target qualification/recovery succeeds. Closing
        // the former session uses its captured actor and cannot clear the new view.
        installDocument(opened, targetDraft, reference, recoveredSave, true, alias);
        adopted = true;
        if (alias)
            scheduleCurrentDraft();
        if (prior)
            await closeOwnedSession(prior.sessionId, captured);
    }
    finally {
        if (opened && !adopted)
            await closeOwnedSession(opened.sessionId, captured);
    }
}
function installDocument(opened: Doc, retainedDraft: HostedDraft, reference: string, recoveredSave = false, stagedCache = false, rebindAlias = false) {
    if (!rebindAlias)
        source.checkNavigationCapacity(opened.resource);
    const qualified = bindHostedDraftReference(qualifyHostedDraft(opened, retainedDraft), reference);
    source.personalInputReference(opened.resource, reference);
    if (rebindAlias)
        source.rebindResourceAlias(opened.resource);
    clearNodeNavigation();
    requestEpoch++;
    doc = opened;
    draft = stagedCache ? qualified : hostedOpenedDraft(opened.source, qualified);
    installingDocument = true;
    if (opened.recoveredDraft != null && opened.recoveredDraftWitness && draft.source === opened.recoveredDraft)
        recoveryContinuations.set(opened, null);
    try {
        source.open(opened.sessionId, opened.resource, draft.source, draft.inputId);
        source.disabled = !opened.rights.write;
    }
    finally {
        installingDocument = false;
    }
    setUiText(el("title"), opened.title);
    compareRequired = opened.recoveredDraftConflict || draftDisposition(opened.source, draft) === "compare";
    el("comparison").hidden = !compareRequired;
    setUiText(el("current"), compareRequired ? opened.source : "");
    history.documentChanged();
    el<HTMLIFrameElement>("render").srcdoc = "";
    el("render").hidden = true;
    if (!stagedCache)
        retain();
    controls();
    queries.authorChanged(true);
    knowledgeAuthorChanged();
    publishDocumentReference(opened, reference);
    if (recoveredSave)
        void queries.refreshCatalog();
    status(draft.pending ? uiMessage("hostedShell.originalSaveUnknownCannotSaveAgain") : compareRequired ? uiMessage("hostedShell.bothDraftsRetainedCompareMerge") : draft.source !== opened.source ? uiMessage("hostedShell.personalDraftRecoveredBodyUnsaved") : uiMessage("hostedShell.currentDraftRead"));
}
document.addEventListener("keydown", event => {
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || source.composing || !identity || busy || event.target instanceof Element && event.target.closest("dialog"))
        return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "p") {
        event.preventDefault();
        if (event.shiftKey)
            openEverydayMenu(uiMessage("cli.commandsHeading"), everydayCommands(), status);
        else
            openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), status);
    }
    else if (event.altKey && ["ArrowLeft", "ArrowRight"].includes(event.key)) {
        event.preventDefault();
        void run(() => stepNavigation(event.key === "ArrowLeft" ? -1 : 1));
    }
});
const orderedDrafts = new WeakMap<Doc, OrderedDraft>();
type HostedSessionRetirement = {
    sessionId: string;
    audience: HostedSessionAudience;
    waitFor?: { target: Doc; input: ReturnType<typeof hostedInput>; audience: HostedSessionAudience; transmission?: OrderedDraftTransmission };
    closing?: Promise<void>;
};
const hostedWriterRetirements = new Map<string, HostedSessionRetirement>();
function queueHostedWriterRetirement(previous: Doc, target: Doc): void {
    if (previous.sessionId === target.sessionId) return;
    const input = hostedInput(target);
    const cut = {target,input,audience:sessionAudience(),transmission:draftOrder(target).issued(input)};
    for (const entry of hostedWriterRetirements.values()) if (entry.waitFor?.target.sessionId === previous.sessionId) entry.waitFor = cut;
    hostedWriterRetirements.set(previous.sessionId,{sessionId:previous.sessionId,audience:sessionAudience(),waitFor:cut});
}
async function retireHostedWriters(selected: Doc, transmission: OrderedDraftTransmission): Promise<void> {
    if (draftOrder(selected).acknowledged(transmission) !== transmission.inputRevision) return;
    for (const entry of hostedWriterRetirements.values()) if (entry.waitFor?.target.sessionId === selected.sessionId && entry.waitFor.transmission === transmission) entry.waitFor = undefined;
    for (const entry of Array.from(hostedWriterRetirements.values())) if (!entry.waitFor) await closeOwnedSession(entry.sessionId,entry.audience);
}

function draftOrder(selected: Doc): OrderedDraft { let order = orderedDrafts.get(selected); if (!order) { order = new OrderedDraft(); orderedDrafts.set(selected, order); } return order; }
function hostedInput(selected: Doc, value = source.value) {
    return { sessionId: selected.sessionId, source: value, serial: source.serial, inputId: source.authorInputId,
        ...(doc === selected && draft ? draft.basis?.sessionId === selected.sessionId ? { baseId: draft.basis.baseId } : { retainedBaseSource: draft.baseSource } : { baseId: selected.baseId }) };
}
function bindHostedTransmission(selected: Doc, transmission: OrderedDraftTransmission): void {
    for (const entry of hostedWriterRetirements.values()) if (entry.waitFor?.target.sessionId === selected.sessionId) {
        entry.waitFor.input = transmission;
        entry.waitFor.transmission = transmission;
    }
}
function issueHostedInput(selected: Doc, input: ReturnType<typeof hostedInput>, send: Parameters<OrderedDraft["persist"]>[1]) {
    let transmission!: OrderedDraftTransmission;
    const result = draftOrder(selected).persist(input,send,cut=>{transmission=cut;bindHostedTransmission(selected,cut);});
    return {transmission,result};
}
async function persistHostedDraft(selected: Doc, value = source.value): Promise<number> {
    const issued = issueHostedInput(selected,hostedInput(selected,value),async wire=>{
        const ack = await request<{status:string;inputRevision:number;evidence:"authorDraft"|"savedView"}>("draft",wire);
        if (ack.status !== "inputAcknowledged" || !["authorDraft","savedView"].includes(ack.evidence)) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
        return ack;
    });
    const revision = await issued.result;
    await retireHostedWriters(selected,issued.transmission);
    return revision;
}
async function hostedHistoryBasis(settle: boolean) {
    const selected = doc, epoch = requestEpoch;
    const basis = await source.historyBasis(settle);
    if (!selected || selected !== doc || epoch !== requestEpoch) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    await persistHostedDraft(selected, basis.source);
    if (selected !== doc || epoch !== requestEpoch || draftOrder(selected).acknowledged(hostedInput(selected)) === undefined) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    return basis;
}
async function save() {
    if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource))
        return;
    if (unresolvedPersonalSave(doc)) {
        status(uiMessage("hostedShell.originalSaveUnknownCannotSaveAgain"));
        return;
    }
    await source.flush();
    if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource))
        return;
    if (unresolvedPersonalSave(doc)) {
        status(uiMessage("hostedShell.originalSaveUnknownCannotSaveAgain"));
        return;
    }
    draft.source = source.value;
    const epoch = requestEpoch;
    const selected = doc;
    const input = hostedInput(selected);
    const inputRevision = await persistHostedDraft(selected, input.source);
    if (epoch !== requestEpoch || doc !== selected || draftOrder(selected).acknowledged(hostedInput(selected)) !== inputRevision) throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    const sent = input.source;
    const op = crypto.randomUUID();
    // Freeze the exact wire independently of the mutable editor before hashing.
    const frozen = freezeOriginalAttempt(freezeHostedSave({ ...draft, source: sent }, selected.sessionId, selected.baseId, op, inputRevision), "save");
    const exactRequest = JSON.parse(frozen.wire);
    await source.beginPersonalSave(op, sent);
    if (epoch !== requestEpoch || doc !== selected || !draft) {
        source.settlePersonalSave(op, false);
        return;
    }
    draft.source = source.value;
    draft.pending = { resource: selected.resource, operationId: op, source: sent, request: exactRequest };
    await retain();
    if (epoch !== requestEpoch || doc !== selected || !draft)
        return;
    if (unresolvedPersonalSave(selected, op)) {
        controls();
        status(uiMessage("hostedShell.originalSaveUnknownCannotSaveAgain"));
        return;
    }
    controls();
    queries.authorChanged();
    knowledgeAuthorChanged();
    try {
        const result = await request<SaveStatus>("save", exactRequest);
        if (epoch !== requestEpoch || doc !== selected || !draft)
            return;
        checkedSaveStatus(result, op, selected.resource);
        if (result.status !== "committed") {
            status(uiMessage("hostedShell.saveUnconfirmedCheckOriginal"));
            return;
        }
        doc.baseId = result.baseId!;
        doc.source = sent;
        await finishPersonalOperation(op, "committed", result);
        if (epoch !== requestEpoch || doc !== selected || !draft)
            return;
        draft = applySaveResult(draft, sent, {sessionId:selected.sessionId,baseId:result.baseId!});
        retain();
        source.refreshQueries();
        queries.authorChanged(true);
        knowledgeAuthorChanged();
        void queries.refreshCatalog();
        const laterInputRetained = draft.source !== sent;
        status(() => uiTextValue(uiMessage("shell.savedSentence")) + uiTextValue(laterInputRetained ? uiMessage("hostedShell.newInputStillInDraft") : ""));
    }
    catch (error) {
        if (epoch !== requestEpoch || !draft)
            throw error;
        // Network/unknown failure keeps the exact original ID until status lookup.
        if (error instanceof HostedError && error.http === 409) {
            let result = await request<SaveStatus>("operation", { resource: selected.resource, operationId: op });
            if (epoch !== requestEpoch || doc !== selected || !draft)
                throw error;
            checkedSaveStatus(result, op, selected.resource);
            if (["notRegistered", "notCommitted"].includes(result.status))
                result = await request<SaveStatus>("operation/settle", { resource: selected.resource, operationId: op });
            if (epoch !== requestEpoch || doc !== selected || !draft)
                throw error;
            checkedSaveStatus(result, op, selected.resource);
            if (result.status === "cancelled") {
                await finishPersonalOperation(op, "cancelled", result);
                if (epoch !== requestEpoch || doc !== selected || !draft)
                    return;
                delete draft.pending;
                compareRequired = true;
            }
        }
        retain();
        throw error;
    }
}
async function run(action: () => Promise<void>) {
    if (busy)
        return;
    const version = ++actionVersion;
    busy = true;
    controls();
    try {
        await action();
    }
    catch (e) {
        if (version === actionVersion)
            status(e instanceof Error ? errorUiText(e) : uiMessage("hostedShell.requestIncompleteDraftRetained"));
    }
    finally {
        if (version === actionVersion) {
            busy = false;
            source.disabled = !doc?.rights.write;
            controls();
        }
    }
}
function sourceChanged() {
    if (installingDocument)
        return;
    clearNodeNavigation();
    el<HTMLIFrameElement>("render").srcdoc = "";
    el("render").hidden = true;
    try {
        retain();
        status(navigator.onLine ? uiMessage("hostedShell.personalDraftBodyUnsaved") : uiMessage("hostedShell.connectionLostDraftRetained"));
    }
    catch {
        status(uiMessage("hostedShell.browserRecoveryUnavailableCopyInput"));
    }
    queries.authorChanged();
    knowledgeAuthorChanged();
    scheduleCurrentDraft();
}
function scheduleCurrentDraft() {
    clearTimeout(draftTimer);
    const captured = doc;
    const epoch = requestEpoch;
    draftTimer = setTimeout(() => {
        if (!captured || doc !== captured || epoch !== requestEpoch || source.pausedSession || !captured.rights.write || compareRequired || draft?.pending || busy || history.hasPending(captured.resource))
            return;
        void persistHostedDraft(captured).catch(() => { });
    }, 800);
}
el<HTMLFormElement>("login").onsubmit = e => { e.preventDefault(); token = el<HTMLInputElement>("token").value.trim(); void run(connect); };
el("logout").onclick = () => { clearDisplay(uiMessage("hostedShell.signedOut"), true); el<HTMLInputElement>("token").value = ""; };
el("refresh").onclick = () => void run(async () => {
    const epoch = requestEpoch, current = await request<Me>("me");
    if (epoch !== requestEpoch)
        return;
    if (!identity || current.generation !== identity.generation || current.actor !== identity.actor || current.deployment !== identity.deployment)
        clearDisplay(uiMessage("hostedShell.authorizationUpdatedViewsCacheCleared"));
    await confirmIdentity(current);
    if (identity !== current || !personalAudience())
        return;
    cachePrefix = hostedCachePrefix("draft", current.deployment, current.actor, current.generation);
    sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current));
    await connectPanels(current, requestEpoch, token);
});
el<HTMLFormElement>("search").onsubmit = e => { e.preventDefault(); void run(() => list(el<HTMLInputElement>("term").value.trim())); };
el("save").onclick = () => void run(save);
el("reopen").onclick = () => void run(() => open(doc!.resource));
el("rebase").onclick = () => void run(confirmDraftBasis);
async function confirmDraftBasis(): Promise<void> {
    if (!doc || !draft)
        return;
    const selected = doc, retained = draft, epoch = requestEpoch;
    await source.flush();
    if (doc !== selected || draft !== retained || requestEpoch !== epoch)
        throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    const basis = { sessionId: selected.sessionId, baseId: selected.baseId, source: source.value, inputId: source.authorInputId };
    const resolved = await request<{ status: string }>("draft/resolve", { sessionId: basis.sessionId, source: basis.source });
    if (resolved.status !== "draftRebased" || doc !== selected || draft !== retained || requestEpoch !== epoch || selected.baseId !== basis.baseId || source.value !== basis.source || source.authorInputId !== basis.inputId)
        throw new UiError(uiMessage("hostedEditor.draftOrSessionChanged"));
    retained.baseSource = selected.source;
    retained.basis = { sessionId: basis.sessionId, baseId: basis.baseId };
    compareRequired = false;
    knowledgeAuthorChanged();
    retain();
    el("comparison").hidden = true;
    status(uiMessage("hostedShell.mergeBasisConfirmedSaveToCommit"));
}
el("preview").onclick = () => void run(async () => {
    const epoch = requestEpoch;
    await source.flush();
    const renderedSource = source.value;
    const rendered = await request<{
        ok: boolean;
        html: string | null;
        explanation: unknown;
        diagnostics: unknown;
        causes: unknown;
    }>("preview", { sessionId: doc!.sessionId, source: renderedSource });
    if (epoch !== requestEpoch)
        return;
    if (source.value !== renderedSource) {
        status(uiMessage("hostedShell.draftChangedRegeneratePreview"));
        return;
    }
    const explanation = rendered.explanation === null ? null : parseUiMessage(rendered.explanation), diagnostics = decodeRawDiagnostics(rendered.diagnostics), causes = decodeErrorCauses(rendered.causes);
    if (typeof rendered.ok !== "boolean" || !diagnostics || !causes || (rendered.ok ? explanation !== null || typeof rendered.html !== "string" : !explanation))
        throw new UiError(uiMessage("error.invalidResponse"));
    if (!rendered.ok) {
        status(descriptionUiText({ explanation: explanation!, diagnostics, causes }));
        return;
    }
    // A fresh browsing context avoids reusing a hidden empty srcdoc document,
    // which some Chromium builds retain with a zero-sized layout after reload.
    const frame = document.createElement("iframe");
    frame.id = "render";
    setUiAttr(frame, "title", uiMessage("hostedShell.authorizedContentPreview"));
    frame.setAttribute("sandbox", "");
    frame.srcdoc = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:;"><style>body{padding:24px;line-height:1.8;font-family:serif}img,svg{max-width:100%}</style>${rendered.html}`;
    el("render").replaceWith(frame);
    status(uiMessage("hostedShell.previewAuthorizedDependenciesOnly"));
});
el("download").onclick = () => void run(async () => {
    const epoch = requestEpoch;
    const resource = doc!.resource;
    const prepared = await request<{
        ticket: string;
        length: number;
        sha256: string;
        chunkSize: number;
    }>("download/prepare", { resource });
    const chunks: ArrayBuffer[] = [];
    for (let offset = 0; offset < prepared.length; offset += prepared.chunkSize) {
        chunks.push(await request<ArrayBuffer>("download/chunk", { ticket: prepared.ticket, offset }, true));
        if (epoch !== requestEpoch)
            return;
    }
    const blob = new Blob(chunks, { type: "text/plain;charset=utf-8" });
    const hash = [...new Uint8Array(await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()))].map(b => b.toString(16).padStart(2, "0")).join("");
    if (hash !== prepared.sha256)
        throw new UiError(uiMessage("hostedShell.downloadIncompleteNoDelivery"));
    if (epoch !== requestEpoch)
        return;
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${resource}.typ`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    status(uiMessage("hostedShell.savedSourceDownloaded"));
});
el("history").onclick = () => { void history.open().then(() => selectHostedPanel("history-list", { button: el<HTMLButtonElement>("history"), tab: "review" })).catch(error => status(errorUiText(error))); };
el("personal-input-recovery").onclick = () => {
    try {
        showPersonalInputs();
    }
    catch (error) {
        status(error instanceof Error ? errorUiText(error) : String(error));
    }
};
el("copy-reference").onclick = () => void run(async () => {
    const selected = stableReference, epoch = requestEpoch;
    if (!selected || selected.sessionId !== doc?.sessionId)
        return;
    const latest = await request<{
        resource: string;
        reference: string;
    }>("navigation/ref", { sessionId: selected.sessionId });
    if (epoch !== requestEpoch || stableReference !== selected || latest.resource !== selected.resource || latest.reference !== selected.reference)
        return;
    await navigator.clipboard.writeText(latest.reference);
    if (epoch === requestEpoch)
        status(uiMessage("hostedShell.authorizedStableNoteLinkCopied"));
});
window.addEventListener("offline", () => status(uiMessage("hostedShell.connectionLostNoAutomaticCommit")));
window.addEventListener("beforeunload", e => {
    if (personalInputs.pending() || personalControls.inspect().producers.some(producer => producer.unapplied || producer.nativePending || producer.pendingDeliveries) || structures.hasPending || media.hasPending || library.hasPending || history.hasPending() || draft && (draft.pending || draft.source !== draft.baseSource)) {
        e.preventDefault();
    }
});
window.addEventListener("pagehide", e => {
    if (e.persisted)
        return;
    try {
        retain();
    }
    catch { /* A storage refusal must not prevent session release. */ }
    releaseViewSessions();
});
setInterval(() => {
    if (!identity || busy)
        return;
    const old = identity, epoch = requestEpoch, credential = token;
    void request<Me>("me").then(async (next) => {
        if (epoch !== requestEpoch || credential !== token || identity !== old)
            return;
        if (next.generation !== old.generation || next.actor !== old.actor || next.deployment !== old.deployment) {
            clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshDocuments"));
            await confirmIdentity(next);
        }
        else
            void refreshOpenSession();
    }).catch(() => { });
}, 5000);

function showOfficeQuery(id: Parameters<HostedOffice["setQuery"]>[0], metadata: Parameters<HostedOffice["setQuery"]>[1]): void {
    office.setQuery(id, metadata);
    selectHostedPanel("office");
}
/** Reparent known presentation surfaces once; original controllers keep every
 * authorization flag, pending proposal and original-operation owner. */
function installHostedLayout(): void {
    const workspace = el("workspace"), article = workspace.querySelector("article")!, navigation = workspace.querySelector("aside")!;
    workspace.classList.add("hosted-workspace");
    article.classList.add("hosted-document");
    navigation.classList.add("hosted-navigation");
    const documentBar = document.createElement("div");
    documentBar.className = "hosted-document-bar";
    const toggleNavigation = document.createElement("button");
    toggleNavigation.type = "button";
    toggleNavigation.className = "hosted-navigation-toggle";
    toggleNavigation.append(hostedChromeIcon("navigation"));
    setUiAttr(toggleNavigation, "aria-label", uiMessage("hostedChrome.navigation"));
    setUiAttr(toggleNavigation, "title", uiMessage("hostedChrome.navigation"));
    toggleNavigation.setAttribute("aria-controls", "hosted-navigation");
    navigation.id = "hosted-navigation";
    const narrowNavigation = matchMedia("(max-width: 760px)");
    const syncNavigation = () => {
        const expanded = narrowNavigation.matches ? workspace.classList.contains("is-navigation-open") : !workspace.classList.contains("is-navigation-collapsed");
        if (!expanded && navigation.contains(document.activeElement)) toggleNavigation.focus({ preventScroll: true });
        toggleNavigation.setAttribute("aria-expanded", String(expanded));
        navigation.setAttribute("aria-hidden", String(!expanded));
        navigation.inert = !expanded;
    };
    toggleNavigation.onclick = () => {
        workspace.classList.toggle(narrowNavigation.matches ? "is-navigation-open" : "is-navigation-collapsed");
        syncNavigation();
    };
    narrowNavigation.addEventListener("change", syncNavigation);
    syncNavigation();
    el("title").classList.add("hosted-title-label");
    documentBar.append(toggleNavigation, everydayBar, documentTabs, el("title"), el("save"));
    const ribbonHost = document.createElement("div");
    ribbonHost.className = "hosted-ribbon-host";
    ribbonHost.append(ribbon.element);
    const body = document.createElement("div");
    body.className = "hosted-workspace-body";
    const canvas = document.createElement("div");
    canvas.className = "hosted-canvas";
    el("source").setAttribute("role", "tabpanel");
    for (const id of ["node-navigation", "source", "comparison", "personal-inputs", "render"]) canvas.append(el(id));
    body.append(canvas, inspector);
    const addPanel = (id: string, title: UiText, wide = false, tab: RibbonTab = "knowledge"): HTMLButtonElement => {
        const host = el(id), slot = document.createElement("div"), button = document.createElement("button");
        slot.className = "hosted-panel-slot";
        slot.hidden = true;
        slot.append(host);
        button.type = "button";
        setUiText(button, title);
        button.setAttribute("aria-pressed", "false");
        button.setAttribute("aria-controls", id);
        button.onclick = () => selectHostedPanel(id, { button, tab });
        hostedPanels.set(id, { host, slot, button, trigger: button, triggerTab: tab, tab, title, wide });
        inspectorBody.append(slot);
        const observer = new MutationObserver(syncHostedPanels);
        observer.observe(host, { attributes: true, attributeFilter: ["hidden"] });
        panelObservers.push(observer);
        return button;
    };
    const panelButtons = {
        fields: addPanel("fields", uiMessage("hostedShell.panelProperties"), false, "home"),
        query: addPanel("query", uiMessage("hostedShell.panelQueries"), true),
        links: addPanel("knowledge-links", uiMessage("hostedShell.panelKnowledgeLinks"), true),
        citations: addPanel("citations", uiMessage("hostedShell.panelCitations")),
        tags: addPanel("navigation-registry", uiMessage("hostedShell.panelTags")),
        review: addPanel("review", uiMessage("hostedShell.panelAnnotations"), true, "review"),
        media: addPanel("media", uiMessage("hostedShell.panelMedia"), true),
        office: addPanel("office", uiMessage("hostedShell.panelOfficeImportOutput"), true, "file"),
        contentImport: addPanel("content-import", uiMessage("shell.contentImport"), true, "file"),
        library: addPanel("library", uiMessage("hostedShell.panelLibraryConfiguration"), false, "file"),
        structure: addPanel("structure", uiMessage("hostedShell.panelStructure"), false, "file"),
        backup: addPanel("owner-backup", uiMessage("hostedShell.panelRecoveryMaterials"), false, "file"),
        capture: addPanel("capture-panel", uiMessage("capture.title"), false, "file"),
    };
    const historyRecovery = addPanel("history-list", uiMessage("shell.timeline"), true, "file");
    hostedPanels.get("capture-panel")!.trigger = captureButton;
    hostedPanels.get("knowledge-links")!.slot.prepend(el("stable-link"));
    renderEverydayBar();
    ribbon.setGroup("file", "hosted-file", uiMessage("hostedChrome.files"), [el("download"), el("reopen"), everydayControls.get("recent")!]);
    ribbon.setGroup("file", "hosted-delivery", uiMessage("hostedShell.panelOfficeImportOutput"), [panelButtons.office, panelButtons.contentImport, recognitionButton]);
    ribbon.setGroup("file", "hosted-library", uiMessage("hostedChrome.library"), [captureButton, panelButtons.library, panelButtons.structure]);
    ribbon.setGroup("file", "hosted-recovery", uiMessage("hostedShell.panelRecoveryMaterials"), [historyRecovery, panelButtons.backup]);
    ribbon.setGroup("file", "hosted-settings", uiMessage("hostedChrome.settings"), [interfaceLocaleHost, el("logout")]);
    ribbon.setGroup("home", "hosted-note", uiMessage("hostedChrome.note"), [panelButtons.fields, el("personal-input-recovery")]);
    ribbon.setGroup("insert", "hosted-insert", uiMessage("hostedChrome.content"), [everydayControls.get("insert")!, el("latex"), el("diagram")]);
    ribbon.setGroup("knowledge", "hosted-knowledge", uiMessage("hostedChrome.knowledge"), [panelButtons.query, panelButtons.links, panelButtons.citations, panelButtons.tags, panelButtons.media]);
    ribbon.setGroup("review", "hosted-review", uiMessage("hostedChrome.review"), [panelButtons.review, el("history")]);
    ribbon.setGroup("view", "hosted-preview", uiMessage("hostedChrome.preview"), [el("preview")]);
    source.attachRibbon(ribbon);
    source.attachScrollHost(canvas);
    for (const stale of article.children) disposeUiTree(stale);
    article.replaceChildren(documentBar, ribbonHost, body);
    root.append(el("status"));
    syncHostedPanels();
    window.addEventListener("pagehide", event => { if (!event.persisted) panelObservers.forEach(observer => observer.disconnect()); });
}
installHostedLayout();

if (token)
    void run(connect);
import { readStrictJsonResponse } from "./strict-json";
