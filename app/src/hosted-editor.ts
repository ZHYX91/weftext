import { SourceEditor, sourceBoundary, type SourcePatch } from "./editor";
import { EmbeddedQueryController, sameQueryBasis, type EmbeddedQueryBasis } from "./embedded-query";
import type { QueryPlan, QueryResult, QueryRow } from "./query-panel";
import type { FrozenQuery } from "./export-state";
import { WritingController, groupWritingRenderNodes, groupWritingEmptySlots, type WritingMap, type WritingRange } from "./writing";
import { createWritingToolbar, createStructureControls, insertWritingLink } from "./writing-controls";
import { nativeElement } from "./native-node";
import { OfflineFontLoader } from "./font-readiness";
import type { Origin, RenderNode, SyntaxToken, ViewMode } from "./types";
import "./writing.css";
import "./hosted-editor.css";
import "./embedded-query.css";
import type { HostedQueryDependencies } from "./hosted-query-state";

interface Projection { ok: boolean; tree: RenderNode | null; source: string; serial: number; message: string; sourceInfo: { syntaxTokens: SyntaxToken[] } }
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
  constructor(private container: HTMLElement, private request: Request, private changed: () => void, private save: () => void, private error: (e: Error) => void, private queryHost?: HostedEditorQueryHost) {
    this.fonts = new OfflineFontLoader({ fonts: document.fonts, readManifest: () => request("fonts") });
    container.classList.add("hosted-editor");
    this.message.setAttribute("role", "status");
  }
  get value(): string { return this.writing?.recoverySource() ?? this.editor?.source ?? ""; }
  get serial(): number { return this.editor?.generation ?? 0; }
  get viewMode(): ViewMode { return this.mode; }
  get selection(): { from: number; to: number } | undefined { return this.editor?.selection; }
  select(from: number, to: number): void { this.editor?.select(from, to); }
  locateNodeFragment(origin: Origin, basis: EmbeddedQueryBasis): void {
    if (!this.editor || this.writing?.state.composing || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis) || origin.isMain !== true || origin.path !== `resource:${this.resource}` || !sourceBoundary(basis.source, origin.utf16From) || !sourceBoundary(basis.source, origin.utf16To) || basis.source.slice(origin.utf16From, origin.utf16To) !== origin.raw) throw new Error("定位期间目标草稿已变化，未采用旧标签范围。");
    // Selecting the actual native author range changes neither source nor Undo.
    this.mode = "source"; this.editor.clearReveal(); this.editor.readOnly(this.blocked); this.paint(true); this.editor.select(origin.utf16From, origin.utf16To); this.queryHost?.modeChanged?.();
  }
  set disabled(value: boolean) { this.editor?.readOnly(value || this.mode === "read"); if (this.blocked === value) return; this.blocked = value; this.paint(); }
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
  refreshEnvironment(): void { if (!this.editor) return; this.projection = undefined; this.queryPreview = false; this.queries?.invalidate(); this.message.textContent = "库配置依据已变化，正在重新编译当前草稿；正文与撤销历史保留。"; this.schedule(0); }
  queryInputsChanged(): void { this.queries?.runInputsChanged(); }
  async prepareQueryBasis(): Promise<EmbeddedQueryBasis> {
    await this.flush();
    const basis = { sessionId: this.sessionId, source: this.value, serial: this.serial };
    if (!this.editor || this.blocked || this.mode === "read" || this.writing?.state.composing) throw new Error("请在当前可写正文中提出保存定义，尚未发送提案。");
    await this.request("editor/map", basis);
    if (!sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis) || this.blocked || this.viewMode === "read") throw new Error("登记期间草稿或编辑资格已变化，请重新核对当前计划。");
    return basis;
  }
  open(sessionId: string, resource: string, source: string): void {
    // Reopening the same retained draft changes its Base/session, not Undo.
    if (this.resource !== resource || !this.editor) {
      this.clear(); this.resource = resource;
      this.editor = new SourceEditor(source, () => { this.queries?.invalidate(); this.changed(); this.schedule(); }, this.save);
      const editor = this.editor;
      let retained = false;
      this.writing = new WritingController({ editor,
        basis: () => ({ sessionId: this.sessionId, source: editor.source, serial: editor.generation }),
        editable: () => this.editor === editor && !this.blocked && this.mode !== "read",
        request: request => this.request("editor/edit", request), onError: this.error,
        onPendingSource: () => { this.queries?.invalidate(); this.changed(); }, onApplied: () => this.schedule(50),
        onState: () => {
          const next = this.writing?.retainInput ?? false;
          if (retained && !next && this.mode === "write") queueMicrotask(() => { if (this.editor === editor) this.paint(); });
          retained = next;
        },
      });
      if (this.queryHost) this.queries = new EmbeddedQueryController({
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
    } else if (this.editor.source !== source) {
      this.editor.apply(0, this.editor.source.length, source);
    }
    this.sessionId = sessionId; this.projection = undefined; this.generation++;
    this.editor.clearReveal(); this.editor.readOnly(this.blocked || this.mode === "read"); this.paint(); this.schedule(0);
  }
  clear(): void {
    clearTimeout(this.timer); this.generation++; this.queries?.dispose(); this.queries = undefined; this.queryPreview = false; this.writing?.dispose(); this.editor?.view.destroy();
    this.writing = undefined; this.editor = undefined; this.projection = undefined; this.resource = ""; this.sessionId = "";
    this.blocked = true; this.container.replaceChildren();
    this.sourceHost.replaceChildren(); this.flow.replaceChildren(); this.toolbar.replaceChildren(); this.message.textContent = "";
    // Shared structure dialogs may contain the just-revoked source. Dispose
    // both their visible data and their pending controller capability.
    document.querySelectorAll<HTMLDialogElement>(".writing-dialog").forEach(dialog => { dialog.close(); dialog.remove(); });
  }
  private schedule(delay = 250): void { clearTimeout(this.timer); this.generation++; this.timer = setTimeout(() => { void this.compile(); }, delay); }
  async applyQueryPatches(patches: SourcePatch[], basis: EmbeddedQueryBasis, event = "input.query.proposal"): Promise<void> {
    if (!this.editor || this.blocked || this.mode === "read" || this.writing?.state.composing || !sameQueryBasis({ sessionId: this.sessionId, source: this.value, serial: this.serial }, basis)) throw new Error("查询提案的草稿或编辑资格已变化，输入保留，未覆盖正文。");
    this.editor.applyPatches(basis.source, patches, undefined, event); this.schedule(0);
  }
  private async compile(): Promise<void> {
    const editor = this.editor, writing = this.writing; if (!editor || !writing || !this.sessionId) return;
    const basis = { sessionId: this.sessionId, source: editor.source, serial: editor.generation }, generation = this.generation;
    this.message.textContent = "正在更新当前草稿预览…";
    try {
      const [map, projection] = await Promise.all([this.request<WritingMap>("editor/map", basis), this.request<Projection>("editor/render", basis), this.fonts.ensureDefaults()]);
      if (this.editor !== editor || this.generation !== generation || this.sessionId !== basis.sessionId || editor.source !== basis.source || editor.generation !== basis.serial) return;
      if (this.queries) {
        try { await this.queries.discover(basis); } catch (error) { if (this.editor === editor && this.generation === generation) this.error(error instanceof Error ? error : new Error(String(error))); }
        if (this.editor !== editor || this.generation !== generation || this.sessionId !== basis.sessionId || editor.source !== basis.source || editor.generation !== basis.serial) return;
      }
      this.projection = projection; writing.installMap(map, basis.source, basis.serial);
      editor.tokens(projection.sourceInfo.syntaxTokens, this.mode === "styled"); this.message.textContent = projection.ok ? "预览对应当前草稿；编辑仍须明确保存。" : projection.message;
      this.paint();
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
    editor.clearReveal(); this.mode = mode; editor.readOnly(this.blocked || mode === "read");
    this.queries?.modeChanged();
    this.queryHost?.modeChanged?.();
    if (this.projection) editor.tokens(this.projection.sourceInfo.syntaxTokens, mode === "styled"); this.paint(true);
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
    if (!this.blocked && this.mode !== "read") {
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
      const slot = empty.get(item.key); if (writable && slot) el.append(input("编辑空内容", slot));
      if (writable && item.origin?.isMain) {
        const range = writing.map?.structures.find(s => s.range.utf16From === item.origin!.utf16From && s.range.utf16To === item.origin!.utf16To)?.range;
        const controls = range && createStructureControls(writing, range, options);
        if (controls) { const wrapper = node("div"); wrapper.append(el, controls); if (openReference) wrapper.append(openReference); return wrapper; }
      }
      if (openReference) { const wrapper = node("div"); wrapper.append(el, openReference); return wrapper; }
      return el;
    };
    this.flow.append(draw(projection.tree));
    if (writable && writing.map?.bodyInsertion?.writable) {
      const el = node("textarea", "", "literal-input append-input"); el.rows = 1; el.placeholder = "继续写作…"; el.setAttribute("aria-label","新段落文字输入"); writing.bindBodyInput(el); this.flow.append(el);
    }
    if (this.mode === "write") this.flow.append(node("p", "生成内容、依赖文件及未确认范围只读；完整正文保留在源码入口。", "hint"));
  }
}
