import { SourceEditor, sourceBoundary, type SourcePatch } from "./editor";
import { everydayWritingCommands } from "./everyday-writing";
import type { EverydayCommand } from "./everyday-navigation";
import { EmbeddedQueryController, sameQueryBasis, type EmbeddedQueryBasis } from "./embedded-query";
import type { QueryPlan, QueryResult, QueryRow } from "./query-panel";
import type { FrozenQuery } from "./export-state";
import { WritingController, groupWritingRenderNodes, groupWritingEmptySlots, type WritingMap, type WritingRange } from "./writing";
import { createWritingToolbar, createStructureControls, insertWritingLink } from "./writing-controls";
import { createMathSelectionControls } from "./math-selection";
import { mountDocumentNotes, openDocumentNotes, retireDocumentNotes, type DocumentNotesMount } from "./document-notes";
import { AuthoringAssetsAdapter } from "./authoring-assets-adapter";
import { documentNotesHost, numberingHost, personalNumberingKeys, personalNumberingVisible, markPersonalHeading, displayPersonalHeadings, type AuthoringPanelsEnvironment } from "./authoring-panels-adapter";
import { openNumberingPanel } from "./numbering-panel";
import "./document-notes.css";
import { nativeElement } from "./native-node";
import { OfflineFontLoader } from "./font-readiness";
import type { Origin, RenderNode, SyntaxToken, ViewMode } from "./types";
import "./writing.css";
import "./hosted-editor.css";
import "./embedded-query.css";
import type { HostedQueryDependencies } from "./hosted-query-state";

interface Projection { ok: boolean; tree: RenderNode | null; source: string; serial: number; message: string; sourceInfo: { syntaxTokens: SyntaxToken[]; notes?: import("./document-notes").NotesSourceMap } }
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedEditorQueryHost {
  openRow(row: QueryRow, plan: QueryPlan | null): Promise<void>;
  canOpenRow(row: QueryRow, plan: QueryPlan | null): boolean;
  openOwner(origin: Origin): Promise<void>;
  output(query: FrozenQuery, result: QueryResult): void | Promise<void>;
  modeChanged?(): void;
  navigateReference?(reference: string): Promise<void>;
  navigateResource?(reference: string): Promise<void>;
  dependencies?(): HostedQueryDependencies;
  noteInputAudience?(): string | null;
  pendingNoteInput?(audience: string, value: string): void;
}
const modes: [ViewMode, string][] = [["source", "V01 源码"], ["styled", "V02 带样式源码"], ["focus", "V03 按需语法"], ["write", "V04 可视写作"], ["read", "V05 阅读"]];
function node<K extends keyof HTMLElementTagNameMap>(tag: K, text = "", name = ""): HTMLElementTagNameMap[K] { const el = document.createElement(tag); el.textContent = text; el.className = name; return el; }

/** One actual buffer and undo history; remote endpoints propose native patches. */
export class HostedEditor {
  private editor?: SourceEditor;
  private writing?: WritingController;
  private sessionId = "";
  private resource = "";
  private blocked = true;
  private paused = false;
  private mode: ViewMode = "styled";
  private projection?: Projection;
  private generation = 0;
  private timer?: ReturnType<typeof setTimeout>;
  private toolbar = node("div");
  private sourceHost = node("div", "", "hosted-source");
  private flow = node("div", "", "hosted-flow");
  private message = node("p", "", "hint");
  private fonts: OfflineFontLoader;
  private renderedMode?: ViewMode;
  private renderedBlocked?: boolean;
  private queries?: EmbeddedQueryController;
  private queryPreview = false;
  private notesMount?: DocumentNotesMount;
  private authoringAssets: AuthoringAssetsAdapter;
  private personalKeys?: ReturnType<typeof personalNumberingKeys>;
  private personalAudience = "";
  private retained = new Map<string, { editor: SourceEditor; writing: WritingController; queries?: EmbeddedQueryController; mode: ViewMode; scroll: number; sourceScroll: number; windowScroll:number }>();
  private restoredView?: { resource:string; scroll:number; sourceScroll:number; windowScroll:number };
  constructor(private container: HTMLElement, private request: Request, private changed: () => void, private save: () => void, private error: (e: Error) => void, private queryHost?: HostedEditorQueryHost) {
    this.fonts = new OfflineFontLoader({ fonts: document.fonts, readManifest: () => request("fonts") });
    this.authoringAssets = new AuthoringAssetsAdapter({ request, prefix: "editor/endnotes-module", context: () => this.sessionId && !this.paused && !this.blocked && this.mode !== "read" && this.personalAudience ? { sessionId: this.sessionId, audience: this.personalAudience } : null, changed: async () => { this.projection = undefined; this.queries?.invalidate(); this.generation++; await this.compile(); } });
    container.classList.add("hosted-editor");
    this.message.setAttribute("role", "status");
    container.addEventListener("keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || this.composing || this.blocked || this.paused || this.mode !== "write" || !(event.ctrlKey || event.metaKey) || !["z", "y"].includes(event.key.toLowerCase()) || event.target instanceof Element && event.target.closest("input,textarea,[contenteditable=true],dialog,.embedded-query-builder,.embedded-query-parameters")) return;
      const writing = this.writing; if (!writing) return;
      event.preventDefault(); const redo = event.shiftKey || event.key.toLowerCase() === "y";
      void (redo ? writing.redo() : writing.undo()).catch(this.error);
    });
  }
  get value(): string { return this.writing?.recoverySource() ?? this.editor?.source ?? ""; }
  get serial(): number { return this.editor?.generation ?? 0; }
  get viewMode(): ViewMode { return this.mode; }
  get selection(): { from: number; to: number } | undefined { return this.editor?.selection; }
  get pausedSession(): boolean { return this.paused; }
  get composing(): boolean { return !!this.editor?.composing || !!this.writing?.state.composing; }
  checkNavigationCapacity(resource:string): void { if(this.resource!==resource&&!this.retained.has(resource)&&this.retained.size>=15)throw new Error("本页已保留 16 个正文编辑会话及其 Undo。请先保存所需正文，再刷新页面释放会话；个人恢复稿仍由原缓存保留。"); }
  commands(insertQuery?: () => Promise<void>): EverydayCommand[] { return everydayWritingCommands(this.writing, !this.blocked && !this.paused && this.mode !== "read" && !this.composing, insertQuery); }
  pauseSession(): void { clearTimeout(this.timer); this.generation++; this.paused = true; this.sessionId = ""; this.projection = undefined; this.authoringAssets.invalidate(); if (this.writing) retireDocumentNotes(this.writing); this.queries?.invalidate(); this.mode = "source"; this.editor?.clearTokens(); this.editor?.readOnly(this.blocked); this.paint(true); this.message.textContent = "原对象的结构会话已暂停；当前草稿与 Undo 保留，请明确打开新位置核对后继续。"; }
  rebindResourceAlias(resource: string): void { if (!this.editor || !this.paused) throw new Error("原对象编辑器未暂停，不能重绑未经核对的别名。"); this.resource = resource; }
  select(from: number, to: number): void { this.editor?.select(from, to); }
  selectSourceRange(from: number, to: number): void {
    if (!this.editor || this.writing?.state.composing || !Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to < from || to > this.value.length || !sourceBoundary(this.value, from) || !sourceBoundary(this.value, to)) throw new Error("源码出现项没有当前准确范围，未采用旧位置。");
    this.mode = "source"; this.editor.clearReveal(); this.editor.readOnly(this.blocked); this.paint(true); this.editor.select(from, to); this.queryHost?.modeChanged?.();
  }
  locateNodeFragment(origin: Origin, basis: EmbeddedQueryBasis): void {
    if (!this.editor || this.writing?.state.composing || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis) || origin.isMain !== true || origin.path !== `resource:${this.resource}` || !sourceBoundary(basis.source, origin.utf16From) || !sourceBoundary(basis.source, origin.utf16To) || basis.source.slice(origin.utf16From, origin.utf16To) !== origin.raw) throw new Error("定位期间目标草稿已变化，未采用旧标签范围。");
    // Selecting the actual native author range changes neither source nor Undo.
    this.mode = "source"; this.editor.clearReveal(); this.editor.readOnly(this.blocked); this.paint(true); this.editor.select(origin.utf16From, origin.utf16To); this.queryHost?.modeChanged?.();
  }
  set disabled(value: boolean) { this.editor?.readOnly(value || this.mode === "read" || this.paused && this.mode !== "source"); if (this.blocked === value) return; if (value) this.editor?.clearTokens(); this.blocked = value; this.paint(); }
  async flush(): Promise<void> { await this.writing?.settleForSessionChange(); }
  async historyBasis(settle = false): Promise<EmbeddedQueryBasis> {
    if (settle) await this.flush();
    const basis = { sessionId: this.sessionId, source: this.value, serial: this.serial };
    if (!this.editor || !basis.sessionId) throw new Error("当前正文会话已变化，未登记旧稿。");
    await this.request("editor/map", basis);
    if (!this.editor || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis)) throw new Error("草稿在读取期间已变化，输入保留，请重新核对。");
    return basis;
  }
  applyHistorySource(value: string, basis: EmbeddedQueryBasis): void {
    if (!this.editor || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis)) throw new Error("恢复期间的新输入已保留，未替换当前草稿。");
    this.editor.applyPatches(basis.source, [{ from: 0, to: basis.source.length, expected: basis.source, insert: value }], undefined, "input.history.restore"); this.schedule(0);
  }
  refreshQueries(): void { if (!this.editor) return; this.queries?.invalidate(); this.schedule(0); }
  refreshEnvironment(): void { if (!this.editor) return; this.projection = undefined; this.editor.clearTokens(); this.queryPreview = false; this.queries?.invalidate(); this.message.textContent = "库配置依据已变化，正在重新编译当前草稿；正文与撤销历史保留。"; this.schedule(0); }
  queryInputsChanged(): void { this.queries?.runInputsChanged(); }
  outputSelections(): { occurrenceId: string; queryId: string }[] { return this.queries?.outputSelections() ?? []; }
  async prepareQueryBasis(): Promise<EmbeddedQueryBasis> {
    await this.flush();
    const basis = { sessionId: this.sessionId, source: this.value, serial: this.serial };
    if (!this.editor || this.blocked || this.mode === "read" || this.writing?.state.composing) throw new Error("请在当前可写正文中提出保存定义，尚未发送提案。");
    await this.request("editor/map", basis);
    if (!sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis) || this.blocked || this.viewMode === "read") throw new Error("登记期间草稿或编辑资格已变化，请重新核对当前计划。");
    return basis;
  }
  open(sessionId: string, resource: string, source: string): void {
    this.checkNavigationCapacity(resource);
    this.paused = false;
    // Reopening the same retained draft changes its Base/session, not Undo.
    if (this.resource !== resource || !this.editor) {
      if (this.editor && this.writing && this.resource) {
        this.queries?.invalidate(); this.queries?.detach(); retireDocumentNotes(this.writing);
        this.retained.set(this.resource, { editor:this.editor, writing:this.writing, queries:this.queries, mode:this.mode, scroll:this.flow.scrollTop, sourceScroll:this.editor.view.scrollDOM.scrollTop, windowScroll:window.scrollY });
        this.editor = undefined; this.writing = undefined; this.queries = undefined;
      }
      this.clear(false); this.resource = resource;
      const saved = this.retained.get(resource); this.retained.delete(resource);
      if (saved && saved.editor.source === source) {
        this.editor = saved.editor; this.writing = saved.writing; this.queries = saved.queries; this.mode = saved.mode;
        this.restoredView={resource,scroll:saved.scroll,sourceScroll:saved.sourceScroll,windowScroll:saved.windowScroll};
        if(this.mode==="write")saved.writing.restoreNavigationFocus();
      } else {
      if (saved) { saved.queries?.dispose(); saved.writing.dispose(); saved.editor.destroy(); }
      this.editor = new SourceEditor(source, () => { this.queries?.invalidate(); this.changed(); this.schedule(); }, this.save);
      const editor = this.editor;
      let retained = false;
      this.writing = new WritingController({ editor,
        refreshMap: basis => this.request("editor/map",basis),
        basis: () => ({ sessionId: this.sessionId, source: editor.source, serial: editor.generation }),
        editable: () => this.editor === editor && !this.blocked && this.mode !== "read",
        request: request => this.requestEdit(request), onError: this.error,
        onPendingSource: () => { this.queries?.invalidate(); this.changed(); }, onApplied: () => this.schedule(50),
        onState: () => {
          const next = this.writing?.retainInput ?? false;
          if (retained && !next && this.mode === "write") queueMicrotask(() => { if (this.editor === editor) this.paint(); });
          retained = next;
        },
      });
      if (this.queryHost) this.queries = new EmbeddedQueryController({
        catalog: async () => (await this.request<{ formSchema?: import("./data-panel").DataSchema }>("query/catalog")).formSchema ?? null,
        basis: () => ({ sessionId: this.sessionId, source: this.value, serial: this.serial }), mode: () => this.mode,
        editable: () => this.editor === editor && !this.blocked && this.mode !== "read" && !this.writing?.state.composing,
        request: (path, body) => {
          const routes: Record<string, string> = { "/api/query/embedded/discover": "query/embedded/discover", "/api/query/embedded/run": "query/embedded/run", "/api/query/embedded/edit": "query/embedded/edit" };
          if (!routes[path]) throw new Error("托管查询没有此读取入口。"); const dependencies = path === "/api/query/embedded/run" ? this.queryHost?.dependencies?.() ?? { providers: {}, seriesPolicies: {} } : undefined; return this.request(routes[path], dependencies ? { ...(body as object), ...dependencies } : body);
        },
        prepareEdit: () => this.flush(), apply: async (patches, basis, event) => this.applyQueryPatches(patches, basis, event),
        reveal: (item, host) => {
          if (this.editor !== editor || this.mode !== "focus" || this.blocked) return false;
          if (!editor.reveal({ from: item.range.utf16From, to: item.range.utf16To, raw: item.range.raw }, this.value)) return false;
          editor.readOnly(false); editor.mount(host); return true;
        },
        closeReveal: () => editor.clearReveal(), openRow: this.queryHost.openRow, canOpenRow: this.queryHost.canOpenRow, openResource: this.queryHost.navigateResource,
        openOwner: this.queryHost.openOwner, canOpenOwner: () => false, output: this.queryHost.output, notify: message => this.error(new Error(message)),
      });
      }
    } else if (this.editor.source !== source) {
      this.editor.apply(0, this.editor.source.length, source);
    }
    this.sessionId = sessionId; this.projection = undefined; this.generation++; this.editor.clearTokens();
    this.editor.clearReveal(); this.editor.readOnly(this.blocked || this.mode === "read"); this.paint(); this.schedule(0);
    void this.preparePersonalNumbering().catch(() => { /* The explicit panel reports a current identity failure. */ });
  }
  /** Match the exact editor that open() would retain, without installing it or
   * lending the current note's serial to another target. Server qualification
   * still binds the new session, source hash and actual compile separately. */
  retainedSourceSerial(resource: string, source: string): number {
    const editor = this.resource === resource ? this.editor : this.retained.get(resource)?.editor;
    return editor?.source === source ? editor.generation : 0;
  }
  clear(clearRetained = true): void {
    if (clearRetained) { for (const saved of this.retained.values()) { saved.queries?.dispose(); saved.writing.dispose(); saved.editor.destroy(); } this.retained.clear(); }
    this.restoredView=undefined; this.authoringAssets.invalidate(); this.notesMount?.dispose(); this.notesMount = undefined; this.personalKeys = undefined; this.personalAudience = "";
    this.sessionId = ""; this.projection = undefined; if (this.writing) retireDocumentNotes(this.writing);
    clearTimeout(this.timer); this.generation++; this.queries?.dispose(); this.queries = undefined; this.queryPreview = false; this.writing?.dispose(); this.editor?.destroy();
    this.writing = undefined; this.editor = undefined; this.projection = undefined; this.resource = ""; this.sessionId = "";
    this.blocked = true; this.container.replaceChildren();
    this.paused = false;
    this.sourceHost.replaceChildren(); this.flow.replaceChildren(); this.toolbar.replaceChildren(); this.message.textContent = "";
    // Shared structure dialogs may contain the just-revoked source. Dispose
    // both their visible data and their pending controller capability.
    document.querySelectorAll<HTMLDialogElement>('.writing-dialog,dialog[aria-label="当前笔记编号控制"]').forEach(dialog => { dialog.close(); dialog.remove(); });
  }
  private async preparePersonalNumbering(): Promise<ReturnType<typeof personalNumberingKeys>> {
    const sessionId = this.sessionId, editor = this.editor, resource = this.resource;
    if (!sessionId || !editor || this.paused) throw new Error("当前没有稳定的正文会话，未选择个人编号偏好。");
    const [identity, reference] = await Promise.all([this.request<{ actor: string; deployment: string }>("me"), this.request<{ nodeRef: { workspaceId: string; nodeId: string } }>("navigation/ref", { sessionId })]);
    if (this.editor !== editor || this.sessionId !== sessionId || this.resource !== resource || this.paused || !identity.actor || !identity.deployment || !reference.nodeRef?.workspaceId || !reference.nodeRef.nodeId) throw new Error("个人编号读取期间笔记或账号已变化，未沿用旧偏好。");
    this.personalAudience = JSON.stringify([identity.deployment, identity.actor]); this.personalKeys = personalNumberingKeys(this.personalAudience, reference.nodeRef.workspaceId, reference.nodeRef.nodeId);
    displayPersonalHeadings(this.flow, personalNumberingVisible(this.personalKeys)); return this.personalKeys;
  }
  private authoringEnvironment(): AuthoringPanelsEnvironment {
    const writing = this.writing, editor = this.editor; if (!writing || !editor) throw new Error("当前正文尚未打开。");
    const inputAudience = this.queryHost?.noteInputAudience?.() ?? this.personalAudience;
    const readable = () => this.editor === editor && this.writing === writing && !!this.sessionId && !this.paused;
    return {
      controller: writing, readable, editable: () => readable() && !this.blocked && this.mode !== "read",
      projection: () => readable() && this.current() ? this.projection ?? null : null, selection: () => editor.selection,
      requestEdit: request => this.requestEdit(request), previewProposal: request => this.request("editor/preview", request),
      revealSource: range => this.selectSourceRange(range.utf16From, range.utf16To), revealRendered: key => { if (!readable() || !this.flow.isConnected || !this.flow.getClientRects().length) return false; const target = Array.from(this.flow.querySelectorAll<HTMLElement>("[data-document-node-key]")).find(item => item.dataset.documentNodeKey === key); if (!target) return false; target.scrollIntoView({ block: "center", inline: "nearest" }); target.focus({ preventScroll: true }); return true; },
      ensureEndnoteModule: async () => { await this.preparePersonalNumbering(); await this.authoringAssets.install(); this.message.textContent = "本篇尾注模块资源已核对；正文与撤销记录保留，请重新打开尾注提案。"; },
      recoverEndnoteModule: async () => { await this.preparePersonalNumbering(); const receipt = await this.authoringAssets.recover(); this.message.textContent = receipt ? `原尾注模块操作：${receipt.status}` : "当前没有待核对的模块安装。"; },
      retryEndnoteModule: async () => { await this.preparePersonalNumbering(); await this.authoringAssets.retryOriginal(); this.message.textContent = "原尾注模块安装已确认，正文尚未保存。"; },
      restartEndnoteModule: async () => { await this.preparePersonalNumbering(); await this.authoringAssets.restartAfterCancelled(); this.message.textContent = "已核对原操作明确取消并准备新模块资源；正文尚未保存。"; },
      settleEndnoteModule: async () => { await this.preparePersonalNumbering(); const receipt = await this.authoringAssets.settleOriginal(); this.message.textContent = `原模块操作：${receipt.status}`; },
      pendingInput: value => { if (inputAudience) this.queryHost?.pendingNoteInput?.(inputAudience, value); }, onError: this.error,
    };
  }
  private async openNumbering(): Promise<void> {
    const writing = this.writing; if (!writing) return; await writing.deferUntilCompositionEnds(() => writing.flush());
    const keys = await this.preparePersonalNumbering(); if (this.writing !== writing) throw new Error("正文会话已变化，未打开旧编号面板。");
    await openNumberingPanel(numberingHost(this.authoringEnvironment(), { ...keys, display: enabled => displayPersonalHeadings(this.flow, enabled) }));
  }
  private schedule(delay = 250): void { clearTimeout(this.timer); this.generation++; this.timer = setTimeout(() => { void this.compile(); }, delay); }
  private async requestEdit(request: { sessionId: string; source: string; serial: number; sourceHash: string; intent: import("./writing").WritingIntent }): Promise<import("./writing").WritingResult> {
    const editor = this.editor;
    const current = () => !!editor && this.editor === editor && !this.blocked && !this.paused && this.mode !== "read" && this.sessionId === request.sessionId && editor.source === request.source && editor.generation === request.serial;
    if (!current()) throw new Error("编辑依据已变化，当前输入保留，未登记旧提案。");
    // A previous native result installs its map immediately. Register that exact
    // new Draft before the next proposal instead of waiting for a debounced
    // preview to advance the server's observed serial.
    await this.request("editor/map", { sessionId: request.sessionId, source: request.source, serial: request.serial, sourceHash: request.sourceHash });
    if (!current()) throw new Error("登记期间草稿或编辑资格已变化，未发送旧编辑提案。");
    return this.request("editor/edit", request);
  }
  async applyQueryPatches(patches: SourcePatch[], basis: EmbeddedQueryBasis, event = "input.query.proposal"): Promise<void> {
    if (!this.editor || this.blocked || this.mode === "read" || this.writing?.state.composing || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis)) throw new Error("查询提案的草稿或编辑资格已变化，输入保留，未覆盖正文。");
    this.editor.applyPatches(basis.source, patches, undefined, event); this.schedule(0);
  }
  private async compile(): Promise<void> {
    const editor = this.editor, writing = this.writing; if (!editor || !writing || !this.sessionId) return;
    const basis = { sessionId: this.sessionId, source: editor.source, serial: editor.generation }, generation = this.generation, syntaxBasis = editor.syntaxBasis();
    this.message.textContent = "正在更新当前草稿预览…";
    try {
      const [map, projection] = await Promise.all([this.request<WritingMap>("editor/map", basis), this.request<Projection>("editor/render", basis), this.fonts.ensureDefaults()]);
      if (this.editor !== editor || this.generation !== generation || this.sessionId !== basis.sessionId || editor.source !== basis.source || editor.generation !== basis.serial) return;
      if (projection.source !== basis.source || projection.serial !== basis.serial) throw new Error("预览返回的源码或编辑代次不符，未安装旧样式或编辑范围。");
      if (this.queries) {
        try { await this.queries.discover(basis); } catch (error) { if (this.editor === editor && this.generation === generation) this.error(error instanceof Error ? error : new Error(String(error))); }
        if (this.editor !== editor || this.generation !== generation || this.sessionId !== basis.sessionId || editor.source !== basis.source || editor.generation !== basis.serial) return;
      }
      if (projection.sourceInfo.notes?.sourceHash === map.sourceHash) map.notes = projection.sourceInfo.notes;
      this.projection = projection; writing.installMap(map, basis.source, basis.serial);
      editor.tokens(projection.sourceInfo.syntaxTokens, this.mode === "styled", syntaxBasis); this.message.textContent = projection.ok ? "预览对应当前草稿；编辑仍须明确保存。" : projection.message;
      this.paint();
      const restored=this.restoredView;
      if(restored?.resource===this.resource){this.restoredView=undefined;requestAnimationFrame(()=>{if(this.editor!==editor||this.resource!==restored.resource)return;this.flow.scrollTop=restored.scroll;editor.view.scrollDOM.scrollTop=restored.sourceScroll;if(["source","styled"].includes(this.mode))editor.view.focus();window.scrollTo({top:restored.windowScroll,behavior:"instant"});});}
    } catch (error) {
      if (this.editor !== editor || this.generation !== generation) return;
      this.message.textContent = error instanceof Error ? error.message : String(error);
      if ((error as { code?: string }).code === "busy") this.schedule(800);
    }
  }
  private button(label: string, action: () => void | Promise<void>): HTMLButtonElement {
    const button = node("button", label); button.type = "button";
    button.onclick = () => { Promise.resolve().then(action).catch(this.error); }; return button;
  }
  private async changeMode(mode: ViewMode): Promise<void> {
    const writing = this.writing, editor = this.editor; if (!writing || !editor) return;
    if (writing.state.composing) { await writing.deferUntilCompositionEnds(() => this.changeMode(mode)); return; }
    await writing.settleForSessionChange(); if (this.editor !== editor) return;
    editor.clearReveal(); this.mode = mode; editor.readOnly(this.blocked || mode === "read" || this.paused && mode !== "source");
    this.queries?.modeChanged();
    this.queryHost?.modeChanged?.();
    if (this.current()) editor.tokens(this.projection!.sourceInfo.syntaxTokens, mode === "styled", editor.syntaxBasis());
    else if (this.paused || this.blocked) editor.clearTokens();
    editor.styleMode(mode === "styled"); this.paint(true);
  }
  private reveal(origin: Origin | WritingRange): void {
    if (!this.editor || !this.writing || !this.current() || this.mode === "read") return;
    const construct = this.writing.map?.structures.filter(s => s.range.utf16From <= origin.utf16From && s.range.utf16To >= origin.utf16To).sort((a,b) => (a.range.to-a.range.from)-(b.range.to-b.range.from))[0]?.range ?? origin;
    if (this.mode === "focus") { if (!this.editor.reveal({ from: construct.utf16From, to: construct.utf16To, raw: construct.raw })) return; }
    else { this.mode = "source"; this.editor.clearReveal(); }
    this.paint(true); this.editor.select(origin.utf16From, origin.utf16To);
  }
  private current(): boolean { return !!this.editor && this.projection?.source === this.editor.source && this.projection?.serial === this.editor.generation; }
  private paint(force = false): void {
    const editor = this.editor, writing = this.writing; if (!editor || !writing) return;
    if (!force && (writing.retainInput || editor.composing)) return;
    const nav = node("div", "", "hosted-modes"); nav.setAttribute("role", "group"); nav.setAttribute("aria-label", "文档视图");
    for (const [mode, label] of modes) { const b = this.button(label, () => this.changeMode(mode)); b.setAttribute("aria-pressed", String(this.mode === mode)); nav.append(b); }
    if (this.mode === "source" && this.queries?.hasQueries) nav.append(this.button(this.queryPreview ? "收起正文查询预览" : "查看正文查询预览", () => { this.queryPreview = !this.queryPreview; this.paint(true); }));
    nav.append(this.button("更新写作预览", () => this.compile()));
    this.sourceHost.hidden = !["source","styled"].includes(this.mode) && !(this.mode === "focus" && editor.revealed);
    this.flow.hidden = this.mode === "source" && !this.queryPreview;
    if (!this.sourceHost.hidden && editor.view.dom.parentElement !== this.sourceHost) editor.mount(this.sourceHost);
    const options = { onError: this.error, revealSource: (range: WritingRange) => this.reveal(range), insertLink: () => insertWritingLink(writing) };
    this.toolbar.replaceChildren();
    this.notesMount?.dispose(); const notes = node("div", "", "document-notes-summary"); this.toolbar.append(this.button("编号控制", () => this.openNumbering()), notes); this.notesMount = mountDocumentNotes(notes, documentNotesHost(this.authoringEnvironment()));
    if (!this.blocked && this.mode !== "read" && (!this.paused || this.mode === "source")) {
      if (this.mode === "write") this.toolbar.append(createWritingToolbar(writing, options));
      else this.toolbar.append(this.button("撤销", () => writing.undo()), this.button("重做", () => writing.redo()));
    }
    if (this.mode === "focus" && editor.revealed) this.toolbar.append(this.button("收起语法", () => { editor.clearReveal(); this.paint(true); }));
    if (force || !this.container.contains(this.flow) || this.renderedMode !== this.mode || this.renderedBlocked !== this.blocked) {
      const layout = node("div", "", `hosted-editor-body mode-${this.mode}`); layout.append(this.sourceHost, this.flow);
      this.container.replaceChildren(nav, this.toolbar, this.message, layout);
      this.renderedMode = this.mode; this.renderedBlocked = this.blocked;
    }
    this.flow.replaceChildren();
    const projection = this.projection;
    if (!projection?.ok || !projection.tree) { this.flow.append(node("p", projection?.message || "当前草稿预览尚未就绪。可继续编辑源码。")); return; }
    const current = this.current(), writable = current && !this.blocked && this.mode === "write";
    this.queries?.beginProjection(projection.tree, { sessionId: this.sessionId, source: editor.source, serial: editor.generation });
    if (this.mode === "source") {
      const visit = (item: RenderNode): void => { const query = this.queries?.forNode(item); if (query) this.flow.append(query); else for (const child of item.children) visit(child); };
      if (this.queryPreview) { this.flow.append(node("p", "当前完整源码中的查询结果预览；原源码仍在同一编辑器中。", "hint")); visit(projection.tree); }
      return;
    }
    if (!current) this.flow.append(node("p", "显示上次预览；旧范围不能用于可视编辑。", "hint"));
    const groups = current && writing.map ? groupWritingRenderNodes(projection.tree, writing.map, editor.source) : [];
    const grouped = new Map(groups.flatMap(group => group.memberKeys.map(key => [key,group] as const)));
    const empty = new Map((current && writing.map ? groupWritingEmptySlots(projection.tree, writing.map, editor.source) : []).map(slot => [slot.nodeKey,slot.region]));
    const input = (label: string, region: Parameters<WritingController["bindLiteral"]>[1]) => { const el = node("textarea", "", "literal-input"); el.rows = 1; el.spellcheck = false; el.setAttribute("aria-label", label); writing.bindLiteral(el, region); return el; };
    let personalHeadingOrdinal = 0;
    const draw = (item: RenderNode): Node => {
      const query = this.queries?.forNode(item); if (query) return query;
      if (item.kind === "tag") return document.createDocumentFragment();
      if (item.kind === "text") {
        const group = grouped.get(item.key);
        if (writable && group) return group.firstKey === item.key ? input("编辑段落文字", group.region) : document.createDocumentFragment();
        if (this.mode === "focus" && current && item.origin?.isMain) {
          const span = node("span", item.text ?? "", "focus-text"); span.tabIndex = 0; span.setAttribute("role","button"); span.setAttribute("aria-label", `展开源码：${(item.text ?? "").slice(0,30)}`);
          span.onclick = () => this.reveal(item.origin!); span.onkeydown = e => { if (["Enter"," "].includes(e.key)) { e.preventDefault(); this.reveal(item.origin!); } }; return span;
        }
        return document.createTextNode(item.text ?? "");
      }
      if (item.kind === "frame") return node("p", "此布局需分页呈现；可使用独立预览或源码入口。", "hint");
      const el = nativeElement(item);
      el.setAttribute("data-document-node-key", item.key);
      const semanticRole = item.attrs["data-weftext-role"], noteId = item.attrs["data-weftext-note-id"], targetId = item.attrs["data-weftext-target-id"];
      if (semanticRole === "heading") markPersonalHeading(el, ++personalHeadingOrdinal);
      if (current && typeof noteId === "string" && ["footnote-reference", "endnote-reference"].includes(String(semanticRole))) {
        el.setAttribute("role", "button"); el.setAttribute("tabindex", "0"); el.setAttribute("aria-label", `查看${semanticRole === "endnote-reference" ? "尾注" : "脚注"}正文与全部标记`);
        const open = () => openDocumentNotes(documentNotesHost(this.authoringEnvironment()), semanticRole === "endnote-reference" ? "endnote" : "footnote", noteId);
        el.addEventListener("click", event => { event.preventDefault(); event.stopPropagation(); void open().catch(this.error); }, true);
        el.addEventListener("keydown", event => { if (event instanceof KeyboardEvent && !event.isComposing && !writing.state.composing && ["Enter", " "].includes(event.key)) { event.preventDefault(); event.stopPropagation(); void open().catch(this.error); } });
      }
      let openReference: HTMLButtonElement | undefined;
      if (item.tag === "a" && typeof item.attrs.href === "string") {
        el.setAttribute("title", item.attrs.href);
        if (/^https?:\/\//i.test(item.attrs.href)) { el.setAttribute("href", item.attrs.href); el.setAttribute("target", "_blank"); el.setAttribute("rel", "noopener noreferrer"); }
        else if (/^weftext-node:\/\//.test(item.attrs.href) && this.queryHost?.navigateReference) {
          const reference = item.attrs.href; el.setAttribute("href", "#"); el.addEventListener("click", event => { if (event.target instanceof Element && event.target.closest("input,textarea,button,[contenteditable=true],[role=button]")) return; event.preventDefault(); void this.queryHost!.navigateReference!(reference).catch(this.error); });
          if (writable) openReference = this.button("打开稳定链接", () => this.queryHost!.navigateReference!(reference));
        } else if (/^weftext-resource:\/\//.test(item.attrs.href) && this.queryHost?.navigateResource) {
          const reference = item.attrs.href; el.setAttribute("href", "#"); el.addEventListener("click", event => { if (event.target instanceof Element && event.target.closest("input,textarea,button,[contenteditable=true],[role=button]")) return; event.preventDefault(); void this.queryHost!.navigateResource!(reference).catch(this.error); });
          if (writable) openReference = this.button("查看稳定资源链接", () => this.queryHost!.navigateResource!(reference));
        }
      }
      for (const child of item.children ?? []) el.append(draw(child));
      if (item.text) el.append(document.createTextNode(item.text));
      if (current && typeof targetId === "string" && ["figure", "equation"].includes(String(semanticRole))) { const actions = node("span", "", "document-notes-object-actions"); actions.append(this.button(semanticRole === "equation" ? "公式编号与题注" : "此对象题注与编号", () => openDocumentNotes(documentNotesHost(this.authoringEnvironment()), "caption", targetId))); el.append(actions); }
      if (current && typeof noteId === "string" && ["footnote-body", "endnote-body"].includes(String(semanticRole))) el.append(this.button("查看正文标记与注释", () => openDocumentNotes(documentNotesHost(this.authoringEnvironment()), semanticRole === "endnote-body" ? "endnote" : "footnote", noteId)));
      const slot = empty.get(item.key); if (writable && slot) el.append(input("编辑空内容", slot));
      if (writable && item.origin?.isMain) {
        const range = writing.map?.structures.find(s => s.range.utf16From === item.origin!.utf16From && s.range.utf16To === item.origin!.utf16To)?.range;
        const controls = range && createStructureControls(writing, range, { ...options, renderedTable: el });
        if (controls) { const wrapper = node("div"); if (controls.classList.contains("writing-table-surface")) wrapper.append(controls); else wrapper.append(el, controls); if (openReference) wrapper.append(openReference); return wrapper; }
      }
      if (openReference) { const wrapper = node("div"); wrapper.append(el, openReference); return wrapper; }
      if (item.tag === "math" && current) {
        const wrapper = node("div", "", "math-discussion-target");
        const isCurrent = (): boolean => this.editor === editor && this.projection === projection && this.value === projection.source && this.serial === projection.serial && !this.paused;
        wrapper.append(el, createMathSelectionControls({ math: item, tree: projection.tree!, source: projection.source, current: isCurrent,
          notify: message => this.error(new Error(message)), select: selection => { if (isCurrent()) editor.select(selection.from, selection.to, false); } })); return wrapper;
      }
      return el;
    };
    this.flow.append(draw(projection.tree));
    if (this.personalKeys) displayPersonalHeadings(this.flow, personalNumberingVisible(this.personalKeys));
    if (writable && writing.map?.bodyInsertion?.writable) {
      const el = node("textarea", "", "literal-input append-input"); el.rows = 1; el.placeholder = "继续写作…"; el.setAttribute("aria-label","新段落文字输入"); writing.bindBodyInput(el); this.flow.append(el);
    }
    if (this.mode === "write") this.flow.append(node("p", "生成内容、依赖文件及未确认范围只读；完整正文保留在源码入口。", "hint"));
  }
}
