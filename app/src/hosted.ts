import {parseUiMessage} from "./ui-locale";
import {decodeRawDiagnostics,decodeErrorCauses,descriptionUiText} from "./ui-diagnostics";
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
import { PersonalInputRecovery, type PersonalInputRecord } from "./personal-input-recovery";
import { renderPersonalInputs } from "./personal-input-panel";
import { replayPersonalInput } from "./source-input-origin";
import { PersonalControlInput } from "./personal-control-input";
import type { AuthorInputFactory } from "./author-input";
import { freezeOriginalAttempt } from "./original-operation";
import { HostedLibrary } from "./hosted-library";
import { HostedMedia } from "./hosted-media";
import { HostedStructure } from "./hosted-structure";
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
import { bindHostedDraftReference, draftDisposition, applySaveResult, freezeHostedSave, hostedOpenedDraft, mergeDefinitionReservations, hostedCachePrefix, sameHostedQueryAudience, sameHostedQueryIdentity, type HostedDraft } from "./hosted-state";
type Rights = {
    read: boolean;
    write: boolean;
    history: boolean;
    download: boolean;
};
type Doc = {
    resource: string;
    title: string;
    sessionId: string;
    baseId: string;
    source: string;
    recoveredDraft?: string;
    recoveredDraftBaseSource?: string;
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
function checkedSaveStatus(result: SaveStatus, operationId: string, resource: string): SaveStatus {
    if (!result || typeof result !== "object" || result.operationId !== operationId || !["committed", "cancelled", "resolved", "recoveryPending", "notCommitted", "notRegistered"].includes(result.status) || result.status !== "notRegistered" && result.resource !== resource || result.status === "committed" && (typeof result.baseId !== "string" || !result.baseId))
        throw new Error("原保存回执的编号、目标或基线不匹配；没有采用该结果。");
    return result;
}
const root = document.querySelector<HTMLElement>("#hosted")!;
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
window.addEventListener("pagehide", event => { if (!event.persisted)
    interfaceLocale?.dispose(); });
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const everydayNavigation = new EverydayNavigation();
let navigatingHistory = false;
const everydayBar = document.createElement("nav");
everydayBar.className = "everyday-bar";
setUiAttr(everydayBar, "aria-label", uiMessage("shell.everydayNavigation"));
el("source").before(everydayBar);
function everydayEntries() { return structureDocuments.map(item => ({ key: item.resource, title: item.title, detail: item.resource, open: () => run(() => open(item.resource)) })); }
function everydayCommands(): EverydayCommand[] {
    return [
        { id: "quick-open", label: uiMessage("shell.quickOpen"), run: () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), status) },
        { id: "relationship-graph", label: uiMessage("shell.openRelationshipGraph"), run: () => knowledgeLinks.showStructure("graph") },
        { id: "heading-structure", label: uiMessage("shell.openHeadingStructure"), run: () => knowledgeLinks.showStructure("headings") },
        { id: "recent", label: uiMessage("shell.recentVisits"), run: showRecent },
        { id: "save", label: uiMessage("shell.saveCurrentNote"), unavailable: !authoringEditable() ? uiMessage("hostedShell.bodyCannotSave") : undefined, run: () => run(save) },
        ...source.commands(async () => { el("query").hidden = false; el("query").scrollIntoView(); el("query").querySelector<HTMLInputElement>('input[data-control-role="definition-label"]')?.focus(); status(uiMessage("hostedShell.queryPromotePlanHint")); }),
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
function renderEverydayBar() {
    if (doc && !navigatingHistory)
        everydayNavigation.visit(doc.resource);
    everydayBar.replaceChildren();
    const add = (label: UiText, action: () => void | Promise<void>, disabled = false) => { const button = document.createElement("button"); button.type = "button"; setUiText(button, label); button.disabled = disabled; button.onclick = () => { void Promise.resolve().then(action).catch(error => status(String(error))); }; everydayBar.append(button); };
    add(uiMessage("shell.back"), () => run(() => stepNavigation(-1)), !everydayNavigation.target(-1) || busy);
    add(uiMessage("shell.forward"), () => run(() => stepNavigation(1)), !everydayNavigation.target(1) || busy);
    add(uiMessage("shell.quickOpen"), () => openEverydayPicker(uiMessage("shell.quickOpen"), everydayEntries(), status), busy);
    add(uiMessage("shell.recentVisits"), showRecent, busy);
    add(uiMessage("cli.commandsHeading"), () => openEverydayMenu(uiMessage("cli.commandsHeading"), everydayCommands(), status), busy);
    add(uiMessage("shell.insertBlockActions"), () => openEverydayMenu(uiMessage("shell.insertBlockActions"), source.commands(), status), busy);
}
const personalInputs = new PersonalInputRecovery(sessionStorage);
const personalControls = new PersonalControlInput(document, context => context.audience === personalAudience() && context.epoch === authorizationEpoch, ({ context, id, revision, text }) => {
    if (context.owner.kind === "capture")
        throw new Error("捕获输入不能写入正文恢复域。");
    try {
        personalInputs.update(context.audience, [{ id, owner: context.owner, slot: context.slot, path: context.path, origin: { baseLength: 0, pieces: [], detached: text ? [{ id, revision, text }] : [], revision } }]);
        controls();
    }
    catch (error) {
        status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.personalFormInputMemoryOnly"));
    }
});
const authorInputs: AuthorInputFactory = (owner, current, sink, leaseRevision) => { const audience = personalAudience(); return audience ? personalControls.session(audience, authorizationEpoch, owner, current, sink, leaseRevision) : undefined; };
const fields = new HostedFields(el("fields"), request, (id, metadata) => office.setQuery(id, metadata), () => queries.environmentChanged(uiMessage("hostedShell.fieldsChangedQueryMustRerun")), authorInputs);
const source: HostedEditor = new HostedEditor(el("source"), request, sourceChanged, () => { void run(save); }, error => status(error.message), {
    canOpenRow: row => (row as HostedQueryRow).canOpen === true && !!(row as HostedQueryRow).resource,
    openRow: row => {
        const target = row as HostedQueryRow;
        if (target.canOpen !== true || !target.resource)
            return Promise.reject(new UiError(uiMessage("hostedShell.queryRowNoBodyReadRight")));
        return openQueryResource(target.resource);
    },
    openOwner: async () => { throw new UiError(uiMessage("hostedShell.querySourceNoAuthorizedNavigation")); },
    output: (query, result) => office.setQuery(query.queryId, { columns: result.columns ?? [], rowCount: result.rows.length, coverage: result.coverage }),
    modeChanged: () => { queries.authorChanged(); knowledgeAuthorChanged(); },
    showHeadingStructure: () => knowledgeLinks.showStructure("headings"),
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
    serial: source.serial, baseId: doc.baseId, dirty: source.value !== doc.source || compareRequired,
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
    getDocument: () => doc && identity && !source.pausedSession ? { sessionId: doc.sessionId, source: source.value, serial: source.serial, baseId: doc.baseId, path: `resource:${doc.resource}`, root: "", dirty: source.value !== doc.source } : null,
    applyPatch: async (patch, basis) => {
        await source.flush();
        if (!authoringEditable() || doc?.sessionId !== basis.sessionId || doc.baseId !== basis.baseId || source.value !== basis.source || source.serial !== basis.serial)
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
async function closeOwnedSession(sessionId: string, captured: Pick<HostedNodeAudience, "authorization" | "credential" | "actor" | "generation" | "deployment">): Promise<void> {
    // Disposing a former coordinate must never be interpreted as revoking a
    // different view. Even after logout, use only the original credential, never
    // the next actor's token. Cleanup has no global error or content delivery.
    if (!captured.credential)
        return;
    try {
        const response = await fetch("/api/hosted/close", { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${captured.credential}` }, body: JSON.stringify({ sessionId }), cache: "no-store", keepalive: true, signal: AbortSignal.timeout(10000) });
        await response.arrayBuffer();
    }
    catch { /* The server lease also bounds abandoned sessions when delivery fails. */ }
}
function releaseViewSessions() {
    const captured = sessionAudience(), sessions = new Set(retiredStructureSessions);
    if (doc)
        sessions.add(doc.sessionId);
    for (const id of sessions)
        void closeOwnedSession(id, captured);
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
const queries = new HostedQuery(el("query"), request, openQueryResource, (id, metadata) => office.setQuery(id, metadata), {
    context: () => doc ? { resource: doc.resource, sessionId: doc.sessionId, baseId: doc.baseId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending, compare: compareRequired, definitionReservations: draft?.definitionReservations } : null,
    prepareBasis: () => source.prepareQueryBasis(),
    apply: (patches, basis) => source.applyQueryPatches(patches, basis, "input.query.definition.proposal"),
    authorInputs,
    reservations: (resource, receipts, includeReserved = false) => {
        if (!doc || !draft || resource !== doc.resource)
            return;
        draft.definitionReservations = mergeDefinitionReservations(draft.definitionReservations ?? [], receipts, includeReserved);
        retain();
    },
}, () => source.queryInputsChanged(), navigateResource, { canEdit: (resource, fieldId) => fields.canEdit(resource, fieldId), open: (resource, fieldId) => fields.openGranted(resource, fieldId) });
const knowledgeLinks = new HostedKnowledgeLinks(el("knowledge-links"), (path, body, signal) => request(path, body, false, 0, signal), {
    identity: hostedIdentity,
    context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
    flush: async () => { await source.historyBasis(true); },
    apply: async (patches, basis, event) => { retain(); await source.applyQueryPatches(patches, basis, event); },
    navigate: reference => reference.startsWith("weftext-resource://") ? navigateResource(reference) : navigateReference(reference),
    select: (from, to) => source.selectSourceRange(from, to),
    revealRendered: heading => source.revealRendered(heading.key, heading),
    authorizationChanged: (message, current) => { clearDisplay(message); confirmIdentity({ ...identity!, ...current }); },
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
        confirmIdentity({ ...identity, ...current });
}
const citations = new HostedCitations(el("citations"), request, {
    identity: hostedIdentity,
    context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
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
    context: () => doc && draft ? { resource: doc.resource, sessionId: doc.sessionId, path: `resource:${doc.resource}`, root: "", baseId: doc.baseId, baseSource: doc.source, source: source.value, serial: source.serial, dirty: source.value !== doc.source, readOnly: !doc.rights.write || source.viewMode === "read", savePending: !!draft.pending, unresolvedSave: compareRequired } : null,
    basis: settle => source.historyBasis(settle), pendingChanged: () => { controls(); queries.authorChanged(); knowledgeAuthorChanged(); scheduleCurrentDraft(); }, restored: acknowledgeHistory, finishUnavailable: finishUnavailableHistory,
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
            await request("draft", { sessionId: selected.sessionId, source: source.value });
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
        const ref = await request<{
            resource: string;
            reference: string;
        }>("navigation/ref", { sessionId: fresh.sessionId });
        if (epoch !== requestEpoch || doc !== selected || ref.reference !== expected || ref.resource !== fresh.resource)
            throw new UiError(uiMessage("hostedShell.structureNewLocationEligibilityChanged"));
        if (original?.reference === expected) {
            const retained = selected?.resource === intent.source && draft ? { ...draft, source: source.value } : original;
            if (selected?.resource === intent.source && source.pausedSession)
                source.rebindResourceAlias(fresh.resource);
            requestEpoch++;
            installDocument(fresh, retained);
            adopted = true;
            if (fresh.rights.write && retained.source !== fresh.source) {
                await request("draft", { sessionId: fresh.sessionId, source: retained.source });
                if (doc === fresh)
                    scheduleCurrentDraft();
            }
        }
        else {
            requestEpoch++;
            installDocument(fresh, cached(fresh.resource) ?? { source: fresh.recoveredDraft ?? fresh.source, baseSource: fresh.recoveredDraftBaseSource ?? fresh.source, reference: ref.reference });
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
        let target: Doc, targetDraft: HostedDraft;
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
            let local: HostedDraft | undefined;
            try {
                local = cached(target.resource);
            }
            catch {
                throw new UiError(uiMessage("hostedShell.targetRecoveryUnparseable"));
            }
            targetDraft = nodeTargetDraft(target.source, target.recoveredDraft, target.recoveredDraftBaseSource, source.pausedSession && draft?.reference === parts.base ? { ...draft, source: source.value } : local);
        }
        const retainedAlias = !currentTarget && source.pausedSession && draft?.reference === parts.base;
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
            if (retainedAlias)
                source.rebindResourceAlias(target.resource);
            requestEpoch++;
            installDocument(target, targetDraft);
            adopted = true;
            if (prior)
                void closeOwnedSession(prior.sessionId, captured);
            scheduleCurrentDraft();
        }
        adoptedAudience = nodeAudience();
        source.locateNodeFragment(origin, { sessionId: target.sessionId, source: basis.source, serial: basis.serial });
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
async function readReference(selected: Doc, epoch: number) {
    stableReference = undefined;
    el("stable-link").hidden = true;
    el<HTMLInputElement>("node-reference").value = "";
    try {
        const result = await request<{
            resource: string;
            reference: string;
        }>("navigation/ref", { sessionId: selected.sessionId });
        if (epoch !== requestEpoch || doc !== selected || !draft)
            return;
        if (result.resource !== selected.resource || !/^weftext-node:\/\//.test(result.reference))
            throw new UiError(uiMessage("hostedShell.noteAuthorizedStableIdentityMissing"));
        const qualified = bindHostedDraftReference({ ...draft, source: source.value }, result.reference);
        draftCache.write(selected.resource, qualified);
        draft = qualified;
        source.personalInputReference(selected.resource, result.reference);
        stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference: result.reference };
        el<HTMLInputElement>("node-reference").value = result.reference;
        el("stable-link").hidden = false;
        controls();
    }
    catch (error) {
        if (epoch === requestEpoch && doc === selected)
            status(error instanceof Error ? errorUiText(error) : uiMessage("hostedShell.stableIdentityUnavailable"));
    }
}
async function acknowledgeHistory(record: HostedHistoryRecord): Promise<"applied" | "superseded"> {
    const selected = doc, epoch = requestEpoch;
    if (!selected || !draft || selected.resource !== record.request.resource || !record.status?.result)
        throw new UiError(uiMessage("hostedShell.openOriginalForRestoreReceipt"));
    await source.flush();
    if (epoch !== requestEpoch || doc !== selected || !draft)
        throw new UiError(uiMessage("hostedShell.objectChangedRestoreReceiptRetained"));
    const fresh = await request<Doc>("open", { resource: selected.resource });
    let adopted = false;
    try {
        const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
        if (epoch !== requestEpoch || doc !== selected || !draft)
            throw new UiError(uiMessage("hostedShell.restoreObjectOrUserChanged"));
        const accepted = history.installStatus(record, confirmed);
        if (!accepted.result)
            throw new UiError(uiMessage("hostedShell.restoreResultNoCurrentDeliveryRight"));
        const installed = currentHostedHistoryResult(record, fresh.source, accepted), buffer = { sessionId: selected.sessionId, source: source.value, serial: source.serial };
        const current: HistoryContext = { ...buffer, path: `resource:${selected.resource}`, root: "", baseId: selected.baseId, baseSource: selected.source, dirty: buffer.source !== selected.source, nodeId: record.basis.nodeId, workspaceId: record.basis.workspaceId };
        if (installed && restoreBufferDisposition(record.basis, current) === "replaceClean")
            source.applyHistorySource(accepted.result.source, buffer);
        const retainedSource = source.value, originalDraftBase = draft.baseSource;
        doc = fresh;
        source.open(fresh.sessionId, fresh.resource, retainedSource);
        source.disabled = !fresh.rights.write;
        draft = { ...draft, source: retainedSource, baseSource: installed ? fresh.source : originalDraftBase };
        compareRequired = !installed;
        el("comparison").hidden = installed;
        setUiText(el("current"), installed ? "" : fresh.source);
        setUiText(el("title"), fresh.title);
        adopted = true;
        retain();
        history.documentChanged();
        controls();
        queries.authorChanged(true);
        knowledgeAuthorChanged();
        void queries.refreshCatalog();
        void readReference(fresh, epoch);
        try {
            await request("close", { sessionId: selected.sessionId });
        }
        catch (error) {
            if (!(error instanceof HostedError) || error.http !== 410)
                throw error;
        }
        if (installed && retainedSource !== fresh.source && fresh.rights.write)
            await request("draft", { sessionId: fresh.sessionId, source: retainedSource });
        return installed ? "applied" : "superseded";
    }
    finally {
        if (!adopted) {
            try {
                await request("close", { sessionId: fresh.sessionId });
            }
            catch { /* Keep the original receipt even if session cleanup became unavailable. */ }
        }
    }
}
async function finishUnavailableHistory(record: HostedHistoryRecord): Promise<void> {
    const selected = doc, epoch = requestEpoch;
    if (!selected || !draft || selected.resource !== record.request.resource || !unavailableCommittedHistory(record))
        throw new UiError(uiMessage("hostedShell.openOriginalConfigurationQueryFact"));
    await source.flush();
    if (epoch !== requestEpoch || doc !== selected || !draft)
        throw new UiError(uiMessage("hostedShell.objectChangedOriginalFactRetained"));
    const fresh = await request<Doc>("open", { resource: selected.resource });
    let adopted = false;
    try {
        const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
        if (epoch !== requestEpoch || doc !== selected || !draft)
            throw new UiError(uiMessage("hostedShell.rereadObjectOrUserChanged"));
        history.installStatus(record, confirmed);
        if (!unavailableCommittedHistory(record))
            throw new UiError(uiMessage("hostedShell.originalDeliveryStatusChanged"));
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
        void readReference(fresh, epoch);
        try {
            await request("close", { sessionId: selected.sessionId });
        }
        catch (error) {
            if (!(error instanceof HostedError) || error.http !== 410)
                throw error;
        }
    }
    finally {
        if (!adopted) {
            try {
                await request("close", { sessionId: fresh.sessionId });
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
    el("personal-inputs").replaceChildren();
    el("personal-inputs").hidden = true;
    personalControls.clear();
    releaseViewSessions();
    clearTimeout(draftTimer);
    expiredSessionId = undefined;
    everydayNavigation.clear();
    everydayBar.replaceChildren();
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
    const clearErrors: unknown[] = [];
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
                    confirmIdentity(current);
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
                        confirmIdentity(current);
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
                    confirmIdentity(current);
                }
            }
            catch { /* Caller stops this resource delivery; personal source remains protected. */ }
        }
        else if ([404, 409, 410].includes(response.status) && (failureScope === "stagedTarget" || path === "office/legacy-capabilities" || path === "open" || path.startsWith("navigation/") || path.startsWith("structure/") || path.startsWith("knowledge-links/") || path.startsWith("citations/") || path.startsWith("export/") || path.startsWith("import/") || path.startsWith("owner-backup/") || path.startsWith("recognition/")) && sameHostedQueryIdentity(captured, audience())) {
            try {
                const current = await request<Me>("me");
                if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) {
                    clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshLinks"));
                    confirmIdentity(current);
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
    el<HTMLButtonElement>("save").disabled = busy || !doc?.rights.write || source.pausedSession || compareRequired || !!draft?.pending || !!doc && history.hasPending(doc.resource);
    for (const id of ["preview", "reopen"])
        el<HTMLButtonElement>(id).disabled = busy || !doc || source.pausedSession && id === "preview";
    el<HTMLButtonElement>("download").disabled = busy || !doc?.rights.download || source.pausedSession;
    el<HTMLButtonElement>("history").disabled = busy || !doc?.rights.history || source.pausedSession;
    el<HTMLButtonElement>("rebase").disabled = busy || !doc?.rights.write || source.pausedSession || !!draft?.pending;
    el<HTMLButtonElement>("copy-reference").disabled = busy || !stableReference || stableReference.sessionId !== doc?.sessionId;
}
function retain() {
    if (!doc || !draft)
        return;
    draft.source = source.value;
    // Storage failure is visible and prevents a save request that could lose an
    // unknown operation receipt on reload.
    persistPersonalInputs();
    draftCache.write(doc.resource, draft);
}
function confirmIdentity(next: Me): void {
    identity = next;
    verifiedPersonalAudience = JSON.stringify([next.deployment, next.actor]);
    sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(next));
    controls();
}
function personalAudience(): string | null { const current = identity ? JSON.stringify([identity.deployment, identity.actor]) : ""; return current && current === verifiedPersonalAudience ? current : null; }
function persistPersonalInputs(): void {
    const audience = personalAudience();
    if (!audience)
        return;
    const updates = source.personalInputRecords();
    personalInputs.update(audience, updates);
}
function finishPersonalOperation(operationId: string, terminal: "committed" | "cancelled" | "resolved"): void {
    source.settlePersonalSave(operationId, terminal === "committed");
    const audience = personalAudience();
    if (!audience)
        return;
    for (const record of personalInputs.list(audience).filter(record => record.pending?.operationId === operationId))
        personalInputs.settle(audience, record, terminal);
}
function showPersonalInputs(): void {
    const audience = personalAudience();
    if (!audience)
        throw new UiError(uiMessage("hostedShell.reauthenticateOriginalAccount"));
    const panel = el("personal-inputs");
    panel.hidden = false;
    renderPersonalInputs(panel, personalInputs, audience, { restore: record => restorePersonalInput(record, audience), operation: record => recoverPersonalOperation(record, audience), changed: () => {
            if (personalAudience() === audience) {
                controls();
                showPersonalInputs();
            }
        } });
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
        throw new Error("此记录没有正文原保存编号。");
    let outcome = await request<SaveStatus>("operation", { operationId: record.pending.operationId });
    if (authorizationEpoch !== epoch || personalAudience() !== audience)
        return;
    checkedSaveStatus(outcome, record.pending.operationId, record.owner.resource);
    if (["notRegistered", "notCommitted"].includes(outcome.status))
        outcome = await request<SaveStatus>("operation/settle", { resource: record.owner.resource, operationId: record.pending.operationId });
    if (authorizationEpoch !== epoch || personalAudience() !== audience)
        return;
    personalInputs.current(audience, record);
    checkedSaveStatus(outcome, record.pending.operationId, record.owner.resource);
    if (!["committed", "cancelled", "resolved"].includes(outcome.status))
        throw new UiError(uiMessage("hostedShell.originalSaveUnknownIdRetained"));
    finishPersonalOperation(record.pending.operationId, outcome.status as "committed" | "cancelled" | "resolved");
}
async function restorePersonalInput(record: PersonalInputRecord, audience: string): Promise<void> {
    if (record.owner.kind !== "body")
        throw new Error("此表单输入只供取回；没有借用当前正文目标自动重放。");
    const owner = record.owner;
    const selected = doc, epoch = authorizationEpoch;
    if (!selected || !draft || !selected.rights.write || compareRequired || draft.pending)
        throw new UiError(uiMessage("hostedShell.openOriginalCheckDraftAndOperation"));
    await source.flush();
    await verifyPersonalIdentity(audience, epoch);
    record = personalInputs.current(audience, record);
    if (doc !== selected || !draft || source.value !== selected.source || draft.source !== selected.source)
        throw new UiError(uiMessage("hostedShell.existingDraftInputNotOverwritten"));
    if (personalInputs.list(audience).some(other => other.pending?.status === "unresolved" && other.owner.kind === "body" && (other.owner.reference && other.owner.reference === owner.reference || other.owner.resource === owner.resource)))
        throw new UiError(uiMessage("hostedShell.originalSaveCheckSameId"));
    const basis = { sessionId: selected.sessionId, source: source.value, serial: source.serial };
    const reference = await request<{
        resource: string;
        reference: string;
    }>("navigation/ref", { sessionId: selected.sessionId });
    const hash = await nodeSourceHash(basis.source);
    if (authorizationEpoch !== epoch || personalAudience() !== audience || doc !== selected || source.value !== basis.source || source.serial !== basis.serial)
        throw new UiError(uiMessage("hostedShell.recoveryCheckNewInputOrObjectChanged"));
    personalInputs.current(audience, record);
    if (!owner.reference || reference.reference !== owner.reference || reference.resource !== selected.resource || !record.baseHash || hash !== record.baseHash)
        throw new UiError(uiMessage("hostedShell.recoveryTargetOrBaselineMismatch"));
    const restored = replayPersonalInput(record.origin, basis.source);
    source.restorePersonalInput(restored, record.origin, record.baseHash, basis);
    retain();
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
    confirmIdentity(next);
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
        let targetDraft = cached(resource) ?? { source: opened.recoveredDraft ?? opened.source, baseSource: opened.recoveredDraftBaseSource ?? opened.source };
        let alias = false, recoveredSave = false;
        if (prior && draft?.reference && source.pausedSession && prior.resource !== resource) {
            const ref = await request<{
                resource: string;
                reference: string;
            }>("navigation/ref", { sessionId: opened.sessionId });
            if (!current())
                return;
            if (ref.resource === opened.resource && ref.reference === draft.reference) {
                targetDraft = { ...draft, source: source.value };
                alias = true;
            }
        }
        if (targetDraft.pending) {
            let outcome = await request<SaveStatus>("operation", { operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget");
            if (!current())
                return;
            checkedSaveStatus(outcome, targetDraft.pending.operationId, resource);
            if (["notRegistered", "notCommitted"].includes(outcome.status))
                outcome = await request<SaveStatus>("operation/settle", { resource, operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget");
            if (!current())
                return;
            checkedSaveStatus(outcome, targetDraft.pending.operationId, resource);
            if (outcome.status === "committed") {
                finishPersonalOperation(targetDraft.pending.operationId, "committed");
                targetDraft = applySaveResult(targetDraft, targetDraft.pending.source);
                recoveredSave = true;
            }
            else if (["cancelled", "resolved"].includes(outcome.status)) {
                finishPersonalOperation(targetDraft.pending.operationId, outcome.status as "cancelled" | "resolved");
                delete targetDraft.pending;
            }
        }
        if (!await authority() || !current())
            return;
        retain();
        targetDraft = hostedOpenedDraft(opened.source, targetDraft);
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
        requestEpoch++;
        if (alias)
            source.rebindResourceAlias(resource);
        installDocument(opened, targetDraft, recoveredSave, true);
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
function installDocument(opened: Doc, retainedDraft: HostedDraft, recoveredSave = false, stagedCache = false) {
    clearNodeNavigation();
    doc = opened;
    draft = stagedCache ? retainedDraft : hostedOpenedDraft(opened.source, retainedDraft);
    const epoch = requestEpoch;
    installingDocument = true;
    try {
        source.open(opened.sessionId, opened.resource, draft.source);
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
    void readReference(opened, epoch);
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
async function save() {
    if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource))
        return;
    await source.flush();
    if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource))
        return;
    draft.source = source.value;
    const sent = draft.source;
    const epoch = requestEpoch;
    const op = crypto.randomUUID();
    const selected = doc;
    // Freeze the exact wire independently of the mutable editor before hashing.
    const frozen = freezeOriginalAttempt(freezeHostedSave(draft, doc.sessionId, doc.baseId, op), "save");
    const exactRequest = JSON.parse(frozen.wire);
    await source.beginPersonalSave(op, sent);
    if (epoch !== requestEpoch || doc !== selected || !draft) {
        source.settlePersonalSave(op, false);
        return;
    }
    draft.source = source.value;
    draft.pending = { operationId: op, source: sent, request: exactRequest };
    retain();
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
        finishPersonalOperation(op, "committed");
        draft = applySaveResult(draft, sent);
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
            let result = await request<SaveStatus>("operation", { operationId: op });
            if (epoch !== requestEpoch || doc !== selected || !draft)
                throw error;
            checkedSaveStatus(result, op, selected.resource);
            if (["notRegistered", "notCommitted"].includes(result.status))
                result = await request<SaveStatus>("operation/settle", { resource: selected.resource, operationId: op });
            if (epoch !== requestEpoch || doc !== selected || !draft)
                throw error;
            checkedSaveStatus(result, op, selected.resource);
            if (result.status === "cancelled") {
                finishPersonalOperation(op, "cancelled");
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
        void request("draft", { sessionId: captured.sessionId, source: source.value }).catch(() => { });
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
    confirmIdentity(current);
    cachePrefix = hostedCachePrefix("draft", current.deployment, current.actor, current.generation);
    sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current));
    await connectPanels(current, requestEpoch, token);
});
el<HTMLFormElement>("search").onsubmit = e => { e.preventDefault(); void run(() => list(el<HTMLInputElement>("term").value.trim())); };
el("save").onclick = () => void run(save);
el("reopen").onclick = () => void run(() => open(doc!.resource));
el("rebase").onclick = () => void run(async () => {
    if (!doc || !draft)
        return;
    await source.flush();
    await request("draft/resolve", { sessionId: doc.sessionId, source: source.value });
    draft.baseSource = doc.source;
    compareRequired = false;
    knowledgeAuthorChanged();
    retain();
    el("comparison").hidden = true;
    status(uiMessage("hostedShell.mergeBasisConfirmedSaveToCommit"));
});
el("preview").onclick = () => void run(async () => {
    const epoch = requestEpoch;
    await source.flush();
    const renderedSource = source.value;
    const rendered = await request<{
        ok: boolean;
        html: string|null;
        explanation:unknown;
        diagnostics:unknown;
        causes:unknown;
    }>("preview", { sessionId: doc!.sessionId, source: renderedSource });
    if (epoch !== requestEpoch)
        return;
    if (source.value !== renderedSource) {
        status(uiMessage("hostedShell.draftChangedRegeneratePreview"));
        return;
    }
    const explanation=rendered.explanation===null ? null : parseUiMessage(rendered.explanation), diagnostics=decodeRawDiagnostics(rendered.diagnostics), causes=decodeErrorCauses(rendered.causes);
    if(typeof rendered.ok!=="boolean" || !diagnostics || !causes || (rendered.ok ? explanation!==null || typeof rendered.html!=="string" : !explanation))
        throw new UiError(uiMessage("error.invalidResponse"));
    if (!rendered.ok) {
        status(descriptionUiText({explanation:explanation!,diagnostics,causes}));
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
el("history").onclick = () => void history.open();
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
    if (structures.hasPending || media.hasPending || library.hasPending || history.hasPending() || draft && (draft.pending || draft.source !== draft.baseSource)) {
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
    void request<Me>("me").then(next => {
        if (epoch !== requestEpoch || credential !== token || identity !== old)
            return;
        if (next.generation !== old.generation || next.actor !== old.actor || next.deployment !== old.deployment) {
            clearDisplay(uiMessage("hostedShell.authorizationChangedRefreshDocuments"));
            confirmIdentity(next);
        }
        else
            void refreshOpenSession();
    }).catch(() => { });
}, 5000);
if (token)
    void run(connect);
import { readStrictJsonResponse } from "./strict-json";
