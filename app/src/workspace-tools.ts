import { type WorkspaceRibbon, type RibbonTab } from "./workspace-chrome";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { openReminderOverview } from "./reminder-overview";
import { activateReminderNotification } from "./reminder-activation";
import { joinUiText } from "./ui-locale";
import { apiErrorUiText } from "./api-error";
import { decodeProcessingRead } from "./style-wire";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { committedDeliveryComplete, deliverCommitted, originalCommitted, originalDeliveryErrors } from "./original-operation";
import "./data-view.css";
import "./style-panel.css";
import "./export-panel.css";
import { api, apiBlob, operationId } from "./api";
import { openSyncPanel } from "./sync-panel";
import { createLibraryTrashController } from "./library-trash";
import { mountMediaViewer, mountResourceViewer } from "./media-viewer";
import { decodeResourceList } from "./media-wire";
import type { ResourceOpenTarget } from "./media-selection";
import "./media-viewer.css";
import { DataPanel, type DocumentBasis, type PanelHost, type SourcePatch } from "./data-panel";
import { QueryPanel } from "./query-panel";
import { OutputPanel } from "./export-panel";
import { mountKnowledgePanel, type KnowledgeContext, type KnowledgePlan, type KnowledgeReceipt, type KnowledgeTextMapping } from "./knowledge-panel";
import { decodeKnowledgeDelivery, sameKnowledgeBasis, type KnowledgeNodeRef } from "./knowledge-state";
import { openLibraryStylePanel, type NativeStyleController } from "./style-panel";
import { LatexInsertionPanel } from "./latex-insertion";
import { DiagramPanel } from "./diagram-ui";
import { openProcessingSettingsPanel, decodeProcessingDocument, type ProcessingRead } from "./processing-settings-panel";
import type { OpenedDocument } from "./types";
import type { FrozenQuery, EmbeddedQuerySelection } from "./export-state";
import type { StructureHeading } from "./heading-structure-state";
import { DocumentOutlineView, type OutlineSnapshot } from "./document-outline";
export interface WorkspaceToolsHost {
    embeddedQueries?(): EmbeddedQuerySelection[];
    getDocument(): (DocumentBasis & {
        dirty: boolean;
        workspaceId?: string;
    }) | null;
    selection(): KnowledgeContext["selection"];
    getOutlineSnapshot(): OutlineSnapshot | null;
    applyPatch(patch: SourcePatch, basis: DocumentBasis): Promise<void>;
    openDocument(path: string, root?: string): Promise<void>;
    openResolved(document: OpenedDocument): Promise<void>;
    openResolvedConfirmed(document: OpenedDocument): Promise<boolean>;
    releaseOpenedSession(sessionId: string): Promise<void>;
    created(document: OpenedDocument, operationId: string | undefined): Promise<void>;
    applyReceipt(receipt: KnowledgeReceipt, context: KnowledgeContext): Promise<void>;
    savedDefinition(result: unknown, basis: DocumentBasis): Promise<void>;
    navigate(target: string | KnowledgeNodeRef): Promise<void>;
    selectSource(from: number, to: number): void;
    revealHeading?(heading: StructureHeading): boolean;
    beforeAction(): Promise<void>;
    canEditSource(): boolean;
    notify(message: UiText, error?: boolean): void;
    stylesChanged(root: string): Promise<void>;
    refreshLibrary?(root: string): Promise<void>;
    syncApplied(root: string): Promise<void>;
}
export type WorkspaceViewKind = "data" | "knowledge" | "query" | "outline";
export interface WorkspaceToolsPresentation {
    reveal(kind: WorkspaceViewKind): void;
    hide(kind: WorkspaceViewKind): void;
    visible(kind: WorkspaceViewKind): boolean;
}
/** UI composition only: every action uses the document's existing Draft or Core. */
export class WorkspaceTools {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly dataHost: PanelHost;
    private closeOwners(): { assertCloseReady(): void; freezeForClose(): boolean; thawAfterClose(): void }[] {
        return [this.propertyPanel, this.queryPanel, this.outputPanel, this.knowledgePanel, this.libraryTrash, ...this.styles.values()];
    }
    assertCloseReady(): void { this.closeGuard.assertReady(this.pendingImport !== null); for (const owner of this.closeOwners()) owner.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        const frozen: ReturnType<WorkspaceTools["closeOwners"]> = [];
        try {
            for (const owner of this.closeOwners()) {
                frozen.push(owner);
                if (!owner.freezeForClose()) throw new Error("Workspace proposal owner refused close");
            }
            if (!this.closeGuard.freeze(this.pendingImport !== null)) throw new Error("Workspace proposal changed during close");
        }
        catch {
            for (const owner of frozen.reverse()) owner.thawAfterClose();
            this.closeGuard.thaw();
            return false;
        }
        clearTimeout(this.refreshTimer);
        return true;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); for (const owner of this.closeOwners()) owner.thawAfterClose(); }
    openReminders(): void { this.closeGuard.assertMutable(); openReminderOverview(this.dataHost); }
    activateReminder(activationKey: string): Promise<void> { return this.closeGuard.run(() => activateReminderNotification(this.dataHost, activationKey)); }
    private readonly commandGroups: { tab: RibbonTab; id: string; label: UiText; nodes: HTMLElement[] }[] = [];
    /** Stable content owners. Dock layout only moves these elements. */
    readonly views: Readonly<Record<WorkspaceViewKind, HTMLElement>>;
    private readonly propertyPanel: DataPanel;
    private readonly queryPanel: QueryPanel;
    private readonly outputPanel: OutputPanel;
    private readonly knowledgeContainer = document.createElement("div");
    private readonly knowledgePanel;
    private readonly outlinePanel: DocumentOutlineView;
    private readonly libraryTrash;
    private library: {
        root: string;
        workspaceId?: string;
    } | null = null;
    private presentation: WorkspaceToolsPresentation | null = null;
    private readonly styles = new Map<string, NativeStyleController>();
    private refreshTimer?: ReturnType<typeof setTimeout>;
    private pendingImport: {
        plan: KnowledgePlan;
        context: KnowledgeContext;
        receipt?: KnowledgeReceipt;
    } | null = null;
    constructor(private readonly host: WorkspaceToolsHost) {
        this.libraryTrash = createLibraryTrashController({ request: api, created: host.created, notify: host.notify, environmentChanged: root => this.invalidateLibraryEnvironment(root) });
        const dataHost: PanelHost = {
            canEdit: host.canEditSource,
            request: api, requestBlob: apiBlob, getDocument: host.getDocument, applyPatch: host.applyPatch,
            openDocument: host.openDocument, openResolved: host.openResolved, openResolvedConfirmed: host.openResolvedConfirmed, releaseOpenedSession: host.releaseOpenedSession, notify: host.notify,
            environmentChanged: root => this.invalidateLibraryEnvironment(root),
            savedDefinition: host.savedDefinition,
            applyOperationReceipt: (receipt, basis) => host.applyReceipt(receipt as KnowledgeReceipt, { ...basis, dirty: basis.dirty ?? false }),
            prepareAction: host.beforeAction,
            openTemplates: async () => {
                await this.show("knowledge");
                this.knowledgePanel.selectTab("templates");
            },
            created: async (result, operationId) => {
                const opened = result as OpenedDocument;
                if (!opened || typeof opened.sessionId !== "string" || typeof opened.source !== "string")
                    throw new UiError(uiMessage("workspaceTools.createdNoteOpenResultMissing"));
                await host.created(opened, operationId);
            },
        };
        this.dataHost = dataHost;
        this.propertyPanel = new DataPanel(dataHost);
        this.outputPanel = new OutputPanel({ ...dataHost, embeddedQueries: host.embeddedQueries, imported: async (receipt) => {
                const root = receipt.root ?? dataHost.getDocument()?.root;
                if (!root)
                    throw new UiError(uiMessage("workspaceTools.importReceiptLibraryLocationMissing"));
                await host.refreshLibrary?.(root);
                if (this.library?.root === root)
                    await this.queryPanel.refresh();
            } });
        this.queryPanel = new QueryPanel(dataHost, query => this.outputPanel.open(query));
        const latex = new LatexInsertionPanel({ ...dataHost, selection: host.selection, editable: host.canEditSource });
        const diagrams = new DiagramPanel(dataHost, host.canEditSource);
        this.knowledgePanel = mountKnowledgePanel(this.knowledgeContainer, {
            context: () => this.context(), load: context => api("/api/knowledge/read", { context }),
            proofread: async (context, options) => {
                const render = await api<{
                    ok: boolean;
                }>("/api/render", { sessionId: context.sessionId, source: context.source, serial: context.serial });
                if (!render.ok)
                    throw new UiError(uiMessage("workspaceTools.compileBeforeProofreading"));
                return api("/api/knowledge/proofread", { context, options });
            },
            processingDefaults: async () => {
                const root = this.library?.root;
                if (!root)
                    throw new UiError(uiMessage("workspaceTools.openLibraryFirst"));
                const suggestion = decodeProcessingRead(await api("/api/processing/read", { root }), decodeProcessingDocument);
                if (this.library?.root !== root)
                    throw new UiError(uiMessage("workspaceTools.libraryChangedRereadSuggestions"));
                if (suggestion.warning)
                    throw new UiError(apiErrorUiText(suggestion.warning));
                return { processing: suggestion.processing === null ? null : decodeProcessingDocument(suggestion.processing), source: suggestion.source, baseId: suggestion.baseId };
            },
            prepare: (intent, context, id) => api("/api/knowledge/prepare", { operationId: id, context, intent }),
            commit: (plan, context) => this.commit(plan, context),
            navigate: host.navigate, notify: host.notify, beforeAction: host.beforeAction, canEditSource: host.canEditSource,
            inspectOperation: async (id, context) => {
                const receipt = decodeKnowledgeDelivery(await api<KnowledgeReceipt>("/api/operations/status", { operationId: id, context }));
                if (receipt.operationId !== id) throw new UiError(uiMessage("error.invalidResponse"));
                if (originalCommitted(receipt) && !committedDeliveryComplete(receipt)) this.invalidateLibraryEnvironment(context.root);
                return deliverCommitted(receipt, value => host.applyReceipt(value, context));
            },
            selectSource: host.selectSource,
            previewTemplate: (id, parameters, context) => api("/api/knowledge/template-preview", { templateId: id, parameters, context }),
            linkCopy: (link, form, context) => api("/api/knowledge/link-copy", { occurrence: link.occurrence, form, context }),
            mountMedia: (container, selected, initial) => mountMediaViewer(container, {
                context: () => this.context(), list: async (sessionId) => decodeResourceList(await api("/api/resources/list", { sessionId })),
                read: (sessionId, resource, revision) => apiBlob("/api/resources/read", { sessionId, resource, revision }),
                importResource: (file, sessionId) => this.importResource(file, sessionId), notify: host.notify,
            }, selected, initial),
        });
        const outline = document.createElement("section");
        this.outlinePanel = new DocumentOutlineView(outline, {
            getDocument: host.getDocument, getOutlineSnapshot: host.getOutlineSnapshot,
            selectSource: host.selectSource, revealHeading: host.revealHeading, notify: host.notify,
            canNavigate: () => !this.closeGuard.frozen,
        });
        this.views = Object.freeze({ data: this.propertyPanel.element, knowledge: this.knowledgeContainer, query: this.queryPanel.element, outline });
        for (const [kind, view] of Object.entries(this.views)) {
            view.classList.add("workspace-tool-view");
            view.dataset.workspaceTool = kind;
            this.closeGuard.bind(view);
        }
        const add = (tab: RibbonTab, id: string, label: UiText, nodes: HTMLElement[]) => this.commandGroups.push({ tab, id, label, nodes });
        add("knowledge", "note", uiMessage("localChrome.note"), [
            this.button(uiMessage("dataPanel.title"), () => this.toggle("data")),
            this.button(uiMessage("workspaceTools.linksAnnotations"), () => this.toggle("knowledge")),
            this.button(uiMessage("workspaceTools.queriesViews"), () => this.toggle("query")),
        ]);
        add("file", "exchange", uiMessage("localChrome.exchange"), [this.button(uiMessage("export.title"), () => this.outputPanel.open())]);
        add("knowledge", "library", uiMessage("localChrome.library"), [
            this.button(uiMessage("error.api.hostedLibrary.styleRoleTitle"), () => this.showStyles()),
            this.button(uiMessage("knowledge.recycleBin"), async () => {
                if (!this.library?.workspaceId) throw new UiError(uiMessage("workspaceTools.openLibraryFirst"));
                await this.host.beforeAction();
                await this.libraryTrash.open(this.library.root);
            }),
            this.button(uiMessage("workspaceTools.sync"), async () => { await this.host.beforeAction(); await openSyncPanel(this.library?.root ?? "", { beforeAction: this.host.beforeAction, applied: this.host.syncApplied }); }),
            this.button(uiMessage("workspaceTools.processingDefaultsVocabulary"), () => this.showProcessingSettings()),
        ]);
        add("insert", "objects", uiMessage("localChrome.objects"), [
            this.button(uiMessage("hostedShell.latexEquation"), () => latex.open()),
            this.button(uiMessage("hostedShell.mermaidDiagram"), () => diagrams.open()),
        ]);
        add("review", "review", uiMessage("localChrome.review"), [
            this.button(uiMessage("localChrome.annotations"), async () => { await this.show("knowledge"); this.selectKnowledgeTab("annotations"); }),
            this.button(uiMessage("knowledge.proofreading"), async () => { await this.show("knowledge"); this.selectKnowledgeTab("proofreading"); }),
        ]);
    }
    /** Ribbon switches retain these same command nodes and every proposal owner. */
    mountRibbon(ribbon: WorkspaceRibbon): void {
        this.closeGuard.bind(ribbon.element);
        for (const group of this.commandGroups) ribbon.setGroup(group.tab, group.id, group.label, group.nodes);
    }

    attachDock(presentation: WorkspaceToolsPresentation): void {
        this.closeGuard.assertMutable();
        this.presentation = presentation;
    }
    private viewVisible(kind: WorkspaceViewKind): boolean { return this.presentation?.visible(kind) ?? false; }

    private async showProcessingSettings(): Promise<void> {
        return this.closeGuard.run(async () => {
            await this.host.beforeAction();
            const root = this.library?.root;
            if (!root)
                throw new UiError(uiMessage("workspaceTools.openLibraryFirst"));
            await openProcessingSettingsPanel({
                persistenceKey: `weftext-processing:${root}`,
                read: () => api<ProcessingRead>("/api/processing/read", { root }),
                prepare: request => api("/api/processing/prepare", { root, ...request }),
                status: request => api("/api/processing/status", { root, ...request }),
                settle: request => api("/api/processing/settle", { root, ...request }),
                apply: request => api("/api/processing/apply", { request: { root, ...request.request }, planId: request.planId }),
                committed: () => this.invalidateLibraryEnvironment(root),
                onApplied: () => { this.host.notify(uiMessage("workspaceTools.sharedDefaultsCommitted")); },
            });
        });
    }
    private button(label: UiText, run: () => void | Promise<void>): HTMLButtonElement {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "text-button";
        setUiText(button, label);
        button.onclick = () => { void this.closeGuard.run(async () => { await run(); }).catch(error => this.host.notify(errorUiText(error), true)); };
        return button;
    }
    private context(): KnowledgeContext | null { const doc = this.host.getDocument(); return doc ? { ...doc, selection: this.host.selection() } : null; }
    assertLibraryDeparture(): void {
        this.closeGuard.assertMutable();
        this.queryPanel.assertLibraryDeparture();
    }
    /** All rejecting owners are checked before either consumer changes scope. */
    assertLibraryChange(library: { root: string; workspaceId?: string } | null): void {
        this.closeGuard.assertMutable();
        this.queryPanel.assertLibraryChange(library);
    }
    setLibrary(library: {
        root: string;
        workspaceId?: string;
    } | null, scope: string | null = null): void {
        this.assertLibraryChange(library);
        this.queryPanel.setLibrary(library);
        this.queryPanel.setScope(scope);
        this.library = library;
    }
    invalidateLibraryEnvironment(root = this.library?.root): void { if (this.closeGuard.frozen) return;
        if (root) {
            this.knowledgePanel.environmentChanged(root);
            this.queryPanel.environmentChanged(root);
        }
    }
    async search(text: string, includeBody: boolean): Promise<void> {
        return this.closeGuard.run(async () => {
            this.queryPanel.setSearch(text, includeBody);
            await this.show("query");
        });
    }
    openQueryOutput(query: FrozenQuery): void { if (this.closeGuard.frozen) return; this.outputPanel.open(query); }
    openResource(documentBasis: OpenedDocument, target: ResourceOpenTarget): void {
        const end = this.closeGuard.begin();
        const dialog = document.createElement("dialog");
        dialog.className = "modal";
        const title = document.createElement("h2");
        setUiText(title, uiMessage("workspaceTools.noteResources"));
        const container = document.createElement("div");
        // A read-only resource view owns the exact resolved session, independently
        // of a conflicting editor Draft or a later active-tab change.
        const controller = mountResourceViewer(container, {
            context: () => ({ sessionId: documentBasis.sessionId }),
            list: async (sessionId) => decodeResourceList(await api("/api/resources/list", { sessionId })),
            read: (sessionId, resource, revision) => apiBlob("/api/resources/read", { sessionId, resource, revision }),
            notify: this.host.notify,
        }, target);
        const close = (): void => { controller.dispose(); dialog.close(); dialog.remove(); end(); };
        dialog.append(title, container, this.button(uiMessage("common.close"), close));
        dialog.oncancel = event => { event.preventDefault(); close(); };
        document.body.append(dialog);
        dialog.showModal();
    }
    /** Opening a view never replaces another view or its proposal owner. */
    async show(kind: WorkspaceViewKind): Promise<void> {
        return this.closeGuard.run(async () => {
            if (kind !== "outline") await this.host.beforeAction();
            if (!this.presentation) throw new Error("Workspace dock is not attached");
            this.presentation.reveal(kind);
            await this.refreshView(kind);
        });
    }
    /** Only toolbar switches use toggle; explicit open actions always use show. */
    private async toggle(kind: WorkspaceViewKind): Promise<void> {
        return this.closeGuard.run(async () => {
            if (kind !== "outline") await this.host.beforeAction();
            if (!this.presentation) throw new Error("Workspace dock is not attached");
            if (this.presentation.visible(kind)) {
                this.presentation.hide(kind);
                return;
            }
            this.presentation.reveal(kind);
            await this.refreshView(kind);
        });
    }
    /** Accepted native projections can refresh navigation without a preview surface. */
    refreshOutline(): void {
        if (this.closeGuard.frozen) return;
        this.outlinePanel.refresh();
    }
    /** Dock-tab activation reads the current context without settling author input. */
    async refreshView(kind: WorkspaceViewKind): Promise<void> {
        return this.closeGuard.run(async () => {
            if (kind === "outline")
                this.outlinePanel.refresh();
            else if (kind === "data")
                await this.propertyPanel.refresh();
            else if (kind === "query")
                await this.queryPanel.refresh();
            else
                await this.knowledgePanel.refresh();
        });
    }
    selectKnowledgeTab(tab: import("./knowledge-panel").KnowledgePanelTab): void {
        if (this.closeGuard.frozen) return;
        const visible = this.viewVisible("knowledge");
        this.presentation?.reveal("knowledge");
        this.knowledgePanel.selectTab(tab);
        if (!visible)
            void this.refreshView("knowledge").catch(error => this.host.notify(errorUiText(error), true));
    }
    /** Selection/adoption must renew an open property consumer even when its
     * empty or previous form owns focus. Body edits keep their debounce below. */
    async selectedDocumentChanged(): Promise<void> {
        if (this.closeGuard.frozen) return;
        return this.closeGuard.run(async () => {
            this.outlinePanel.refresh();
            if (this.viewVisible("data")) {
                clearTimeout(this.refreshTimer);
                await this.propertyPanel.refresh();
            }
            if (this.viewVisible("knowledge") || this.knowledgePanel.backgroundEnabled() || !this.host.getDocument())
                await this.knowledgePanel.refresh();
        });
    }
    documentChanged(): void { if (this.closeGuard.frozen) return;
        clearTimeout(this.refreshTimer);
        this.outlinePanel.invalidate();
        this.knowledgePanel.attachSurface(null);
        // A known moved writer has no current consumer context until explicit
        // comparison. This retires snapshots/late delivery without reading paths.
        if (!this.host.getDocument())
            void this.knowledgePanel.refresh().catch(error => this.host.notify(String(error), true));
        this.refreshTimer = setTimeout(() => {
            if (this.closeGuard.frozen) return;
            if (this.viewVisible("data") && !this.views.data.contains(document.activeElement))
                void this.propertyPanel.refresh().catch(error => this.host.notify(String(error), true));
            if (this.viewVisible("query") && !this.views.query.contains(document.activeElement))
                this.queryPanel.documentChanged();
            if (this.viewVisible("knowledge") || this.knowledgePanel.backgroundEnabled())
                void this.knowledgePanel.refresh().catch(error => this.host.notify(errorUiText(error), true));
        }, 450);
    }
    async refreshAfterSync(root: string): Promise<void> {
        return this.closeGuard.run(async () => {
            clearTimeout(this.refreshTimer);
            // Retire pre-sync knowledge before any other consumer can await.
            this.knowledgePanel.environmentChanged(root);
            this.outlinePanel.refresh();
            const sameRoot = (value?: string): boolean => value?.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase() === root.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
            // Query results depend on the library, even without an open source. The
            // panel retains its current plan/parameters; frozen outputs stay frozen.
            if (sameRoot(this.library?.root))
                await this.queryPanel.refresh();
            if (!this.host.getDocument()) {
                if (this.viewVisible("data"))
                    await this.propertyPanel.refresh();
                await this.knowledgePanel.refresh();
                return;
            }
            if (!sameRoot(this.host.getDocument()?.root))
                return;
            if (this.viewVisible("data"))
                await this.propertyPanel.refresh();
            // Knowledge refresh also renews the selected media/annotation catalog.
            // Its own input guard retains an actively edited review form.
            if (this.viewVisible("knowledge") || this.knowledgePanel.backgroundEnabled())
                await this.knowledgePanel.refresh();
        });
    }
    async projectionChanged(surface: HTMLElement | null, mappings: KnowledgeTextMapping[]): Promise<void> {
        return this.closeGuard.run(async () => {
            this.outlinePanel.refresh();
            this.knowledgePanel.attachSurface(surface, mappings);
            // Knowledge discovery observes the projection; it must never delay body
            // installation or an unrelated property/navigation action. Default-off
            // hints also avoid compiling other notes while this panel is closed.
            if (this.host.getDocument()?.nodeId && (this.viewVisible("knowledge") || this.knowledgePanel.backgroundEnabled()))
                void this.knowledgePanel.refresh().catch(error => this.host.notify(String(error), true));
        });
    }
    attachSurface(surface: HTMLElement | null, mappings: KnowledgeTextMapping[] = []): void { if (this.closeGuard.frozen) return; this.knowledgePanel.attachSurface(surface, mappings); }
    private async importResource(file: File, sessionId: string): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.pendingImport) {
                const pending = this.pendingImport;
                const status = decodeKnowledgeDelivery(await api<KnowledgeReceipt>("/api/operations/status", { operationId: pending.plan.operationId, context: pending.context }));
                if (status.operationId !== pending.plan.operationId)
                    throw new UiError(uiMessage("error.invalidResponse"));
                if (pending.receipt && originalCommitted(pending.receipt) && !originalCommitted(status))
                    throw new UiError(uiMessage("workspaceTools.importCommittedStatusMismatch"));
                pending.receipt = status;
                if (originalCommitted(status)) {
                    if (!committedDeliveryComplete(status)) this.invalidateLibraryEnvironment(pending.context.root);
                    const delivered = await deliverCommitted(status, value => this.host.applyReceipt(value, pending.context));
                    pending.receipt = delivered;
                    if (!committedDeliveryComplete(delivered))
                        throw new UiError(originalDeliveryErrors(delivered).length ? joinUiText(originalDeliveryErrors(delivered), "\n") : uiMessage("workspaceTools.resourceImportDeliveryPending"));
                    this.pendingImport = null;
                    this.host.notify(uiMessage("workspaceTools.previousResourceImportConfirmed"));
                    return;
                }
                if (status.status === "resolved") {
                    this.pendingImport = null;
                    throw new UiError(uiMessage("workspaceTools.originalImportEnded"));
                }
                throw new UiError(uiMessage("workspaceTools.originalImportPending"));
            }
            if (file.size > 64 * 1024 * 1024)
                throw new UiError(uiMessage("workspaceTools.resourceSizeLimit"));
            await this.host.beforeAction();
            const context = this.context();
            if (!context || context.sessionId !== sessionId)
                throw new UiError(uiMessage("workspaceTools.noteChangedSelectResourceAgain"));
            const array = new Uint8Array(await file.arrayBuffer());
            let binary = "";
            for (let index = 0; index < array.length; index += 8192)
                binary += String.fromCharCode(...array.subarray(index, index + 8192));
            const plan = await api<KnowledgePlan>("/api/resources/prepare-import", { sessionId, name: file.name, bytes: btoa(binary), operationId: operationId() });
            const dialog = document.createElement("dialog");
            dialog.className = "modal";
            const title = document.createElement("h2");
            setUiText(title, uiMessage("workspaceTools.importResource"));
            const description = document.createElement("p");
            setUiText(description, displayMessage("workspaceTools.resourceCopyDescription", { p0: uiDisplayParameter(file.name), p1: uiDisplayParameter(file.size.toLocaleString()) }));
            const list = document.createElement("ul");
            for (const change of plan.changes ?? []) {
                const item = document.createElement("li");
                setUiText(item, change.path ?? uiMessage("workspaceTools.currentResourceManifest"));
                list.append(item);
            }
            dialog.append(title, description, list);
            document.body.append(dialog);
            const accepted = await new Promise<boolean>(resolve => {
                const finish = (result: boolean): void => { dialog.close(); dialog.remove(); resolve(result); };
                dialog.append(this.button(uiMessage("common.cancel"), () => finish(false)), this.button(uiMessage("workspaceTools.import"), () => finish(true)));
                dialog.oncancel = event => { event.preventDefault(); finish(false); };
                dialog.showModal();
            });
            if (!accepted)
                return;
            this.pendingImport = { plan, context };
            try {
                const result = await this.commit(plan, context);
                this.pendingImport.receipt = result;
                if (committedDeliveryComplete(result))
                    this.pendingImport = null;
                else
                    throw new UiError(originalDeliveryErrors(result).length ? joinUiText(originalDeliveryErrors(result), "\n") : uiMessage("workspaceTools.originalImportResultPending"));
            }
            catch (error) {
                this.host.notify(uiMessage("workspaceTools.resourceImportUnconfirmed"), true);
                throw error;
            }
        });
    }
    private async commit(plan: KnowledgePlan, context: KnowledgeContext): Promise<KnowledgeReceipt> {
        return this.closeGuard.run(async () => {
            if (!sameKnowledgeBasis(context, this.context()))
                throw new UiError(uiMessage("workspaceTools.draftChangedPrepareAgain"));
            if (plan.draftOnly === true) {
                const edit = plan.edit as SourcePatch | undefined;
                if (!edit || typeof edit.from !== "number" || typeof edit.to !== "number" || typeof edit.text !== "string" || typeof plan.source !== "string")
                    throw new UiError(uiMessage("workspaceTools.linkSourceChangesIncomplete"));
                if (context.source.slice(0, edit.from) + edit.text + context.source.slice(edit.to) !== plan.source)
                    throw new UiError(uiMessage("workspaceTools.linkProposalPreviewMismatch"));
                await this.host.applyPatch(edit, context);
                this.host.notify(uiMessage("workspaceTools.linkUpdatedInDraft"));
                return { status: "committed", operationId: plan.operationId };
            }
            const receipt = decodeKnowledgeDelivery(await api<KnowledgeReceipt>("/api/operations/commit", { operationId: plan.operationId, planId: plan.planId, context }));
            if (receipt.operationId !== plan.operationId)
                throw new UiError(uiMessage("error.invalidResponse"));
            // Delivery errors still describe a known library commit; their early
            // return in deliverCommitted cannot leave pre-commit reads eligible.
            if (originalCommitted(receipt) && !committedDeliveryComplete(receipt)) this.invalidateLibraryEnvironment(context.root);
            const delivered = await deliverCommitted(receipt, value => this.host.applyReceipt(value, context));
            if (!committedDeliveryComplete(delivered))
                this.host.notify(uiMessage("workspaceTools.committedUiUpdateIncomplete"), true);
            return delivered;
        });
    }
    private async showStyles(): Promise<void> {
        return this.closeGuard.run(async () => {
            const library = this.library;
            if (!library?.workspaceId)
                throw new UiError(uiMessage("workspaceTools.openLibraryBeforeStyle"));
            await this.host.beforeAction();
            const root = library.root;
            const controller = await openLibraryStylePanel({
                read: () => api("/api/styles/read", { root }),
                prepare: request => api("/api/styles/prepare", { root, ...request }),
                apply: request => api("/api/styles/apply", { root, ...request }),
                committed: () => this.invalidateLibraryEnvironment(root),
                openSource: path => this.host.openDocument(path, root),
                onApplied: () => this.host.stylesChanged(root), operationId,
            }, this.styles.get(root));
            this.styles.set(root, controller);
        });
    }
}
