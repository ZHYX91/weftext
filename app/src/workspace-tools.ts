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
export interface WorkspaceToolsHost {
    embeddedQueries?(): EmbeddedQuerySelection[];
    getDocument(): (DocumentBasis & {
        dirty: boolean;
        workspaceId?: string;
    }) | null;
    selection(): KnowledgeContext["selection"];
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
type WorkspacePanelKind = "data" | "knowledge" | "query";
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
    readonly inspector = document.createElement("aside");
    private readonly propertyPanel: DataPanel;
    private readonly queryPanel: QueryPanel;
    private readonly outputPanel: OutputPanel;
    private readonly knowledgeContainer = document.createElement("div");
    private readonly knowledgePanel;
    private readonly libraryTrash;
    private readonly header = document.createElement("header");
    private readonly panelBody = document.createElement("div");
    private library: {
        root: string;
        workspaceId?: string;
    } | null = null;
    private active: WorkspacePanelKind | null = null;
    private readonly styles = new Map<string, NativeStyleController>();
    private refreshTimer?: ReturnType<typeof setTimeout>;
    private pendingImport: {
        plan: KnowledgePlan;
        context: KnowledgeContext;
        receipt?: KnowledgeReceipt;
    } | null = null;
    constructor(private readonly host: WorkspaceToolsHost) {
        this.closeGuard.bind(this.inspector);
        this.libraryTrash = createLibraryTrashController({ request: api, created: host.created, notify: host.notify, environmentChanged: root => this.invalidateQueryEnvironment(root) });
        this.inspector.className = "workspace-inspector";
        this.inspector.hidden = true;
        this.header.className = "inspector-header";
        this.panelBody.className = "inspector-body";
        this.inspector.append(this.header, this.panelBody);
        const dataHost: PanelHost = {
            canEdit: host.canEditSource,
            request: api, requestBlob: apiBlob, getDocument: host.getDocument, applyPatch: host.applyPatch,
            openDocument: host.openDocument, openResolved: host.openResolved, openResolvedConfirmed: host.openResolvedConfirmed, releaseOpenedSession: host.releaseOpenedSession, notify: host.notify,
            environmentChanged: root => this.invalidateQueryEnvironment(root),
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
            inspectOperation: async (id, context) => { const receipt = decodeKnowledgeDelivery(await api<KnowledgeReceipt>("/api/operations/status", { operationId: id, context })); if (receipt.operationId !== id)
                throw new UiError(uiMessage("error.invalidResponse")); return deliverCommitted(receipt, value => host.applyReceipt(value, context)); },
            selectSource: host.selectSource,
            revealHeading: host.revealHeading,
            previewTemplate: (id, parameters, context) => api("/api/knowledge/template-preview", { templateId: id, parameters, context }),
            linkCopy: (link, form, context) => api("/api/knowledge/link-copy", { occurrence: link.occurrence, form, context }),
            mountMedia: (container, selected, initial) => mountMediaViewer(container, {
                context: () => this.context(), list: async (sessionId) => decodeResourceList(await api("/api/resources/list", { sessionId })),
                read: (sessionId, resource, revision) => apiBlob("/api/resources/read", { sessionId, resource, revision }),
                importResource: (file, sessionId) => this.importResource(file, sessionId), notify: host.notify,
            }, selected, initial),
        });
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
                committed: () => this.invalidateQueryEnvironment(root),
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
    invalidateQueryEnvironment(root = this.library?.root): void { if (this.closeGuard.frozen) return;
        if (root)
            this.queryPanel.environmentChanged(root);
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
    /** Opening is idempotent; a retained active panel must never turn this into
     * closing after navigation or a repeated command. */
    async show(kind: WorkspacePanelKind): Promise<void> {
        return this.closeGuard.run(async () => {
            await this.host.beforeAction();
            await this.showPanel(kind);
        });
    }
    /** Only toolbar switches use toggle; explicit open actions always use show. */
    private async toggle(kind: WorkspacePanelKind): Promise<void> {
        return this.closeGuard.run(async () => {
            await this.host.beforeAction();
            if (this.active === kind) {
                this.active = null;
                this.inspector.hidden = true;
                return;
            }
            await this.showPanel(kind);
        });
    }
    private async showPanel(kind: WorkspacePanelKind): Promise<void> {
        return this.closeGuard.run(async () => {
            this.active = kind;
            this.inspector.hidden = false;
            this.inspector.classList.toggle("wide", kind === "query");
            const title = document.createElement("strong");
            setUiText(title, { data: uiMessage("dataPanel.regionLabel"), knowledge: uiMessage("workspaceTools.linksAnnotations"), query: uiMessage("workspaceTools.queriesViews") }[kind]);
            this.header.replaceChildren(title, this.button(uiMessage("common.close"), () => { this.active = null; this.inspector.hidden = true; }));
            this.panelBody.replaceChildren(kind === "data" ? this.propertyPanel.element : kind === "query" ? this.queryPanel.element : this.knowledgeContainer);
            if (kind === "data")
                await this.propertyPanel.refresh();
            else if (kind === "query")
                await this.queryPanel.refresh();
            else
                await this.knowledgePanel.refresh();
        });
    }
    selectKnowledgeTab(tab: import("./knowledge-panel").KnowledgePanelTab): void { if (this.closeGuard.frozen) return; this.knowledgePanel.selectTab(tab); }
    documentChanged(): void { if (this.closeGuard.frozen) return;
        clearTimeout(this.refreshTimer);
        this.knowledgePanel.attachSurface(null);
        // A known moved writer has no current consumer context until explicit
        // comparison. This retires snapshots/late delivery without reading paths.
        if (!this.host.getDocument())
            void this.knowledgePanel.refresh().catch(error => this.host.notify(String(error), true));
        this.refreshTimer = setTimeout(() => {
            if (this.closeGuard.frozen) return;
            if (this.active === "data" && !this.inspector.contains(document.activeElement))
                void this.propertyPanel.refresh().catch(error => this.host.notify(String(error), true));
            if (this.active === "query" && !this.inspector.contains(document.activeElement))
                this.queryPanel.documentChanged();
        }, 450);
    }
    async refreshAfterSync(root: string): Promise<void> {
        return this.closeGuard.run(async () => {
            clearTimeout(this.refreshTimer);
            const sameRoot = (value?: string): boolean => value?.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase() === root.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
            // Query results depend on the library, even without an open source. The
            // panel retains its current plan/parameters; frozen outputs stay frozen.
            if (sameRoot(this.library?.root))
                await this.queryPanel.refresh();
            if (!this.host.getDocument()) {
                if (this.active === "data")
                    await this.propertyPanel.refresh();
                await this.knowledgePanel.refresh();
                return;
            }
            if (!sameRoot(this.host.getDocument()?.root))
                return;
            if (this.active === "data")
                await this.propertyPanel.refresh();
            // Knowledge refresh also renews the selected media/annotation catalog.
            // Its own input guard retains an actively edited review form.
            await this.knowledgePanel.refresh();
        });
    }
    async projectionChanged(surface: HTMLElement | null, mappings: KnowledgeTextMapping[]): Promise<void> {
        return this.closeGuard.run(async () => {
            this.knowledgePanel.attachSurface(surface, mappings);
            // Knowledge discovery observes the projection; it must never delay body
            // installation or an unrelated property/navigation action. Default-off
            // hints also avoid compiling other notes while this panel is closed.
            if (this.host.getDocument()?.nodeId && (this.active === "knowledge" || this.knowledgePanel.backgroundEnabled()))
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
                committed: () => this.invalidateQueryEnvironment(root),
                openSource: path => this.host.openDocument(path, root),
                onApplied: () => this.host.stylesChanged(root), operationId,
            }, this.styles.get(root));
            this.styles.set(root, controller);
        });
    }
}
