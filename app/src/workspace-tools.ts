import { UiError, uiTextValue } from "./ui-locale";
import { committedDeliveryComplete, deliverCommitted, originalCommitted, originalDeliveryErrors } from "./original-operation";
import "./data-view.css";
import "./style-panel.css";
import "./export-panel.css";
import { api, apiBlob, operationId } from "./api";
import { openSyncPanel } from "./sync-panel";
import { createLibraryTrashController } from "./library-trash";
import { mountMediaViewer, mountResourceViewer } from "./media-viewer";
import type { ResourceOpenTarget } from "./media-selection";
import "./media-viewer.css";
import { DataPanel, type DocumentBasis, type PanelHost, type SourcePatch } from "./data-panel";
import { QueryPanel } from "./query-panel";
import { OutputPanel } from "./export-panel";
import { mountKnowledgePanel, type KnowledgeContext, type KnowledgePlan, type KnowledgeReceipt, type KnowledgeTextMapping } from "./knowledge-panel";
import { sameKnowledgeBasis, type KnowledgeNodeRef } from "./knowledge-state";
import { openLibraryStylePanel, type NativeStyleController } from "./style-panel";
import { LatexInsertionPanel } from "./latex-insertion";
import { DiagramPanel } from "./diagram-ui";
import { openProcessingSettingsPanel, type ProcessingRead } from "./processing-settings-panel";
import type { OpenedDocument } from "./types";
import type { FrozenQuery, EmbeddedQuerySelection } from "./export-state";
import type { StructureHeading } from "./heading-structure-state";

export interface WorkspaceToolsHost {
  embeddedQueries?(): EmbeddedQuerySelection[];
  getDocument(): (DocumentBasis & { dirty: boolean; workspaceId?: string }) | null;
  selection(): KnowledgeContext["selection"];
  applyPatch(patch: SourcePatch, basis: DocumentBasis): Promise<void>;
  openDocument(path: string, root?: string): Promise<void>;
  openResolved(document: OpenedDocument): Promise<void>;
  created(document: OpenedDocument): Promise<void>;
  applyReceipt(receipt: KnowledgeReceipt, context: KnowledgeContext): Promise<void>;
  savedDefinition(result: unknown, basis: DocumentBasis): Promise<void>;
  navigate(target: string | KnowledgeNodeRef): Promise<void>;
  selectSource(from: number, to: number): void;
  revealHeading?(heading: StructureHeading): boolean;
  beforeAction(): Promise<void>;
  canEditSource(): boolean;
  notify(message: string, error?: boolean): void;
  stylesChanged(root: string): Promise<void>;
  refreshLibrary?(root: string): Promise<void>;
  syncApplied(root: string): Promise<void>;
}
type WorkspacePanelKind = "data" | "knowledge" | "query";

/** UI composition only: every action uses the document's existing Draft or Core. */
export class WorkspaceTools {
  readonly bar = document.createElement("nav");
  readonly inspector = document.createElement("aside");
  private readonly propertyPanel: DataPanel;
  private readonly queryPanel: QueryPanel;
  private readonly outputPanel: OutputPanel;
  private readonly knowledgeContainer = document.createElement("div");
  private readonly knowledgePanel;
  private readonly libraryTrash;
  private readonly header = document.createElement("header");
  private readonly panelBody = document.createElement("div");
  private library: { root: string; workspaceId?: string } | null = null;
  private active: WorkspacePanelKind | null = null;
  private readonly styles = new Map<string, NativeStyleController>();
  private refreshTimer?: ReturnType<typeof setTimeout>;
  private pendingImport: { plan: KnowledgePlan; context: KnowledgeContext; receipt?: KnowledgeReceipt } | null = null;

  constructor(private readonly host: WorkspaceToolsHost) {
    this.libraryTrash = createLibraryTrashController({ request: api, created: host.created, notify: host.notify, environmentChanged: root => this.invalidateQueryEnvironment(root) });
    this.bar.className = "workspace-tools"; this.bar.setAttribute("aria-label", "笔记工具");
    this.inspector.className = "workspace-inspector"; this.inspector.hidden = true;
    this.header.className = "inspector-header"; this.panelBody.className = "inspector-body";
    this.inspector.append(this.header, this.panelBody);
    const dataHost: PanelHost = {
      canEdit: host.canEditSource,
      request: api, requestBlob: apiBlob, getDocument: host.getDocument, applyPatch: host.applyPatch,
      openDocument: host.openDocument, openResolved: host.openResolved, notify: host.notify,
      environmentChanged: root => this.invalidateQueryEnvironment(root),
      savedDefinition: host.savedDefinition,
      applyOperationReceipt: (receipt, basis) => host.applyReceipt(receipt as KnowledgeReceipt, { ...basis, dirty: basis.dirty ?? false }),
      prepareAction: host.beforeAction,
      openTemplates: async () => {
        await this.show("knowledge");
        this.knowledgePanel.selectTab("templates");
      },
      created: async result => {
        const opened = result as OpenedDocument;
        if (!opened || typeof opened.sessionId !== "string" || typeof opened.source !== "string") throw new Error("新笔记缺少实际打开结果，请检查原操作。");
        await host.created(opened);
      },
    };
    this.propertyPanel = new DataPanel(dataHost); this.outputPanel = new OutputPanel({ ...dataHost, embeddedQueries: host.embeddedQueries, imported: async receipt => { const root = receipt.root ?? dataHost.getDocument()?.root; if (!root) throw new Error("导入回执缺少可刷新资料库位置，实际回执已保留。"); await host.refreshLibrary?.(root); if (this.library?.root === root) await this.queryPanel.refresh(); } }); this.queryPanel = new QueryPanel(dataHost, query => this.outputPanel.open(query));
    const latex = new LatexInsertionPanel({ ...dataHost, selection: host.selection, editable: host.canEditSource });
    const diagrams = new DiagramPanel(dataHost, host.canEditSource);
    this.knowledgePanel = mountKnowledgePanel(this.knowledgeContainer, {
      context: () => this.context(), load: context => api("/api/knowledge/read", { context }),
      proofread: async (context, options) => {
        const render = await api<{ ok: boolean }>("/api/render", { sessionId: context.sessionId, source: context.source, serial: context.serial });
        if (!render.ok) throw new Error("当前正文编译未成功，请先处理预览诊断再校对。");
        return api("/api/knowledge/proofread", { context, options });
      },
      processingDefaults: async () => {
        const root = this.library?.root; if (!root) throw new Error("请先打开笔记库。");
        const suggestion = await api<ProcessingRead>("/api/processing/read", { root });
        if (this.library?.root !== root) throw new Error("资料库已变化，请重新读取共享建议。");
        return { processing: suggestion.processing, source: suggestion.source, baseId: suggestion.baseId };
      },
      prepare: (intent, context) => api("/api/knowledge/prepare", { operationId: operationId(), context, intent }),
      commit: (plan, context) => this.commit(plan, context),
      navigate: host.navigate, notify: host.notify, beforeAction: host.beforeAction, canEditSource: host.canEditSource,
      inspectOperation: async (id, context) => { const receipt = await api<KnowledgeReceipt>("/api/operations/status", { operationId: id, context }); return deliverCommitted(receipt, value => host.applyReceipt(value, context)); },
      selectSource: host.selectSource,
      revealHeading: host.revealHeading,
      previewTemplate: (id, parameters, context) => api("/api/knowledge/template-preview", { templateId: id, parameters, context }),
      linkCopy: (link, form, context) => api("/api/knowledge/link-copy", { occurrence: link.occurrence, form, context }),
      mountMedia: (container, selected, initial) => mountMediaViewer(container, {
        context: () => this.context(), list: sessionId => api("/api/resources/list", { sessionId }),
        read: (sessionId, resource, revision) => apiBlob("/api/resources/read", { sessionId, resource, revision }),
        importResource: (file, sessionId) => this.importResource(file, sessionId), notify: host.notify,
      }, selected, initial),
    });
    this.bar.append(this.button("属性", () => this.toggle("data")), this.button("链接与批注", () => this.toggle("knowledge")), this.button("查询与视图", () => this.toggle("query")), this.button("输出与导入", () => this.outputPanel.open()), this.button("库样式", () => this.showStyles()), this.button("回收站", async () => { if (!this.library?.workspaceId) throw new Error("请先打开笔记库。"); await this.host.beforeAction(); await this.libraryTrash.open(this.library.root); }));
    this.bar.append(this.button("同步", async () => { await this.host.beforeAction(); await openSyncPanel(this.library?.root ?? "", { beforeAction: this.host.beforeAction, applied: this.host.syncApplied }); }));
    this.bar.append(this.button("LaTeX 公式", () => latex.open()), this.button("Mermaid 图表", () => diagrams.open()), this.button("处理默认与词表", () => this.showProcessingSettings()));
  }
  private async showProcessingSettings(): Promise<void> {
    await this.host.beforeAction();
    const root = this.library?.root; if (!root) throw new Error("请先打开笔记库。");
    await openProcessingSettingsPanel({
      persistenceKey: `weftext-processing:${root}`,
      read: () => api<ProcessingRead>("/api/processing/read", { root }),
      prepare: request => api("/api/processing/prepare", { root, ...request }),
      status: request => api("/api/processing/status", { root, ...request }),
      settle: request => api("/api/processing/settle", { root, ...request }),
      apply: request => api("/api/processing/apply", { request: { root, ...request.request }, planId: request.planId }),
      committed: () => this.invalidateQueryEnvironment(root),
      onApplied: () => { this.host.notify("共享处理默认已提交；各次导入与校对仍须明确采用建议。"); },
    });
  }
  private button(label: string, run: () => void | Promise<void>): HTMLButtonElement {
    const button = document.createElement("button"); button.type = "button"; button.className = "text-button"; button.textContent = label;
    button.onclick = () => { void Promise.resolve(run()).catch(error => this.host.notify(error instanceof Error ? error.message : String(error), true)); }; return button;
  }
  private context(): KnowledgeContext | null { const doc = this.host.getDocument(); return doc ? { ...doc, selection: this.host.selection() } : null; }
  setLibrary(library: { root: string; workspaceId?: string } | null, scope: string | null = null): void {
    this.library = library; this.queryPanel.setLibrary(library); this.queryPanel.setScope(scope);
  }
  invalidateQueryEnvironment(root = this.library?.root): void { if (root) this.queryPanel.environmentChanged(root); }
  async search(text: string, includeBody: boolean): Promise<void> {
    this.queryPanel.setSearch(text, includeBody);
    await this.show("query");
  }
  openQueryOutput(query: FrozenQuery): void { this.outputPanel.open(query); }
  openResource(documentBasis: OpenedDocument, target: ResourceOpenTarget): void {
    const dialog = document.createElement("dialog"); dialog.className = "modal";
    const title = document.createElement("h2"); title.textContent = "笔记资源";
    const container = document.createElement("div");
    // A read-only resource view owns the exact resolved session, independently
    // of a conflicting editor Draft or a later active-tab change.
    const controller = mountResourceViewer(container, {
      context: () => ({ sessionId: documentBasis.sessionId }),
      list: sessionId => api("/api/resources/list", { sessionId }),
      read: (sessionId, resource, revision) => apiBlob("/api/resources/read", { sessionId, resource, revision }),
      notify: this.host.notify,
    }, target);
    const close = (): void => { controller.dispose(); dialog.close(); dialog.remove(); };
    dialog.append(title, container, this.button("关闭", close));
    dialog.oncancel = event => { event.preventDefault(); close(); };
    document.body.append(dialog); dialog.showModal();
  }
  /** Opening is idempotent; a retained active panel must never turn this into
   * closing after navigation or a repeated command. */
  async show(kind: WorkspacePanelKind): Promise<void> {
    await this.host.beforeAction();
    await this.showPanel(kind);
  }
  /** Only toolbar switches use toggle; explicit open actions always use show. */
  private async toggle(kind: WorkspacePanelKind): Promise<void> {
    await this.host.beforeAction();
    if (this.active === kind) { this.active = null; this.inspector.hidden = true; return; }
    await this.showPanel(kind);
  }
  private async showPanel(kind: WorkspacePanelKind): Promise<void> {
    this.active = kind; this.inspector.hidden = false; this.inspector.classList.toggle("wide", kind === "query");
    const title = document.createElement("strong"); title.textContent = { data: "笔记属性", knowledge: "链接与批注", query: "查询与视图" }[kind];
    this.header.replaceChildren(title, this.button("关闭", () => { this.active = null; this.inspector.hidden = true; }));
    this.panelBody.replaceChildren(kind === "data" ? this.propertyPanel.element : kind === "query" ? this.queryPanel.element : this.knowledgeContainer);
    if (kind === "data") await this.propertyPanel.refresh();
    else if (kind === "query") await this.queryPanel.refresh();
    else await this.knowledgePanel.refresh();
  }
  selectKnowledgeTab(tab: import("./knowledge-panel").KnowledgePanelTab): void { this.knowledgePanel.selectTab(tab); }
  documentChanged(): void {
    clearTimeout(this.refreshTimer);
    this.knowledgePanel.attachSurface(null);
    // A known moved writer has no current consumer context until explicit
    // comparison. This retires snapshots/late delivery without reading paths.
    if (!this.host.getDocument()) void this.knowledgePanel.refresh().catch(error => this.host.notify(String(error), true));
    this.refreshTimer = setTimeout(() => {
      if (this.active === "data" && !this.inspector.contains(document.activeElement)) void this.propertyPanel.refresh().catch(error => this.host.notify(String(error), true));
      if (this.active === "query" && !this.inspector.contains(document.activeElement)) this.queryPanel.documentChanged();
    }, 450);
  }
  async refreshAfterSync(root: string): Promise<void> {
    clearTimeout(this.refreshTimer);
    const sameRoot = (value?: string): boolean => value?.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase() === root.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
    // Query results depend on the library, even without an open source. The
    // panel retains its current plan/parameters; frozen outputs stay frozen.
    if (sameRoot(this.library?.root)) await this.queryPanel.refresh();
    if (!this.host.getDocument()) {
      if (this.active === "data") await this.propertyPanel.refresh();
      await this.knowledgePanel.refresh(); return;
    }
    if (!sameRoot(this.host.getDocument()?.root)) return;
    if (this.active === "data") await this.propertyPanel.refresh();
    // Knowledge refresh also renews the selected media/annotation catalog.
    // Its own input guard retains an actively edited review form.
    await this.knowledgePanel.refresh();
  }
  async projectionChanged(surface: HTMLElement | null, mappings: KnowledgeTextMapping[]): Promise<void> {
    this.knowledgePanel.attachSurface(surface, mappings);
    // Knowledge discovery observes the projection; it must never delay body
    // installation or an unrelated property/navigation action. Default-off
    // hints also avoid compiling other notes while this panel is closed.
    if (this.host.getDocument()?.nodeId && (this.active === "knowledge" || this.knowledgePanel.backgroundEnabled())) void this.knowledgePanel.refresh().catch(error => this.host.notify(String(error), true));
  }
  attachSurface(surface: HTMLElement | null, mappings: KnowledgeTextMapping[] = []): void { this.knowledgePanel.attachSurface(surface, mappings); }
  private async importResource(file: File, sessionId: string): Promise<void> {
    if (this.pendingImport) {
      const pending = this.pendingImport;
      const status = await api<KnowledgeReceipt>("/api/operations/status", { operationId: pending.plan.operationId, context: pending.context });
      if (pending.receipt && originalCommitted(pending.receipt) && !originalCommitted(status)) throw new Error("原导入已确认提交，后续状态不一致；原回执与编号保留。");
      pending.receipt = status;
      if (originalCommitted(status)) {
        const delivered = await deliverCommitted(status, value => this.host.applyReceipt(value, pending.context));
        if (!committedDeliveryComplete(delivered)) throw new UiError(() => originalDeliveryErrors(delivered).map(uiTextValue).join("\n") || "原资源导入已提交，交付尚未完成。");
        this.pendingImport = null; this.host.notify("已确认上一次资源导入成功。"); return;
      }
      if (status.status === "resolved") { this.pendingImport = null; throw new Error("原导入已明确终结，原文件仍保留。请重新选择后准备。"); }
      throw new Error("原导入尚未完成，请继续查询原操作或处理其恢复状态。");
    }
    if (file.size > 64 * 1024 * 1024) throw new Error("单个资源导入上限为 64 MiB，原文件保持不变。");
    await this.host.beforeAction();
    const context = this.context(); if (!context || context.sessionId !== sessionId) throw new Error("当前笔记已变化，请重新选择资源。");
    const array = new Uint8Array(await file.arrayBuffer()); let binary = "";
    for (let index = 0; index < array.length; index += 8192) binary += String.fromCharCode(...array.subarray(index, index + 8192));
    const plan = await api<KnowledgePlan>("/api/resources/prepare-import", { sessionId, name: file.name, bytes: btoa(binary), operationId: operationId() });
    const dialog = document.createElement("dialog"); dialog.className = "modal";
    const title = document.createElement("h2"); title.textContent = "导入资源";
    const description = document.createElement("p"); description.textContent = `将 ${file.name}（${file.size.toLocaleString()} 字节）复制到当前笔记。原文件保留。`;
    const list = document.createElement("ul");
    for (const change of plan.changes ?? []) { const item = document.createElement("li"); item.textContent = change.path ?? "当前笔记的资源清单"; list.append(item); }
    dialog.append(title, description, list); document.body.append(dialog);
    const accepted = await new Promise<boolean>(resolve => {
      const finish = (result: boolean): void => { dialog.close(); dialog.remove(); resolve(result); };
      dialog.append(this.button("取消", () => finish(false)), this.button("导入", () => finish(true)));
      dialog.oncancel = event => { event.preventDefault(); finish(false); }; dialog.showModal();
    });
    if (!accepted) return;
    this.pendingImport = { plan, context };
    try { const result = await this.commit(plan, context); this.pendingImport.receipt = result; if (committedDeliveryComplete(result)) this.pendingImport = null; else throw new UiError(() => originalDeliveryErrors(result).map(uiTextValue).join("\n") || "原导入结果仍待核对。"); }
    catch (error) { this.host.notify("资源导入结果待确认。再次导入会先检查原操作。", true); throw error; }
  }
  private async commit(plan: KnowledgePlan, context: KnowledgeContext): Promise<KnowledgeReceipt> {
    if (!sameKnowledgeBasis(context, this.context())) throw new Error("草稿已变化，请重新准备。");
    if (plan.draftOnly === true) {
      const edit = plan.edit as SourcePatch | undefined;
      if (!edit || typeof edit.from !== "number" || typeof edit.to !== "number" || typeof edit.text !== "string" || typeof plan.source !== "string") throw new Error("链接提案没有完整的准确源码变更。");
      if (context.source.slice(0, edit.from) + edit.text + context.source.slice(edit.to) !== plan.source) throw new Error("链接提案与预览不一致。");
      await this.host.applyPatch(edit, context);
      return { status: "committed", operationId: plan.operationId, message: "链接已更新到当前草稿，可撤销。" };
    }
    const receipt = await api<KnowledgeReceipt>("/api/operations/commit", { operationId: plan.operationId, planId: plan.planId, context });
    const delivered = await deliverCommitted(receipt, value => this.host.applyReceipt(value, context));
    if (!committedDeliveryComplete(delivered)) this.host.notify("原操作已提交；界面更新尚未完成，请检查原操作。", true);
    return delivered;
  }
  private async showStyles(): Promise<void> {
    const library = this.library;
    if (!library?.workspaceId) throw new Error("请先打开资料库，再设置库默认样式。");
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
  }
}
