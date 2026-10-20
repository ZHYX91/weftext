import "./style.css";
import { RecognitionPanel } from "./recognition-panel";
import { api, ApiError, operationId } from "./api";
import { SourceEditor, applySourcePatches } from "./editor";
import { EverydayNavigation, type EverydayEntry, type EverydayCommand } from "./everyday-navigation";
import { openEverydayMenu, openEverydayPicker } from "./everyday-ui";
import { everydayWritingCommands } from "./everyday-writing";
import { EmbeddedQueryController } from "./embedded-query";
import "./embedded-query.css";
import { HistoryPanel } from "./history-panel";
import { restoreBufferDisposition, type HistoryContext } from "./history-state";
import { DocumentSession, encodeLiteral, canRebindSaveAs, sourceConsumerNeedsComparison, sourceMoveComparisonMessage } from "./session";
import { WritingController, groupWritingRenderNodes, groupWritingEmptySlots, textareaSourceDraft, type WritingMap, type WritingRange } from "./writing";
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
import { NoteInputRecovery } from "./note-input-recovery";
import { openLibraryCreation, type LibraryCreationReceipt } from "./library-creation";
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
  svg.setAttribute("viewBox", "0 0 24 24"); svg.setAttribute("fill", "none"); svg.setAttribute("stroke", "currentColor"); svg.setAttribute("stroke-width", "1.6"); svg.setAttribute("stroke-linecap", "round"); svg.setAttribute("stroke-linejoin", "round"); svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(svg.namespaceURI, "path"); path.setAttribute("d", paths[name] ?? paths.file); svg.append(path); return svg;
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag); element.className = className; if (text) element.textContent = text; return element;
}
function button(label: string, onClick: () => void | Promise<void>, className = "", iconName?: string): HTMLButtonElement {
  const result = el("button", className); result.type = "button"; if (iconName) result.append(icon(iconName)); if (label) result.append(document.createTextNode(label)); result.onclick = () => { void Promise.resolve(onClick()).catch(showError); }; return result;
}
const basename = (path: string): string => path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ?? path;
const normalize = (path: string): string => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
const labelStatus: Record<string, string> = { ordinary: "普通文件", unmanaged: "未纳管", unadmitted: "待登记", admitted: "笔记", duplicate: "重复标识", invalid: "标识错误", unsupported: "不支持", missing: "主文件缺失" };
const modes: { id: ViewMode; title: string; hint: string; short: string }[] = [
  { id: "source", title: "源码", short: "V01", hint: "完整 Typst 源码与所有语法字符" },
  { id: "styled", title: "样式源码", short: "V02", hint: "标题与强调带样式，所有语法字符可见" },
  { id: "focus", title: "按需语法", short: "V03", hint: "阅读时隐藏语法；选择段落可展开源码" },
  { id: "write", title: "可视编辑", short: "V04", hint: "隐藏语法，只编辑引擎确认的文字区域" },
  { id: "read", title: "阅读", short: "V05", hint: "只读查看当前草稿，可选择与复制文字" },
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
  draftFailure: string | null;
  rendering: number;
  renderGeneration: number;
  lastSave?: { source: string; baseId: string; operationId: string; path?: string };
  externalSource?: string;
  comparisonTarget?: OpenedDocument;
  saveAsChangedTarget?: string;
  pendingFragment?: string;
  scroll: Partial<Record<ViewMode, number>>;
  sourceScroll?: number;
  layout: "continuous" | "paged";
  richRetained: boolean;
  sourceTrashed?: boolean;
  sourceRefreshing?: boolean;
  notesMount?: DocumentNotesMount;
  authoringAssets: AuthoringAssetsAdapter;
}

function sourceConsumersAvailable(doc: LiveDocument): boolean { return !doc.sourceRefreshing && !sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget); }

const app = document.querySelector<HTMLDivElement>("#app")!;
const shell = el("div", "shell");
const sidebar = el("aside", "sidebar"); sidebar.setAttribute("aria-label", "资料库导航");
const workspace = el("main", "workspace");
const topbar = el("header", "topbar");
const tabs = el("nav", "tabs"); tabs.setAttribute("aria-label", "打开的文档");
const content = el("div", "content");
const workspaceBody = el("div", "workspace-body");
const statusbar = el("footer", "statusbar");
const toastHost = el("div", "toast-host"); toastHost.setAttribute("aria-live", "polite");
shell.append(sidebar, workspace); workspace.append(topbar, tabs, workspaceBody, statusbar); workspaceBody.append(content); app.append(shell, toastHost);
let inventory: Inventory | null = null;
let documents: LiveDocument[] = [];
let current: LiveDocument | null = null;
let renderedDocument: LiveDocument | null = null;
let renderedMode: ViewMode | null = null;
const everydayNavigation = new EverydayNavigation();
let navigatingHistory = false;
let navigationGeneration = 0;
const navigationContexts = new Map<LiveDocument, { inventory: Inventory | null; query: string; folder: string | null; collapsed: string[]; scroll: number; contentScroll:number; selection:string[]; sort:"name"|"path" }>();
const everydayBar = el("nav", "everyday-bar"); everydayBar.setAttribute("aria-label", "日常导航与命令"); workspace.insertBefore(everydayBar, tabs);
let query = "";
let selectedFolder: string | null = null;
let sidebarComposing = false;
const contentSelections = new Set<string>();
let contentSort: "name" | "path" = "name";
const folderViews = new Map<string, { query: string; scroll: number }>();
const collapsed = new Set<string>();
let previewVisible = localStorage.getItem("weftext-preview") !== "false";
let activeSourceOrigin: Origin | null = null;
let startupError: string | null = null;
let recoveries: RecoveryInfo[] = [];
let renderHost: HTMLElement | null = null;
let flowScroll: HTMLElement | null = null;
let flowHasProjection = false;
let pendingCreation: { route: string; payload: Record<string, unknown> } | null = null;
let pagedPreview: PagedPreview | null = null;
function captureNavigation(): void {
  if (!current) return;
  navigationContexts.set(current, { inventory, query, folder: selectedFolder, collapsed: [...collapsed], scroll: sidebar.querySelector(".file-tree")?.scrollTop ?? 0, contentScroll:sidebar.querySelector(".node-contents")?.scrollTop??0, selection:[...contentSelections], sort:contentSort });
}
function restoreNavigation(doc: LiveDocument): void {
  const saved = navigationContexts.get(doc); if (!saved) return;
  inventory = saved.inventory; query = saved.query; selectedFolder = saved.folder;
  collapsed.clear(); saved.collapsed.forEach(path => collapsed.add(path));
  contentSelections.clear();saved.selection.forEach(path=>contentSelections.add(path));contentSort=saved.sort;
}
function everydayEntries(): EverydayEntry[] {
  const entries = new Map<string, EverydayEntry>();
  for (const doc of documents) entries.set(doc.navigationKey, { key: doc.navigationKey, title: basename(doc.session.path), detail: `${doc.session.path} · ${saveState(doc)}`, open: () => activateDocument(doc) });
  for (const entry of inventory?.entries ?? []) {
    if (!entry.path.toLowerCase().endsWith(".typ") || entry.kind === "blocked") continue;
    const existing = documents.find(doc => normalize(doc.session.path) === normalize(entry.path));
    if (existing) continue;
    const root = inventory!.root, workspaceId = inventory!.workspaceId;
    entries.set(entry.path, { key: entry.path, title: entry.title || entry.name, detail: `${entry.relativePath} · ${labelStatus[entry.identityStatus] ?? entry.identityStatus}`, unavailable: ["duplicate", "invalid", "missing"].includes(entry.identityStatus) ? "当前身份不可用于准确打开，请先核对资料库提示。" : undefined,
      open: () => openInventoryEntry(entry,root,workspaceId) });
  }
  return [...entries.values()];
}
async function openInventoryEntry(entry:InventoryEntry,root:string,workspaceId?:string):Promise<void>{
  if(!entry.nodeId||!workspaceId){await openDocument(entry.path,root);return;}
  const generation=++navigationGeneration;await prepareSwitch();if(generation!==navigationGeneration)return;
  const opened=await api<OpenedDocument>("/api/data/open",{root,entityKey:`weftext-node://${workspaceId}/${entry.nodeId}`});
  if(generation!==navigationGeneration)return;if(opened.nodeId!==entry.nodeId||opened.workspaceId!==workspaceId)throw new Error("返回目标不是原笔记，原稿保留。");await acceptResolvedDocument(opened);
}
function everydayCommands(): EverydayCommand[] {
  const doc = current;
  return [
    { id: "quick-open", label: "快捷打开", detail: "Ctrl+P · 按标题和路径查找", run: () => openEverydayPicker("快捷打开", everydayEntries(), toast) },
    { id: "recent", label: "最近访问", run: () => showRecent() },
    { id: "new-note", label: "新建笔记", unavailable: !inventory?.workspaceId ? "请先打开资料库。" : undefined, run: createNote },
    { id: "open-file", label: "打开库外 Typst 文件", run: openOutsideFile },
    { id: "save", label: "保存当前笔记", unavailable: !doc ? "请先打开笔记。" : undefined, run: () => saveDocument(doc!) },
    { id: "resources", label: "导入或浏览图片与资源", run: async () => { await workspaceTools.show("knowledge"); workspaceTools.selectKnowledgeTab("files"); } },
    ...everydayWritingCommands(doc?.writing, !!doc && sourceConsumersAvailable(doc) && doc.mode !== "read" && !readOnlySource(doc) && !doc.writing.state.composing && !doc.editor.composing, insertEverydayQuery),
  ];
}
function showRecent(): void {
  const entries = everydayEntries(); openEverydayPicker("最近访问", everydayNavigation.recent.flatMap(key => entries.filter(entry => entry.key === key)), toast);
}
async function stepNavigation(direction: -1 | 1): Promise<void> {
  const key = everydayNavigation.target(direction), target = documents.find(doc => doc.navigationKey === key); if (!key || !target) return;
  navigatingHistory = true;
  try { await activateDocument(target); if (current === target) everydayNavigation.arrive(direction, key); }
  finally { navigatingHistory = false; renderEverydayBar(); }
}
function renderEverydayBar(): void {
  if (current && !navigatingHistory) everydayNavigation.visit(current.navigationKey);
  everydayBar.replaceChildren();
  for (const [direction, label] of [[-1,"返回"],[1,"前进"]] as const) { const control = button(label, () => stepNavigation(direction)); control.disabled = !everydayNavigation.target(direction); everydayBar.append(control); }
  everydayBar.append(button("快捷打开", () => openEverydayPicker("快捷打开", everydayEntries(), toast)), button("最近访问", showRecent), button("命令", () => openEverydayMenu("命令", everydayCommands(), toast)), button("插入与块操作", () => openEverydayMenu("插入与块操作", everydayCommands().filter(item => /^(insert-|convert-|move-|undo|redo|resources)/.test(item.id)), toast)));
}
async function insertEverydayQuery(): Promise<void> {
  const doc = current; if (!doc?.session.opened.nodeId || !doc.session.opened.workspaceId) throw new Error("内嵌 Query 需要资料库笔记与已安装的 schema。");
  await prepareSwitch(); const basis = { sessionId:doc.session.opened.sessionId, source:doc.session.source, serial:doc.session.serial, baseId:doc.session.baseId };
  const operation=operationId();
  await modal<void>("插入内嵌 Query", "使用当前已安装的字段配置，在本篇添加查询定义。插入只改草稿，可一次撤销；保存后查询可运行。每次保存前可新增一个定义。", (form, close) => {
    const label = el("input", "text-input"); label.value = "笔记列表"; label.required = true;
    const condition = el("input", "text-input"); condition.placeholder = "留空表示当前库内全部获权笔记";
    const name = el("label", "field-label", "查询名称"); name.append(label); const where = el("label", "field-label", "筛选条件（CEL，可留空）"); where.append(condition);
    const kind = el("select", "text-input"); kind.setAttribute("aria-label", "查询来源"); for (const [value, text] of [["new", "新查询定义"], ["call", "复用保存定义（独立调用）"]]) { const option = el("option", "", text); option.value = value; kind.append(option); }
    const shared = el("fieldset"); shared.hidden = true; shared.append(el("legend", "", "选择真实定义归属"));
    const owners = el("select", "text-input"); owners.setAttribute("aria-label", "定义归属笔记"); const definitions = el("select", "text-input"); definitions.setAttribute("aria-label", "复用的保存定义");
    const mode = el("select", "text-input"); mode.setAttribute("aria-label", "复用定义版本方式"); for (const [value, text] of [["current", "当前定义"], ["pinned", "固定准确版本"]]) { const option = el("option", "", text); option.value = value; mode.append(option); }
    const revision = el("input", "text-input"); revision.setAttribute("aria-label", "复用定义准确固定版本"); revision.disabled = true; mode.onchange = () => { revision.disabled = mode.value !== "pinned"; };
    if (inventory?.root === doc.session.opened.root) for (const entry of inventory.entries.filter(entry => entry.kind === "node" && entry.identityStatus === "admitted" && entry.nodeId)) { const option = el("option", "", entry.title || entry.name); option.value = `weftext-node://${doc.session.opened.workspaceId}/${entry.nodeId}`; owners.append(option); }
    let loadedOwner = "";
    const load = button("读取所选笔记的保存定义", async () => { const owner = owners.value; if (!owner) throw new Error("请先在当前资料库打开导航，再选择真实定义笔记。"); load.disabled = true; try { const result = await api<{ ownerRevision: string | null; definitions: { key: string; label: string }[] }>("/api/query/definitions", { root: doc.session.opened.root, owner }); if (!form.isConnected || owners.value !== owner) return; definitions.replaceChildren(); for (const definition of result.definitions) { const option = el("option", "", definition.label.replace(/^weftext-query-/, "")); option.value = definition.key; definitions.append(option); } revision.value = result.ownerRevision ?? ""; loadedOwner = owner; if (!result.definitions.length) { error.textContent = "所选笔记没有当前可用定义；不会按同名替换。"; error.hidden = false; } } catch (failure) { error.textContent = failure instanceof Error ? failure.message : String(failure); error.hidden = false; } finally { load.disabled = false; } });
    owners.onchange = () => { definitions.replaceChildren(); loadedOwner = ""; };
    shared.append(owners, load, definitions, mode, revision, el("p", "field-help", "此处只新增调用；共享定义归属保持不变。固定版本须准确填写，不能静默采用当前版本。"));
    kind.onchange = () => { shared.hidden = kind.value !== "call"; where.hidden = kind.value === "call"; };
    const error=el("p","writing-error");error.setAttribute("role","alert");error.hidden=true;
    const submit = el("button", "button-primary", "插入查询"); submit.type = "submit";
    form.append(name, kind, where, shared, error, button("取消", () => close(null)), submit);
    const selection = () => JSON.stringify([kind.value, owners.value, definitions.value, mode.value, revision.value]);
    const available=()=>current===doc&&doc.session.opened.sessionId===basis.sessionId&&doc.session.baseId===basis.baseId&&doc.session.source===basis.source&&doc.session.serial===basis.serial&&sourceConsumersAvailable(doc)&&!doc.sourceTrashed&&!readOnlySource(doc)&&doc.mode!=="read";
    form.onsubmit=event=>{
      event.preventDefault();submit.disabled=true;error.hidden=true;
      void (async()=>{
        if(!available())throw new Error("正文在确认期间已变化，原输入保留，请取消后重新选择当前笔记。");
        const nameValue=label.value,conditionValue=condition.value, selected = selection();
        const nativeName=nameValue.trim().replace(/\s+/g,"-");if(!/^[\p{L}\p{N}_.:-]+$/u.test(nativeName))throw new Error("名称请使用文字、数字、连字符、下划线或句点。");
        let plan: import("./query-panel").QueryPlan = {scope:{descendants:true,lifecycle:"live"},types:[],...(conditionValue ? { where: conditionValue } : {}),select:[{name:"title",expr:"title"}],order:[],take:100};
        if (kind.value === "call") { if (loadedOwner !== owners.value || !definitions.value || mode.value === "pinned" && !revision.value) throw new Error("请选择已读取的准确定义和需要的固定版本。"); plan = { scope:{descendants:true,lifecycle:"live"},types:[],select:[],order:[],advanced:{call:{owner:owners.value,selection: mode.value === "pinned" ? {mode:"pinned",key:definitions.value,revision:revision.value} : {mode:"current",key:definitions.value},arguments:{}}} }; }
        const result=await api<{draftOnly:boolean;source:string;serial:number;patchUnit:string;patches:{from:number;to:number;text:string;expected:string}[]}>("/api/query/definition/draft",{...basis,operationId:operation,label:`weftext-query-${nativeName}`,plan});
        if(!available()||!form.isConnected||label.value!==nameValue||condition.value!==conditionValue||selection()!==selected||!result.draftOnly||result.serial!==basis.serial||result.patchUnit!=="utf16")throw new Error("查询提案或输入已变化，原稿和新输入保留，请重新确认。");
        const patches=result.patches.map(patch=>({...patch,insert:patch.text}));if(applySourcePatches(basis.source,patches)!==result.source)throw new Error("查询补丁与提案不符，未修改正文。");
        doc.editor.applyPatches(basis.source,patches,undefined,"input.query.insert");close(undefined);await persistDraft(doc);scheduleRender(doc,0);
      })().catch(failure=>{if(!form.isConnected){showError(failure);return;}error.textContent=failure instanceof ApiError&&failure.code==="schemaRevisionRequired"?"请先在查询配置中显式安装字段配置，再插入查询；当前正文未变。":failure instanceof Error?failure.message:String(failure);error.hidden=false;submit.disabled=false;});
    };
  });
}
let projectionMappings: KnowledgeTextMapping[] = [];
let readingSelection: { sessionId: string; serial: number; selection: NonNullable<KnowledgeContext["selection"]> } | null = null;
const recognitionPanel = new RecognitionPanel({
  request: (path, body) => api(`/api/recognition${path}`, body),
  identity: () => current ? current.session.opened.root : null,
  target: () => current ? { sessionId: current.session.opened.sessionId, parent: selectedFolder ?? current.session.opened.root, name: "资料导入" } : null,
  notify: toast,
  created: async receipt => { const root = current?.session.opened.root, ref = receipt.nodeRefs[0]; if (!root || !ref) throw new Error("原导入的新笔记当前不可定位，请核对原操作。"); await prepareSwitch(); const opened = await api<OpenedDocument>("/api/data/open", { root, entityKey: `weftext-node://${ref.workspaceId}/${ref.nodeId}` }); await acceptResolvedDocument(opened); await refreshInventory(); },
});
let recognitionSession = "";

const workspaceTools = new WorkspaceTools({
  embeddedQueries: () => current && sourceConsumersAvailable(current) ? current.queries.outputSelections() : [],
  getDocument: () => current && !current.sourceTrashed && sourceConsumersAvailable(current) ? { sessionId: current.session.opened.sessionId, source: current.session.source, serial: current.session.serial, baseId: current.session.baseId, path: current.session.path, root: current.session.opened.root, nodeId: current.session.opened.nodeId, workspaceId: current.session.opened.workspaceId, dirty: current.session.dirty } : null,
  selection: () => {
    if (!current) return undefined;
    if (readingSelection?.sessionId === current.session.opened.sessionId && readingSelection.serial === current.session.serial) return readingSelection.selection;
    const range = current.editor.selection;
    return range.from === range.to ? undefined : range;
  },
  beforeAction: prepareSwitch, notify: toast, openDocument, openResolved: acceptResolvedDocument,
  refreshLibrary: async root => { if (inventory && normalize(inventory.root) === normalize(root)) await refreshInventory(); },
  syncApplied: async root => {
    if (inventory && normalize(inventory.root) === normalize(root)) await refreshInventory();
    for (const doc of documents.filter(d => normalize(d.session.opened.root) === normalize(root))) {
      const originalSession = doc.session.opened.sessionId;
      const stillOpen = (): boolean => documents.includes(doc) && doc.session.opened.sessionId === originalSession;
      doc.sourceRefreshing = true; doc.renderGeneration++; clearTimeout(doc.renderTimer); doc.queries.invalidate(); doc.session.projection = null; doc.editor.clearTokens();
      if (doc === current) { pagedPreview?.invalidate("同步结果正在核对，原草稿保留。"); workspaceTools.documentChanged(); }
      try {
        // A committed sync can arrive during later preedit. Settle/checkpoint
        // input before replacing any surface, retaining shared editor/Undo.
        await doc.writing.settleForSessionChange();
        if (!stillOpen()) continue;
        if (!readOnlySource(doc)) await persistDraft(doc);
        if (!stillOpen()) continue;
        let fresh: OpenedDocument | undefined, unavailable = false;
        try {
          if (doc.session.opened.nodeId && doc.session.opened.workspaceId) fresh = await api<OpenedDocument>("/api/data/open", { root, entityKey: `weftext-node://${doc.session.opened.workspaceId}/${doc.session.opened.nodeId}` });
        } catch { unavailable = true; }
        await doc.writing.deferUntilCompositionEnds(() => {
          if (!stillOpen()) return;
          if (fresh && (fresh.source !== doc.session.baseSource || normalize(fresh.path) !== normalize(doc.session.path))) {
            doc.externalSource = fresh.source; doc.comparisonTarget = fresh;
            doc.session.error = "同步已改变磁盘版本。当前草稿与撤销记录保留，请比较草稿与磁盘内容后继续。";
          } else if (unavailable) doc.session.error = "同步后当前笔记暂不可读取。草稿保留，请检查资料库或回收站。";
          // Equality cannot prove unchanged sidecars or compilation inputs.
          // A moved writer waits for explicit comparison instead of compiling
          // its old pathname or silently installing the fresh Save basis.
          doc.renderGeneration++; doc.queries.invalidate(); doc.session.projection = null;
        });
      } finally { doc.sourceRefreshing = false; if (stillOpen()) scheduleRender(doc, 0); }
    }
    const displayed = current;
    if (displayed && normalize(displayed.session.opened.root) === normalize(root)) {
      await displayed.writing.deferUntilCompositionEnds(() => {
        if (current !== displayed) return;
        pagedPreview?.invalidate();
        if (!displayed.writing.retainInput) renderContent(); else refreshChrome();
      });
    }
    await workspaceTools.refreshAfterSync(root); renderSidebar();
  },
  canEditSource: () => !!current && sourceConsumersAvailable(current) && current.mode !== "read" && !readOnlySource(current) && !current.writing.state.composing,
  savedDefinition: async (result, basis) => {
    const receipt = result as { document?: OpenedDocument; committedSource?: string; documentSourceMatchesCommit?: boolean };
    const opened = receipt.document;
    if (!opened || typeof opened.sessionId !== "string" || typeof opened.source !== "string") throw new Error("Query 保存缺少实际文件回执，请检查原操作。");
    const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
    if (!doc) throw new Error("Query 已保存，请重新打开笔记确认。");
    if (receipt.documentSourceMatchesCommit !== true || receipt.committedSource !== opened.source) {
      doc.session.error = "Query 已提交，但文件随后发生了其他修改。当前草稿和撤销记录保留，请比较磁盘内容后继续。";
      if (doc === current) renderContent(); return;
    }
    await doc.writing.settleForSessionChange();
    if (doc.session.source !== basis.source || doc.session.serial !== basis.serial || doc.session.baseId !== basis.baseId) {
      doc.session.error = "Query 与提交的笔记版本已保存，期间的新输入仍保留；请比较磁盘内容后继续保存。";
      if (doc === current) renderContent();
      return;
    }
    doc.session.opened = opened; doc.editor.clearTokens();
    if (doc.editor.source !== opened.source) doc.editor.apply(0, doc.editor.source.length, opened.source);
    doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path); doc.lastSave = undefined; doc.session.projection = null;
    await persistDraft(doc); scheduleRender(doc, 0); await refreshInventory(); refreshChrome();
  },
  selectSource: (from, to) => { if (current) { current.mode = "source"; current.editor.clearReveal(); renderContent(); current.editor.select(from, to); } },
  applyPatch: async (patch, basis) => {
    const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
    if (!doc || doc.session.source !== basis.source || doc.session.serial !== basis.serial || doc.session.baseId !== basis.baseId || doc.mode === "read" || readOnlySource(doc) || doc.writing.state.composing) throw new Error("当前草稿不能接受这份旧修改，请切换编辑视图并重试。");
    await doc.writing.flush();
    if (doc.session.source !== basis.source || doc.session.serial !== basis.serial) throw new Error("输入已进入草稿，请重新核对并操作。");
    doc.editor.apply(patch.from, patch.to, patch.text); await persistDraft(doc); scheduleRender(doc, 0);
  },
  created: acceptCreatedDocument,
  navigate: async target => {
    if (typeof target === "string") await navigateLink(target);
    else {
      if (!inventory?.workspaceId || target.workspaceId !== inventory.workspaceId) throw new Error("目标不属于当前打开的资料库，请先打开目标库。");
      await prepareSwitch();
      await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: inventory.root, entityKey: `weftext-node://${target.workspaceId}/${target.nodeId}` }));
    }
  },
  applyReceipt: async (receipt, context) => {
    if (!["committed", "saved", "complete", "success"].includes(receipt.status)) return;
    const opened = receipt.document as OpenedDocument | undefined;
    const doc = documents.find(item => item.session.opened.sessionId === context.sessionId);
    if (doc) await doc.writing.settleForSessionChange();
    if (doc && (receipt.documentSourceMatchesCommit === false || receipt.documentError)) {
      doc.session.error = "操作已提交，但重新读取的文件不是本次计划确认的内容。草稿和撤销记录保留，请比较磁盘内容。";
      if (doc === current) renderContent();
    } else if (receipt.documentDisposition === "trashed" && doc) {
      doc.sourceTrashed = true; doc.editor.clearTokens();
      doc.session.error = "笔记已移入回收站。这里保留当前草稿，可另存为，也可从上方回收站恢复。";
      await persistDraft(doc); if (current === doc) renderContent();
    } else if (opened && receipt.documentDisposition === "created") {
      if (current === doc && doc?.session.source === context.source && doc.session.serial === context.serial) await acceptCreatedDocument(opened);
      else toast(`已创建 ${basename(opened.path)}，可从文件列表打开。`);
    } else if (opened && doc && opened.source === doc.session.baseSource) {
      // Namespace changes keep the same saved bytes. Preserve newer local input,
      // the shared editor and Undo while rebinding the real Core save session.
      doc.session.opened = opened; doc.editor.clearTokens(); doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
      doc.lastSave = undefined; doc.session.projection = null;
      await persistDraft(doc); scheduleRender(doc, 0); if (current === doc) renderContent();
    } else if (opened && doc && doc.session.source === context.source && doc.session.serial === context.serial && !doc.session.dirty) {
      doc.session.opened = opened; doc.editor.clearTokens();
      if (doc.editor.source !== opened.source) doc.editor.apply(0, doc.editor.source.length, opened.source);
      doc.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
      doc.lastSave = undefined; doc.session.projection = null; await persistDraft(doc); scheduleRender(doc, 0); if (current === doc) renderContent();
    } else if (opened && doc) {
      doc.session.error = "文件操作已完成，期间的新输入仍保留。请比较当前磁盘版本后继续保存。"; refreshChrome();
    }
    const affected = receipt.affectedDocuments as { priorSessionId: string; document?: OpenedDocument; documentDisposition: string; documentError?: { message?: string } }[] | undefined;
    for (const effect of affected ?? []) {
      const related = documents.find(item => item.session.opened.sessionId === effect.priorSessionId);
      if (!related) continue;
      await related.writing.settleForSessionChange();
      if (effect.documentDisposition === "trashed") {
        related.sourceTrashed = true; related.editor.clearTokens(); related.session.error = "所属文件夹已移入回收站，当前草稿保留。请恢复后继续保存，或另存为。";
        await persistDraft(related);
      } else if (effect.document && effect.document.source === related.session.baseSource) {
        related.session.opened = effect.document; related.editor.clearTokens();
        related.session.acknowledgeSave(effect.document.source, effect.document.baseId, effect.document.path);
        related.lastSave = undefined; related.session.projection = null; await persistDraft(related); scheduleRender(related, 0);
      } else {
        related.session.error = `文件夹操作已完成，草稿保留；${effect.documentError?.message ?? "该子笔记内容发生变化，请重新打开目标并比较。"}`;
        if (effect.document) related.saveAsChangedTarget = effect.document.path;
      }
      if (related === current) renderContent();
    }
    await refreshInventory();
  },
  stylesChanged: async root => {
    pagedPreview?.invalidate("库样式已变化，正在重新编译。");
    for (const doc of documents.filter(item => normalize(item.session.opened.root) === normalize(root))) { doc.session.projection = null; doc.editor.clearTokens(); scheduleRender(doc, 0); }
    if (current?.layout === "paged") await pagedPreview?.render();
    await refreshInventory();
  },
});
workspace.insertBefore(workspaceTools.bar, workspaceBody); workspaceBody.append(workspaceTools.inspector);
const navigationLayout = installNavigationLayout({ shell, sidebar, workspaceBody, inspector: workspaceTools.inspector, toolbar: workspaceTools.bar, notify: toast });

function historyContext(doc: LiveDocument | null): HistoryContext | null {
  if (!doc) return null;
  const { session } = doc;
  const source = doc.writing.recoverySource() ?? session.source;
  return { sessionId: session.opened.sessionId, source, serial: session.serial,
    baseId: session.baseId, baseSource: session.baseSource, path: session.path,
    root: session.opened.root, nodeId: session.opened.nodeId, workspaceId: session.opened.workspaceId,
    dirty: source !== session.baseSource, readOnly: doc.mode === "read" || readOnlySource(doc) || doc.sourceTrashed,
    savePending: session.savePending, unresolvedSave: !!doc.lastSave || !!session.opened.recoveredDraftConflict };
}
const historyPanel = new HistoryPanel({
  getDocument: () => historyContext(current), request: api, prepareRestore: prepareSwitch, notify: toast,
  restorePending: (basis, pending) => {
    const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
    if (doc) { doc.session.savePending = pending; if (doc === current) refreshChrome(); }
  },
  restored: async (result, basis) => {
    const doc = documents.find(item => item.session.opened.sessionId === basis.sessionId);
    if (!doc) throw new Error("旧版已保存，请重新打开原笔记确认；受保护草稿仍由 Core 保留。");
    await doc.writing.settleForSessionChange();
    const disposition = restoreBufferDisposition(basis, historyContext(doc));
    if (disposition === "unrelated" || result.receipt.status !== "saved" || !result.receipt.baseId) throw new Error("恢复回执与原笔记不符，当前草稿保持原样。");
    const variants = [...(doc.session.opened.recoveredDraftVariants ?? []), ...result.protectedDrafts];
    doc.session.opened.recoveredDraftVariants = variants.filter((value, index) => variants.findIndex(other => other.path === value.path && other.baseSource === value.baseSource && other.source === value.source && other.baseKnown === value.baseKnown) === index);
    // Keep the shared editor and Undo. Only a still-clean, unchanged buffer adopts
    // the restored text; any original or late input remains a Draft over the new Base.
    if (disposition === "replaceClean" && doc.editor.source !== result.source) doc.editor.apply(0, doc.editor.source.length, result.source);
    doc.session.opened.source = result.source; doc.session.opened.baseId = result.receipt.baseId;
    doc.session.opened.path = result.receipt.path; doc.session.opened.recoveredDraftConflict = false;
    doc.session.acknowledgeSave(result.source, result.receipt.baseId, result.receipt.path);
    doc.lastSave = undefined; doc.session.projection = null; doc.editor.clearTokens(); doc.queries.invalidate();
    await persistDraft(doc); scheduleRender(doc, 0); await refreshInventory(); refreshChrome();
  },
});

async function refreshInventory(): Promise<void> {
  if (inventory) inventory = await api<Inventory>("/api/library/open", { path: inventory.root });
  renderSidebar();
}

async function acceptCreatedDocument(opened: OpenedDocument): Promise<void> {
  await prepareSwitch();
  const restored = documents.find(item => item.sourceTrashed && (normalize(item.session.path) === normalize(opened.path) || Boolean(opened.nodeId && item.session.opened.nodeId === opened.nodeId && item.session.opened.workspaceId === opened.workspaceId)));
  if (restored && opened.source === restored.session.baseSource) {
    await restored.writing.settleForSessionChange();
    restored.session.opened = opened; restored.editor.clearTokens(); restored.sourceTrashed = false;
    restored.session.acknowledgeSave(opened.source, opened.baseId, opened.path); restored.lastSave = undefined; restored.session.projection = null;
    await persistDraft(restored); current = restored; activeSourceOrigin = null; renderContent(); scheduleRender(restored, 0);
  } else addDocument(opened);
  await refreshInventory();
}

async function acceptResolvedDocument(opened: OpenedDocument): Promise<void> {
  const generation=++navigationGeneration;
  await prepareSwitch();
  if(generation!==navigationGeneration)return;
  const existing = opened.nodeId && opened.workspaceId ? documents.find(item =>
    item.session.opened.nodeId === opened.nodeId && item.session.opened.workspaceId === opened.workspaceId
    && readOnlySource(item) === (opened.readOnly === true || opened.kind === "trash" || opened.kind === "managedTrash")) : undefined;
  if (!existing) addDocument(opened);
  else {
    await existing.writing.settleForSessionChange();
    if (existing.session.baseSource === opened.source) {
      existing.session.opened = opened; existing.editor.clearTokens();
      existing.sourceTrashed = false;
      existing.session.acknowledgeSave(opened.source, opened.baseId, opened.path);
      existing.session.projection = null; existing.lastSave = undefined;
      await persistDraft(existing); scheduleRender(existing, 0);
    } else {
      existing.externalSource = opened.source;
      existing.comparisonTarget = opened;
      existing.session.error = "目标已打开，但磁盘内容发生了其他修改。已保留当前草稿和撤销记录，请比较后继续。";
    }
    current = existing; activeSourceOrigin = null; renderContent(); renderSidebar();
  }
  shell.classList.remove("sidebar-open");
}

function toast(message: string, error = false): void {
  const item = el("div", `toast ${error ? "error" : ""}`, message); toastHost.append(item); setTimeout(() => item.remove(), error ? 7000 : 3500);
}
function showError(error: unknown): void { toast(error instanceof Error ? error.message : String(error), true); }
function setBusy(target: HTMLButtonElement, busy: boolean): void { target.disabled = busy; target.classList.toggle("busy", busy); }

async function modal<T>(title: string, description: string, build: (form: HTMLFormElement, close: (value: T | null) => void) => void): Promise<T | null> {
  const trigger=document.activeElement as HTMLElement|null;
  const dialog = el("dialog", "modal");dialog.setAttribute("aria-label",title);
  const closeButton=button("",()=>close(null),"icon-button","close");closeButton.setAttribute("aria-label",`关闭${title}`);
  const heading = el("div", "modal-heading"); heading.append(el("h2", "", title), closeButton);
  dialog.append(heading, el("p", "modal-description", description));
  const form = el("form", "modal-form"); dialog.append(form); document.body.append(dialog);
  let resolve!: (value: T | null) => void;
  const result = new Promise<T | null>(r => { resolve = r; });
  function close(value: T | null): void { dialog.close(); dialog.remove(); if(trigger?.isConnected)trigger.focus({preventScroll:true});resolve(value); }
  dialog.addEventListener("cancel", event => { event.preventDefault(); close(null); });
  build(form, close); dialog.showModal(); setTimeout(() => form.querySelector<HTMLElement>("input,textarea,button")?.focus(), 20); return result;
}
async function pathDialog(kind: "openLibrary" | "createLibrary" | "file" | "saveAs"): Promise<string | null> {
  const config = {
    openLibrary: ["打开资料库", "输入本机文件夹的完整路径。已有资料库将直接打开，普通文件夹也可以浏览。", "文件夹路径", "打开"],
    createLibrary: ["创建资料库", "输入尚不存在的新文件夹路径。资料库将创建该文件夹并保存笔记与恢复信息。", "新文件夹路径", "创建资料库"],
    file: ["打开 Typst 文件", "输入 .typ 文件的完整路径。库外文件可直接编辑，依赖从文件所在目录读取。", "文件路径", "打开文件"],
    saveAs: ["另存为", "输入新的 .typ 文件完整路径。成功后编辑会话切换到新文件。", "文件路径", "另存为"],
  }[kind];
  return modal<string>(config[0], config[1], (form, close) => {
    const label = el("label", "field-label", config[2]); const input = el("input", "text-input"); input.required = true; input.placeholder = kind.includes("Library") ? "D:\\Documents\\我的资料库" : "D:\\Documents\\笔记.typ";
    if (kind === "saveAs" && current) input.value = current.session.path;
    label.append(input); form.append(label);
    if (kind === "openLibrary" && inventory) { const recent = button(`当前：${inventory.root}`, () => { input.value = inventory!.root; input.focus(); }, "recent-path"); form.append(recent); }
    const actions = el("div", "modal-actions"); actions.append(button("取消", () => close(null), "button-secondary")); const submit = el("button", "button-primary", config[3]); submit.type = "submit"; actions.append(submit); form.append(actions);
    form.onsubmit = event => { event.preventDefault(); if (input.value.trim()) close(input.value.trim()); };
  });
}

function renderSidebar(): void {
  if (sidebarComposing) return;
  navigationLayout.captureSidebarState();
  navigationLayout.setContext(inventory ? { root: inventory.root, workspaceId: inventory.workspaceId, scope: selectedFolder ? { kind: "directory", key: `path:${normalize(selectedFolder)}` } : undefined } : null);
  workspaceTools.setLibrary(inventory, selectedFolder);
  const focused = document.activeElement as HTMLInputElement | null;
  const restoreSearch = focused?.id === "library-search";
  const selection = restoreSearch ? [focused!.selectionStart, focused!.selectionEnd] : null;
  sidebar.replaceChildren();
  const brand = el("div", "brand"); const mark = el("div", "brand-mark", "w"); brand.append(mark, el("span", "brand-name", "Weftext"), el("span", "local-badge", "本机")); sidebar.append(brand);
  const libraryButton = button(inventory ? basename(inventory.root) : "打开资料库", chooseLibrary, "library-button", "book"); libraryButton.append(icon("down")); if (inventory) libraryButton.title = inventory.root; sidebar.append(libraryButton);
  if (inventory && !inventory.workspaceId) sidebar.append(el("span", "library-kind", "普通文件夹 · 文件可直接编辑"));
  const quick = el("div", "sidebar-quick"); quick.append(button("新建笔记", createNote, "new-note", "plus"), button("", openOutsideFile, "icon-button", "file")); quick.lastElementChild!.setAttribute("aria-label", "打开库外 Typst 文件"); (quick.firstChild as HTMLButtonElement).disabled = !inventory?.workspaceId; if (inventory && !inventory.workspaceId) (quick.firstChild as HTMLButtonElement).title = "普通文件夹可浏览文件。创建新的资料库后可新建资料库笔记。"; sidebar.append(quick);
  const search = el("label", "search-box"); search.append(icon("search")); const input = el("input"); input.id = "library-search"; input.placeholder = "搜索文件名与标题…"; input.value = query; input.disabled = !inventory; input.setAttribute("aria-label", "搜索文件名与标题"); input.oninput = event => { query = input.value; if (!(event as InputEvent).isComposing && !sidebarComposing) renderSidebar(); }; input.addEventListener("compositionstart",()=>{sidebarComposing=true;}); input.addEventListener("compositionend",()=>{sidebarComposing=false;query=input.value;renderSidebar();}); search.append(input); sidebar.append(search);
  if (inventory) {
    const scope = el("div", "search-scope"); scope.append(button("正文与属性查询", () => workspaceTools.search(query, true), "text-button"), el("span", "", selectedFolder ? basename(selectedFolder) : "整个资料库")); sidebar.append(scope);
    if (selectedFolder) sidebar.append(button("返回整个资料库", () => browseFolder(null), "clear-scope"));
    if (selectedFolder) renderNodeContents(sidebar, selectedFolder);
  }
  const list = el("div", "file-tree");
  if (!inventory) {
    const empty = el("div", "sidebar-empty"); empty.append(icon("folder"), el("p", "", "你的笔记会出现在这里"), el("span", "", "打开资料库，或直接打开 .typ 文件。")); list.append(empty);
  } else {
    const heading = el("div", "section-label", query ? "搜索结果" : "文件与笔记"); if (!inventory.complete) heading.append(el("span", "partial-label", "扫描不完整")); list.append(heading);
    const paired = new Map(inventory.entries.filter(entry => entry.kind === "node").map(entry => [normalize(entry.parentPath), entry]));
    const navigation = inventory.entries.filter(entry => entry.kind !== "node" || !inventory!.entries.some(dir => dir.kind === "directory" && normalize(dir.path) === normalize(entry.parentPath))).map(entry => {
      const primary = entry.kind === "directory" ? paired.get(normalize(entry.path)) : null;
      return primary ? { ...primary, relativePath: entry.relativePath, name: entry.name, folderPath: entry.path } : { ...entry, folderPath: entry.kind === "directory" ? entry.path : null };
    });
    const matching = navigation.filter(entry => {
      if (selectedFolder && normalize(entry.path) !== normalize(selectedFolder) && !normalize(entry.path).startsWith(normalize(selectedFolder) + "/")) return false;
      if (!query.trim()) return true;
      const needle = query.toLocaleLowerCase(); return `${entry.name} ${entry.relativePath} ${entry.title ?? ""}`.toLocaleLowerCase().includes(needle);
    });
    const visible = matching.filter(entry => query || ![...collapsed].some(path => normalize(entry.path).startsWith(normalize(path) + "/")));
    for (const entry of visible) {
      const directory = entry.kind === "directory";
      const row = button("", () => directory ? browseFolder(entry.folderPath!) : openInventoryEntry(entry, inventory!.root,inventory!.workspaceId), `tree-row ${current && normalize(current.session.path) === normalize(entry.path) ? "active" : ""}`, entry.folderPath ? "folder" : "file");
      const depth = Math.max(0, entry.relativePath.replaceAll("\\", "/").split("/").length - 1); row.style.paddingLeft = `${12 + (query ? 0 : Math.min(depth, 5) * 14)}px`;
      const names = el("span", "tree-names"); names.append(el("span", "tree-name", entry.title || entry.name));
      if (query) names.append(el("span", "tree-path", entry.relativePath));
      row.append(names);
      if (entry.folderPath) { const expand = el("span", "tree-chevron", collapsed.has(entry.folderPath) ? "›" : "⌄"); expand.title = "展开或折叠子文件"; expand.onclick = event => { event.stopPropagation(); toggleFolder(entry.folderPath!); }; row.append(expand); }
      if (!["ordinary", "admitted"].includes(entry.identityStatus)) row.append(el("span", "identity-badge", labelStatus[entry.identityStatus] ?? entry.identityStatus));
      row.title = `${entry.path}\n${labelStatus[entry.identityStatus] ?? entry.identityStatus}`;
      if (entry.kind === "blocked" || !directory && !entry.path.toLowerCase().endsWith(".typ")) row.disabled = true;
      const menu = () => nodeMenu([entry], entry.folderPath ?? undefined);
      row.oncontextmenu = event => { event.preventDefault(); row.focus(); menu(); };
      row.onkeydown = event => { if (!event.isComposing && (event.key === "ContextMenu" || event.shiftKey && event.key === "F10")) { event.preventDefault(); menu(); } };
      if (entry.folderPath) {
        const wrapper = el("div", "tree-row-with-scope");
        const scopeButton = button("", () => browseFolder(entry.folderPath), "tree-scope-button", "search");
        scopeButton.setAttribute("aria-label", `仅浏览 ${entry.name} 文件夹`); scopeButton.title = `将 ${entry.name} 设为浏览范围`;
        const expandButton = button(collapsed.has(entry.folderPath) ? "›" : "⌄", () => toggleFolder(entry.folderPath!), "tree-scope-button");
        expandButton.setAttribute("aria-label", `展开或折叠 ${entry.name}`); expandButton.setAttribute("aria-expanded", String(!collapsed.has(entry.folderPath)));
        const more = button("⋯", menu, "tree-scope-button"); more.onkeydown=row.onkeydown;more.oncontextmenu=row.oncontextmenu;more.setAttribute("aria-label", `${entry.name} 更多操作`); wrapper.append(row, expandButton, scopeButton, more); list.append(wrapper);
      } else { const wrapper=el("div","tree-row-with-scope"); const more=button("⋯",menu,"tree-scope-button");more.onkeydown=row.onkeydown;more.oncontextmenu=row.oncontextmenu;more.setAttribute("aria-label",`${entry.name} 更多操作`);wrapper.append(row,more);list.append(wrapper); }
    }
    if (!visible.length) list.append(el("div", "search-empty", query ? `未找到「${query}」` : "这个文件夹还没有笔记"));
  }
  sidebar.append(list);
  if (inventory?.warnings.length) { const warnings = el("details", "library-warnings"); warnings.append(el("summary", "", `${inventory.warnings.length} 项资料库提示`)); for (const message of inventory.warnings) warnings.append(el("p", "", message)); sidebar.append(warnings); }
  const bottom = el("div", "sidebar-bottom"); bottom.append(button("创建资料库", createLibrary, "text-button", "plus"), button("恢复记录", showRecovery, "text-button", "undo")); sidebar.append(bottom);
  if (restoreSearch) { const newInput = sidebar.querySelector<HTMLInputElement>("#library-search")!; newInput.focus(); if (selection) newInput.setSelectionRange(selection[0], selection[1]); }
  navigationLayout.restoreSidebarState();
}
function toggleFolder(path: string): void { collapsed.has(path) ? collapsed.delete(path) : collapsed.add(path); renderSidebar(); }
function browseFolder(path: string | null): void {
  const old = selectedFolder ?? inventory?.root; if (old) folderViews.set(normalize(old), {query,scroll:sidebar.querySelector(".file-tree")?.scrollTop ?? 0});
  selectedFolder = path; contentSelections.clear(); const saved = folderViews.get(normalize(path ?? inventory?.root ?? "")); query = saved?.query ?? ""; renderSidebar();
  const tree=sidebar.querySelector(".file-tree"); if(tree)tree.scrollTop=saved?.scroll??0;
}
function nodeMenu(entries: InventoryEntry[], folder?: string): void {
  const selected = entries.filter(entry=>entry.path.toLowerCase().endsWith(".typ")), root=inventory?.root,workspaceId=inventory?.workspaceId;
  const commands: EverydayCommand[] = [
    {id:"node-open",label:"打开所选笔记",unavailable:!selected.length||!root?"所选项目没有可打开的 Typst 主文件。":undefined,run:async()=>{for(const entry of selected)await openInventoryEntry(entry,root!,workspaceId);}},
    {id:"node-preview",label:"预览所选项目",run:()=>previewNodes(entries)},
    {id:"node-copy-path",label:"复制所选完整路径",run:()=>navigator.clipboard.writeText(entries.map(entry=>entry.path).join("\n"))},
    {id:"node-operations",label:"整个节点：改名、移动、复制或回收",detail:"打开准确笔记后预览文件影响；未保存草稿须先处理",unavailable:selected.length!==1||!root?"此入口每次处理一个准确节点；多选不会隐式批量改写。":undefined,run:async()=>{await openInventoryEntry(selected[0],root!,workspaceId);await workspaceTools.show("knowledge");workspaceTools.selectKnowledgeTab("files");}},
  ];
  if(folder)commands.unshift({id:"node-browse",label:"浏览节点内容",run:()=>browseFolder(folder)});
  openEverydayMenu(entries.length===1?`${entries[0].title||entries[0].name} 操作`:`所选 ${entries.length} 项操作`,commands,toast,false);
}
async function previewNodes(entries:InventoryEntry[]):Promise<void>{await modal("项目预览","只读显示当前目录扫描中的摘要；不保存正文。",(form,close)=>{for(const entry of entries)form.append(el("strong","",entry.title||entry.name),el("pre","",`${entry.relativePath}\n${labelStatus[entry.identityStatus]??entry.identityStatus}\n${entry.excerpt??"当前扫描没有摘要，可明确打开正文查看。"}`));form.append(button("关闭",()=>close(null)));});}
function renderNodeContents(host: HTMLElement, folder: string): void {
  if(!inventory)return;const panel=el("section","node-contents");panel.setAttribute("aria-label","节点内容");
  const header=el("header");header.append(button(basename(inventory.root),()=>browseFolder(null)));
  const relative=folder.slice(inventory.root.length).replaceAll("\\","/").split("/").filter(Boolean);let parent=inventory.root;
  for(const name of relative){parent+="/"+name;const target=parent;header.append(el("span","","›"),button(name,()=>browseFolder(target)));}
  const sorting=el("select");sorting.setAttribute("aria-label","节点内容个人排序");for(const[value,label]of[["name","名称"],["path","路径"]]){const option=el("option","",label);option.value=value;sorting.append(option);}sorting.value=contentSort;sorting.onchange=()=>{contentSort=sorting.value as typeof contentSort;renderSidebar();};header.append(sorting);panel.append(header,el("p","","个人列表排序；作者结构顺序不受影响。"));
  const entries=inventory.entries.filter(entry=>normalize(entry.parentPath)===normalize(folder)&&normalize(entry.path)!==normalize(folder)).filter(entry=>!query||`${entry.title??""} ${entry.name} ${entry.relativePath}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())).sort((a,b)=>(contentSort==="name"?(a.title||a.name):(a.relativePath)).localeCompare(contentSort==="name"?(b.title||b.name):b.relativePath,"zh-CN"));
  const more=button("所选项目操作",()=>nodeMenu(entries.filter(entry=>contentSelections.has(entry.path))));more.disabled=!entries.some(entry=>contentSelections.has(entry.path));panel.append(more);
  const updateSelection=()=>{panel.querySelectorAll<HTMLElement>("[data-entry-path]").forEach(row=>{const selected=contentSelections.has(row.dataset.entryPath!);row.setAttribute("aria-selected",String(selected));row.querySelector<HTMLInputElement>("input")!.checked=selected;});more.disabled=!entries.some(entry=>contentSelections.has(entry.path));};
  for(const entry of entries){const row=el("div","node-content-row");row.tabIndex=0;row.dataset.entryPath=entry.path;row.setAttribute("aria-selected",String(contentSelections.has(entry.path)));const check=el("input");check.type="checkbox";check.checked=contentSelections.has(entry.path);check.setAttribute("aria-label",`选择 ${entry.name}`);check.onchange=()=>{check.checked?contentSelections.add(entry.path):contentSelections.delete(entry.path);updateSelection();};
    const open=()=>entry.kind==="directory"?browseFolder(entry.path):openInventoryEntry(entry,inventory!.root,inventory!.workspaceId);const title=button(`${entry.title||entry.name} · ${labelStatus[entry.identityStatus]??entry.identityStatus}`,()=>{contentSelections.clear();contentSelections.add(entry.path);updateSelection();});title.ondblclick=()=>{void Promise.resolve(open()).catch(showError);};
    const menu=()=>{if(!contentSelections.has(entry.path)){contentSelections.clear();contentSelections.add(entry.path);}nodeMenu(entries.filter(item=>contentSelections.has(item.path)),entry.kind==="directory"?entry.path:undefined);};row.oncontextmenu=event=>{event.preventDefault();menu();};row.onkeydown=event=>{if(event.isComposing)return;if(event.key==="Enter"&&event.target===row){event.preventDefault();void Promise.resolve(open()).catch(showError);}if(event.key==="ContextMenu"||event.shiftKey&&event.key==="F10"){event.preventDefault();menu();}};
    row.append(check,title,button("打开",open),button("预览",()=>previewNodes([entry])));panel.append(row);}
  if(!entries.length)panel.append(el("p","",query?"当前筛选没有匹配项目。":"此节点暂无内容。"));host.append(panel);
}

function renderTopbar(): void {
  // Writing blur can report state between pointerdown and click. Preserve the
  // actual action nodes while updating labels, so the pending click survives.
  const sessionId = current?.session.opened.sessionId ?? "";
  if (topbar.dataset.session === sessionId) {
    if (current) {
      const title = topbar.querySelector<HTMLElement>(".document-title")!;
      title.textContent = basename(current.session.path);
      const path = topbar.querySelector<HTMLElement>(".document-path")!;
      path.textContent = current.session.path; path.title = current.session.path;
      const state = topbar.querySelector<HTMLElement>(".save-state")!;
      state.className = `save-state ${current.session.dirty ? "dirty" : ""}`; state.textContent = saveState(current);
      const save = topbar.querySelector<HTMLButtonElement>('[data-command="save"]')!;
      save.disabled = current.session.savePending;
      save.lastChild!.textContent = current.session.savePending ? "保存中…" : "保存";
    }
    return;
  }
  topbar.dataset.session = sessionId;
  topbar.replaceChildren();
  const menu = button("", () => { shell.classList.toggle("sidebar-open"); }, "icon-button mobile-menu", "menu"); menu.setAttribute("aria-label", "显示资料库导航"); topbar.append(menu);
  const location = el("div", "document-location");
  if (current) {
    const name = el("div", "document-title", basename(current.session.path));
    const subtitle = el("span", "document-path", current.session.path); subtitle.title = current.session.path;
    location.append(name, subtitle);
  } else location.append(el("span", "home-label", "写作与知识"));
  topbar.append(location);
  const actions = el("div", "top-actions");
  if (current) {
    const state = el("span", `save-state ${current.session.dirty ? "dirty" : ""}`, saveState(current)); actions.append(state);
    actions.append(button("资料导入与识别", () => recognitionPanel.open(), "button-secondary compact"));
    actions.append(button("另存为", saveAs, "button-secondary compact"));
    const timeline = button("时间线", () => historyPanel.open(), "button-secondary compact");
    timeline.disabled = !current.session.opened.nodeId || !current.session.opened.workspaceId;
    actions.append(timeline);
    const saveButton = button(current.session.savePending ? "保存中…" : "保存", () => saveDocument(current!), "button-primary compact", "save"); saveButton.dataset.command = "save"; saveButton.disabled = current.session.savePending; actions.append(saveButton);
  } else actions.append(el("span", "top-note", "本地文件 · 原生 Typst"));
  topbar.append(actions);
}
function saveState(doc: LiveDocument): string { return doc.writing.state.pending ? "文字待确认" : doc.session.opened.recoveredDraftConflict ? "恢复草稿与磁盘冲突" : doc.session.savePending ? "保存中" : doc.lastSave ? "保存结果待确认" : doc.draftFailure ? "草稿恢复写入失败" : doc.session.recovered ? "已恢复草稿" : doc.session.dirty ? "未保存" : "已保存"; }
function renderTabs(): void {
  const ids = JSON.stringify(documents.map(doc => doc.session.opened.sessionId));
  if (tabs.dataset.ids === ids) {
    tabs.querySelectorAll<HTMLButtonElement>(".document-tab").forEach((tab, i) => {
      const doc = documents[i];
      tab.className = `document-tab ${doc === current ? "selected" : ""}`;
      tab.setAttribute("aria-current", doc === current ? "page" : "false");
      tab.querySelector<HTMLElement>(".tab-name")!.textContent = basename(doc.session.path);
      const dirty = tab.querySelector(".tab-dirty");
      if (doc.session.dirty && !dirty) tab.append(el("span", "tab-dirty", "•"));
      else if (!doc.session.dirty) dirty?.remove();
    });
    return;
  }
  tabs.dataset.ids = ids;
  tabs.replaceChildren(); tabs.hidden = !documents.length;
  for (const doc of documents) {
    const tab = button("", () => activateDocument(doc), `document-tab ${doc === current ? "selected" : ""}`, "file"); tab.setAttribute("aria-current", doc === current ? "page" : "false"); tab.append(el("span", "tab-name", basename(doc.session.path))); if (doc.session.dirty) tab.append(el("span", "tab-dirty", "•")); tabs.append(tab);
  }
  if (documents.length) { const add = button("", openOutsideFile, "icon-button tab-add", "plus"); add.setAttribute("aria-label", "打开另一个文件"); tabs.append(add); }
}
function renderStatus(): void {
  statusbar.replaceChildren();
  if (!current) { statusbar.append(el("span", "", "文件保存在你的设备上"), el("span", "", "Weftext 本机预览版")); return; }
  const doc = current; const session = doc.session;
  const left = el("div", "status-left");
  const chars = [...session.source].length; left.append(el("span", "", `${chars.toLocaleString()} 字符`), el("span", "status-kind", session.opened.nodeId ? "资料库笔记" : "普通 Typst 文件"));
  const right = el("div", "status-right");
  if (doc.writing.state.pending || doc.writing.state.busy) right.append(el("span", "pending-label", doc.writing.state.busy ? "文字更新中…" : "文字正在同步至草稿"));
  else if (!sourceConsumersAvailable(doc)) right.append(el("span", "pending-label", doc.sourceRefreshing ? "正在核对同步结果" : "预览与批注等待明确比较"));
  else if (doc.rendering) right.append(el("span", "pending-label", "预览更新中"));
  else if (session.error && !session.projectionCurrent) right.append(el("span", "error-label", session.error));
  else if (!session.projectionCurrent) right.append(el("span", "pending-label", "预览更新中"));
  else if (session.projection?.ok) right.append(icon("check"), el("span", "", "草稿预览已更新"));
  else right.append(icon("alert"), el("span", "", "预览有错误 · 源码已保留"));
  if (doc.draftFailure) right.append(el("span", "error-label", `恢复草稿未写入：${doc.draftFailure}`));
  else if (session.dirty && session.lastDraftSource === session.source) right.append(el("span", "", "恢复草稿已留存"));
  statusbar.append(left, right);
}

function renderWelcome(): void {
  const welcome = el("section", "welcome");
  const eyebrow = el("div", "eyebrow", "WEFTEXT / 你的写作空间");
  welcome.append(eyebrow, el("h1", "", "让想法有处安放。"), el("p", "welcome-intro", "从一页文字开始，在自己的文件里整理知识。\n以原生 Typst 写作，自由切换源码、编辑与阅读。"));
  const cards = el("div", "welcome-cards");
  for (const [label, description, action, iconName] of [
    ["打开资料库", "继续写作，浏览已有笔记", chooseLibrary, "book"],
    ["创建资料库", "为新的想法准备一个文件夹", createLibrary, "plus"],
    ["打开 .typ 文件", "直接编辑设备上的普通文件", openOutsideFile, "file"],
  ] as const) {
    const card = button("", action, "welcome-card"); const cardIcon = el("div", "welcome-card-icon"); cardIcon.append(icon(iconName)); const text = el("div", "welcome-card-text"); text.append(el("strong", "", label), el("span", "", description)); card.append(cardIcon, text, icon("arrow")); cards.append(card);
  }
  welcome.append(cards);
  if (startupError) { const error = el("div", "startup-error"); error.append(icon("alert"), el("p", "", startupError), button("重试连接", bootstrap, "button-secondary")); welcome.append(error); }
  if (inventory) { const section = el("div", "welcome-library"); section.append(el("h2", "", basename(inventory.root)), el("p", "", !inventory.workspaceId ? "普通文件夹已打开。可以从左侧浏览与编辑 .typ 文件。" : inventory.complete ? "资料库已打开，从左侧选择文件，或新建笔记。" : "资料库部分扫描完成，已找到的文件可以打开。"), inventory.workspaceId ? button("新建笔记", createNote, "button-primary", "plus") : button("创建新的资料库", createLibrary, "button-secondary", "plus")); welcome.append(section); }
  const footnote = el("div", "welcome-footnote"); footnote.append(el("span", "footnote-dot"), document.createTextNode("本机运行 · 中文默认支持 · 草稿自动留存")); welcome.append(footnote); content.append(welcome);
}

function documentComparisonMessage(doc: LiveDocument): string | null {
  if (sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget)) return sourceMoveComparisonMessage;
  return doc.comparisonTarget && doc.comparisonTarget.source !== doc.session.baseSource
    ? "磁盘内容已改变。当前草稿与撤销记录保留，请比较后继续保存。" : null;
}

const noteInputRecovery = new NoteInputRecovery({ getItem: key => sessionStorage.getItem(key), setItem: (key, value) => sessionStorage.setItem(key, value) });
function localPersonalAudience(): string {
  let user = localStorage.getItem("weftext-local-personal-id"); if (!user) { user = crypto.randomUUID(); localStorage.setItem("weftext-local-personal-id", user); }
  return `local:${user}`;
}
function localPersonalNumbering(doc: LiveDocument) {
  return personalNumberingKeys(localPersonalAudience(), doc.session.opened.workspaceId ?? normalize(doc.session.opened.root), doc.session.opened.nodeId ?? normalize(doc.session.path));
}
function localAuthoringEnvironment(doc: LiveDocument): AuthoringPanelsEnvironment {
  const readable = () => current === doc && sourceConsumersAvailable(doc) && !doc.sourceTrashed;
  return {
    controller: doc.writing, readable, editable: () => readable() && !readOnlySource(doc) && doc.mode !== "read" && !doc.session.savePending && !doc.lastSave,
    projection: () => readable() && doc.session.projectionCurrent ? doc.session.projection : null,
    selection: () => doc.editor.selection, requestEdit: request => api("/api/edit", request), previewProposal: request => api("/api/editor/preview", request),
    revealSource: range => revealWritingSource(doc, range), revealRendered: key => {
      if (!readable() || !renderHost || !renderHost.isConnected || renderHost.getClientRects().length === 0) return false; const target = Array.from(renderHost.querySelectorAll<HTMLElement>("[data-document-node-key]")).find(item => item.dataset.documentNodeKey === key); if (!target) return false; target.scrollIntoView({ block: "center", inline: "nearest" }); target.focus({ preventScroll: true }); return true;
    },
    ensureEndnoteModule: async () => { await doc.authoringAssets.install(); toast("本篇尾注模块资源已核对；正文与撤销记录保留，请重新打开尾注提案。"); },
    recoverEndnoteModule: async () => { const receipt = await doc.authoringAssets.recover(); toast(receipt ? `原尾注模块操作：${receipt.status}` : "当前没有待核对的尾注模块安装。"); },
    retryEndnoteModule: async () => { await doc.authoringAssets.retryOriginal(); toast("原尾注模块安装已确认，正文尚未保存。"); },
    restartEndnoteModule: async () => { await doc.authoringAssets.restartAfterCancelled(); toast("已按明确取消的原操作核对并准备新模块资源；正文尚未保存。"); },
    settleEndnoteModule: async () => { const receipt = await doc.authoringAssets.settleOriginal(); toast(`原模块操作：${receipt.status}`); },
    pendingInput: value => { noteInputRecovery.add(localPersonalAudience(), value); toast("未应用的新注释输入已保留，可在“恢复记录”中取回。"); }, onError: showError,
  };
}
async function openLocalNumbering(doc: LiveDocument): Promise<void> {
  await doc.writing.deferUntilCompositionEnds(() => doc.writing.flush()); if (current !== doc) throw new Error("当前笔记已切换，未打开旧编号提案。");
  const keys = localPersonalNumbering(doc);
  await openNumberingPanel(numberingHost(localAuthoringEnvironment(doc), { ...keys, display: enabled => { if (current === doc && renderHost) displayPersonalHeadings(renderHost, enabled); } }));
}

function renderDocument(): void {
  if (!current) return;
  const doc = current;
  const comparisonMessage = documentComparisonMessage(doc);
  if (comparisonMessage) doc.session.error = comparisonMessage;
  const controls = el("div", "document-controls");
  const viewSwitch = el("div", "view-switch"); viewSwitch.setAttribute("role", "group"); viewSwitch.setAttribute("aria-label", "文档视图");
  for (const mode of modes) { const choice = button(mode.title, () => changeMode(mode.id), doc.mode === mode.id ? "view-choice active" : "view-choice"); choice.title = `${mode.short} · ${mode.hint}`; choice.setAttribute("aria-pressed", String(doc.mode === mode.id)); viewSwitch.append(choice); }
  controls.append(viewSwitch);
  const layout = el("select", "layout-choice"); layout.setAttribute("aria-label", "文档布局");
  for (const [value, label] of [["continuous", "连续布局"], ["paged", "分页预览"]]) { const option = el("option", "", label); option.value = value; layout.append(option); }
  layout.value = doc.layout;
  layout.onchange = () => { void doc.writing.deferUntilCompositionEnds(async () => { await doc.writing.flush(); doc.layout = layout.value as LiveDocument["layout"]; renderContent(); }).catch(showError); }; controls.append(layout);
  if (doc.mode === "source") { const preview = button(previewVisible ? "收起预览" : "显示预览", () => { previewVisible = !previewVisible; localStorage.setItem("weftext-preview", String(previewVisible)); renderContent(); }, "text-button preview-toggle", "eye"); controls.append(preview); }
  content.append(controls);
  const caption = el("div", "view-caption"); caption.append(el("span", "", modes.find(m => m.id === doc.mode)!.hint));
  if (doc.mode === "read" || readOnlySource(doc)) caption.append(el("span", "readonly-label", readOnlySource(doc) ? "回收站内容 · 只读" : "只读 · 当前草稿"));
  if (doc.session.recovered) caption.append(button("已恢复上次草稿 · 查看", showDraftComparison, "recovery-caption"));
  if (noteInputRecovery.count(localPersonalAudience())) caption.append(button("取回未应用的注释输入", showRecovery, "recovery-caption"));
  content.append(caption);
  doc.notesMount?.dispose(); const authoring = el("section", "document-authoring-controls"), notes = el("div", "document-notes-summary"); authoring.append(button("编号控制", () => openLocalNumbering(doc), "text-button"), notes); content.append(authoring); doc.notesMount = mountDocumentNotes(notes, documentNotesHost(localAuthoringEnvironment(doc)));
  if (doc.session.opened.recoveredDraftConflict) {
    const conflict = el("div", "document-error document-conflict"); conflict.append(icon("alert"), el("span", "", "恢复草稿与当前磁盘内容不同。请比较并明确合并后再保存。"), button("比较并合并", showDraftComparison, "button-secondary compact")); content.append(conflict);
  }
  if (doc.session.error) { const error = el("div", "document-error"); error.append(icon("alert"), el("span", "", doc.session.error), doc.saveAsChangedTarget ? button("打开另存为目标", () => openDocument(doc.saveAsChangedTarget!), "text-button") : button("比较磁盘内容", showDraftComparison, "text-button")); content.append(error); }
  const body = el("div", `document-body mode-${doc.mode} ${previewVisible ? "with-preview" : ""}`); content.append(body);
  if (doc.layout === "paged") {
    if (["source", "styled"].includes(doc.mode)) { const sourcePane = el("section", "source-pane"); sourcePane.append(sourceToolbar(doc)); const host = el("div", "source-host"); sourcePane.append(host); doc.editor.mount(host); body.append(sourcePane); }
    const host = el("section", "paged-host"); body.append(host);
    pagedPreview = new PagedPreview(host, { basis: () => current === doc && sourceConsumersAvailable(doc) ? { sessionId: doc.session.opened.sessionId, source: doc.session.source, serial: doc.session.serial } : null, request: basis => api("/api/paged", basis), fileName: () => basename(doc.session.path).replace(/\.typ$/i, ".pdf"), onError: showError });
    void pagedPreview.render();
    if (doc.queries.hasQueries) {
      const flow = el("section", "flow-pane paged-query-flow"); flow.append(el("p", "embedded-query-help", "此处原生分页预览不含查询结果。需要随文 PDF 时，在“输出与导入”选择“PDF 文档（含已显示正文查询）”，固定当前结果后生成。")); renderHost = el("div", "flow-document preview-document"); renderHost.setAttribute("aria-label", "含正文查询的连续草稿预览"); flow.append(renderHost); body.append(flow); flowScroll = flow; renderProjection(doc);
    }
  } else if (["source", "styled"].includes(doc.mode)) {
    const sourcePane = el("section", "source-pane"); sourcePane.setAttribute("aria-label", "源码编辑区"); sourcePane.append(sourceToolbar(doc)); const host = el("div", "source-host"); sourcePane.append(host); doc.editor.mount(host); body.append(sourcePane);
    if (previewVisible || doc.mode === "styled") { const pane = el("section", "preview-pane"); const header = el("div", "pane-heading"); header.append(el("span", "", "草稿预览"), el("span", "", "连续阅读")); pane.append(header); renderHost = el("div", "flow-document preview-document"); renderHost.setAttribute("aria-label", "草稿流式预览"); pane.append(renderHost); body.append(pane); flowScroll = pane; renderProjection(doc); }
  } else {
    const flow = el("section", "flow-pane"); renderHost = el("div", "flow-document"); renderHost.setAttribute("aria-label", doc.mode === "read" ? "只读草稿" : "可视草稿"); flow.append(renderHost); body.append(flow); flowScroll = flow; renderProjection(doc);
    if (doc.mode === "write" && !readOnlySource(doc)) {
      body.before(createWritingToolbar(doc.writing, { onError: showError, insertLink, revealSource: range => revealWritingSource(doc, range) }));
    }
    if (doc.mode === "focus" && activeSourceOrigin) {
      const source = el("section", "focus-source"); const heading = el("div", "pane-heading"); heading.append(el("span", "", "当前构造的语法"), button("收起语法", () => { doc.editor.clearReveal(); activeSourceOrigin = null; renderContent(); }, "text-button", "close")); source.append(heading, sourceToolbar(doc)); const host = el("div", "source-host"); source.append(host); body.append(source); doc.editor.mount(host);
    }
  }
  if (doc.session.projectionCurrent && doc.session.projection!.diagnostics.length) renderDiagnostics(doc);
  if (doc.session.projectionCurrent && sourceConsumersAvailable(doc)) doc.editor.tokens(doc.session.projection!.sourceInfo.syntaxTokens, doc.mode === "styled", doc.editor.syntaxBasis());
  else if (!sourceConsumersAvailable(doc) || doc.sourceTrashed) doc.editor.clearTokens();
  doc.editor.styleMode(doc.mode === "styled");
  doc.editor.readOnly(doc.mode === "read" || doc.mode === "write" || readOnlySource(doc));
  const scroller=flowScroll, mode=doc.mode;
  requestAnimationFrame(() => { if(current!==doc || doc.mode!==mode)return; if(scroller && flowScroll===scroller && doc.scroll[mode]!=null)scroller.scrollTop=doc.scroll[mode]!; if(["source","styled"].includes(mode)&&doc.sourceScroll!=null)doc.editor.view.scrollDOM.scrollTop=doc.sourceScroll; });
}
function sourceToolbar(doc: LiveDocument): HTMLElement {
  const toolbar = el("div", "source-toolbar"); toolbar.setAttribute("role", "toolbar"); toolbar.setAttribute("aria-label", "源码格式工具");
  const actions: [string, string, string, string][] = [["粗体", "*", "*", "文字"], ["斜体", "_", "_", "文字"], ["删除线", "#strike[", "]", "文字"], ["标题", "= ", "", "章节标题"], ["列表", "- ", "", "列表内容"], ["公式", "$ ", " $", "x + y"]];
  for (const [label, before, after, placeholder] of actions) {
    const control = button(label, () => { if (!readOnlySource(doc) && !doc.editor.view.composing) doc.editor.insert(before, after, placeholder); }, `format-button format-${label}`);
    control.disabled = readOnlySource(doc); toolbar.append(control);
  }
  const link = button("链接", insertLink, "format-button", "link"); link.disabled = readOnlySource(doc); toolbar.append(link);
  const spacer = el("span", "toolbar-spacer"); toolbar.append(spacer, el("span", "keyboard-hint", "Ctrl S 保存 · Ctrl Z 撤销")); return toolbar;
}

function renderContent(): void {
  const recognitionCurrent = current?.session.opened.root ?? "";
  if (recognitionSession !== recognitionCurrent) { recognitionPanel.retireDisplay(true); recognitionSession = recognitionCurrent; }
  if (flowScroll && flowHasProjection && renderedDocument && renderedMode) renderedDocument.scroll[renderedMode] = flowScroll.scrollTop;
  if(renderedDocument && renderedMode && ["source","styled"].includes(renderedMode))renderedDocument.sourceScroll=renderedDocument.editor.view.scrollDOM.scrollTop;
  renderedDocument = current;
  renderedMode = current?.mode ?? null;
  flowHasProjection=false;
  pagedPreview?.dispose(); pagedPreview = null; workspaceTools.attachSurface(null); projectionMappings = [];
  for (const doc of documents) doc.queries.detach();
  renderHost = null; flowScroll = null; content.replaceChildren(); current ? renderDocument() : renderWelcome(); renderTopbar(); renderTabs(); renderStatus(); renderEverydayBar();
}
function refreshChrome(): void {
  const comparisonMessage = current && documentComparisonMessage(current);
  if (current && comparisonMessage) {
    current.session.error = comparisonMessage;
    let notice = content.querySelector<HTMLElement>(".document-error:not(.document-conflict)");
    if (!notice) {
      notice = el("div", "document-error"); notice.append(icon("alert"), el("span", "", comparisonMessage), button("比较磁盘内容", showDraftComparison, "text-button"));
      content.querySelector(".document-body")?.before(notice);
    } else { const message = notice.querySelector("span"); if (message) message.textContent = comparisonMessage; }
  }
  renderTopbar(); renderTabs(); renderStatus();
  if (current) refreshProjectionNotice(current);
  if (current && !current.session.recovered) content.querySelector(".recovery-caption")?.remove();
  if (current && !current.session.error) content.querySelector(".document-error:not(.document-conflict)")?.remove();
  if (current && !current.session.opened.recoveredDraftConflict) content.querySelector(".document-conflict")?.remove();
}

function projectionStaleMessage(doc: LiveDocument): string {
  return doc.session.error
    ? `显示上次成功预览，当前草稿预览失败；此预览只读。${doc.session.error}`
    : "显示上次预览，正在更新。文字编辑暂不可用。";
}
function fillProjectionEmpty(host: HTMLElement, doc: LiveDocument): void {
  host.replaceChildren();
  if (doc.session.error) host.append(icon("alert"), el("h3", "", "草稿暂时无法预览"), el("p", "projection-message", doc.session.error), button("查看源码", () => changeMode("source"), "button-secondary", "code"));
  else host.append(el("div", "loading-ring"), el("p", "", "正在编译草稿…"));
}
// Update only the notice: retain the source editor, active input and scroll.
function refreshProjectionNotice(doc: LiveDocument): void {
  if (doc !== current || !renderHost || !sourceConsumersAvailable(doc) || doc.session.projectionCurrent) return;
  if (doc.session.projection) {
    let notice = renderHost.querySelector<HTMLElement>(":scope > .preview-stale");
    if (!notice) { notice = el("div", "preview-stale"); renderHost.prepend(notice); }
    const message = projectionStaleMessage(doc); if (notice.textContent !== message) notice.textContent = message;
  } else {
    const notice = renderHost.querySelector<HTMLElement>(":scope > .projection-empty");
    if (!notice) return;
    if (doc.session.error ? notice.querySelector(".projection-message")?.textContent !== doc.session.error : !notice.querySelector(".loading-ring")) fillProjectionEmpty(notice, doc);
  }
}
function renderProjection(doc: LiveDocument): void {
  if (doc !== current || !renderHost) return;
  if (doc.writing.retainInput || doc.editor.composing) { renderStatus(); return; }
  const restoreQuerySourceFocus = doc.mode === "focus" && !!doc.editor.revealed && doc.editor.view.hasFocus && !!doc.editor.view.dom.closest(".embedded-query-source-host");
  const preservedScroll=flowHasProjection?flowScroll?.scrollTop:doc.scroll[doc.mode];
  const host = renderHost; host.replaceChildren();flowHasProjection=false;
  if (!sourceConsumersAvailable(doc)) {
    const waiting = el("div", "projection-empty"); waiting.append(el("p", "", doc.sourceRefreshing ? "正在核对同步结果，当前草稿保留。" : "笔记位置已改变，请先比较磁盘内容并明确合并；当前源码和撤销记录保留。")); host.append(waiting); return;
  }
  const projection = doc.session.projection;
  if (!projection) { const waiting = el("div", "projection-empty"); fillProjectionEmpty(waiting, doc); host.append(waiting); return; }
  if (!doc.session.projectionCurrent) { const stale = el("div", "preview-stale", projectionStaleMessage(doc)); host.append(stale); }
  if (!projection.ok || !projection.tree) {
    const failed = el("div", "projection-empty"); failed.append(icon("alert"), el("h3", "", "草稿暂时无法预览"), el("p", "", "完整输入保留在当前窗口；恢复草稿和保存状态以单独提示为准。可使用源码入口继续编辑。"), button("查看源码", () => changeMode("source"), "button-secondary", "code")); host.append(failed); return;
  }
  let writableCount = 0, personalHeadingOrdinal = 0;
  const renderedTree = projection.tree, renderedSource = projection.source;
  projectionMappings = [];
  const groups = doc.session.projectionCurrent && doc.writing.map ? groupWritingRenderNodes(projection.tree, doc.writing.map, doc.session.source) : [];
  const grouped = new Map(groups.flatMap(group => group.memberKeys.map(key => [key, group] as const)));
  const emptySlots = new Map((doc.session.projectionCurrent && doc.writing.map ? groupWritingEmptySlots(projection.tree, doc.writing.map, doc.session.source) : []).map(slot => [slot.nodeKey, slot.region]));
  doc.queries.beginProjection(projection.tree, { sessionId: doc.session.opened.sessionId, source: projection.source, serial: projection.serial });
  function draw(node: RenderNode): Node {
    const query = doc.queries.forNode(node); if (query) return query;
    if (node.kind === "tag") return document.createDocumentFragment();
    if (node.kind === "text") {
      const text = node.text ?? "";
      const group = grouped.get(node.key);
      const region = group?.region;
      if (doc.mode === "write" && !readOnlySource(doc) && group && region) {
        if (group.firstKey !== node.key) return document.createDocumentFragment();
        const input = el("textarea", "literal-input"); input.rows = 1; input.setAttribute("aria-label", "编辑段落文字"); input.dataset.nodeKey = node.key; input.spellcheck = false;
        doc.writing.bindLiteral(input, region); writableCount++; return input;
      }
      if (doc.mode === "focus" && node.origin?.isMain) {
        const span = el("span", "focus-text", text); span.tabIndex = 0; span.setAttribute("role", "button"); span.setAttribute("aria-label", `展开源码：${text.slice(0, 30)}`); span.title = "点击或按 Enter 展开源码";
        const reveal = () => revealSource(node.origin!); span.onclick = reveal; span.onkeydown = event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); reveal(); } }; projectionMappings.push({ nodeKey: node.key, node: span, text, origin: node.origin }); return span;
      }
      const leaf = document.createTextNode(text); projectionMappings.push({ nodeKey: node.key, node: leaf, text, origin: node.origin }); return leaf;
    }
    if (node.kind === "frame") {
      const island = el("div", "layout-island"); island.append(el("span", "", "此布局需要分页呈现")); if (doc.mode !== "read") island.append(button("查看源码", () => node.origin ? revealSource(node.origin) : changeMode("source"), "text-button", "code")); return island;
    }
    const tag = node.tag?.toLowerCase() ?? "span";
    const element = nativeElement(node);
    element.setAttribute("data-document-node-key", node.key);
    const semanticRole = node.attrs["data-weftext-role"], noteId = node.attrs["data-weftext-note-id"], targetId = node.attrs["data-weftext-target-id"];
    if (semanticRole === "heading") markPersonalHeading(element, ++personalHeadingOrdinal);
    if (doc.session.projectionCurrent && typeof noteId === "string" && ["footnote-reference", "endnote-reference"].includes(String(semanticRole))) {
      element.setAttribute("role", "button"); element.setAttribute("tabindex", "0"); element.setAttribute("aria-label", `查看${semanticRole === "endnote-reference" ? "尾注" : "脚注"}正文与全部标记`);
      const open = () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), semanticRole === "endnote-reference" ? "endnote" : "footnote", noteId);
      element.addEventListener("click", event => { event.preventDefault(); event.stopPropagation(); void open().catch(showError); }, true);
      element.addEventListener("keydown", event => { if (event instanceof KeyboardEvent && !event.isComposing && !doc.writing.state.composing && ["Enter", " "].includes(event.key)) { event.preventDefault(); event.stopPropagation(); void open().catch(showError); } });
    }
    const target = node.attrs.href;
    if (tag === "a" && typeof target === "string") { element.setAttribute("href", "#"); element.setAttribute("title", target); element.addEventListener("click", event => { event.preventDefault(); void navigateLink(target).catch(showError); }); }

    for (const child of node.children ?? []) element.append(draw(child));
    if (node.text) element.append(document.createTextNode(node.text));
    if (doc.session.projectionCurrent && typeof targetId === "string" && ["figure", "equation"].includes(String(semanticRole))) { const actions = el("span", "document-notes-object-actions"); actions.append(button(semanticRole === "equation" ? "公式编号与题注" : "此对象题注与编号", () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), "caption", targetId), "text-button")); element.append(actions); }
    if (doc.session.projectionCurrent && typeof noteId === "string" && ["footnote-body", "endnote-body"].includes(String(semanticRole))) element.append(button("查看正文标记与注释", () => openDocumentNotes(documentNotesHost(localAuthoringEnvironment(doc)), semanticRole === "endnote-body" ? "endnote" : "footnote", noteId), "text-button"));
    const empty = emptySlots.get(node.key);
    if (doc.mode === "write" && !readOnlySource(doc) && empty) { const input = el("textarea", "literal-input empty-slot-input"); input.rows = 1; input.setAttribute("aria-label", "编辑空内容"); doc.writing.bindLiteral(input, empty); element.append(input); writableCount++; }
    if (doc.mode === "write" && !readOnlySource(doc) && doc.session.projectionCurrent && node.origin?.isMain) {
      const range = doc.writing.map?.structures.find(item => item.range.utf16From === node.origin!.utf16From && item.range.utf16To === node.origin!.utf16To)?.range;
      if (range) { const controls = createStructureControls(doc.writing, range, { onError: showError, revealSource: slot => revealWritingSource(doc, slot), renderedTable: element }); if (controls) { const wrapper = el("div", "writing-structure"); if (controls.classList.contains("writing-table-surface")) wrapper.append(controls); else wrapper.append(element, controls); return wrapper; } }
    }
    if (doc.mode === "write" && node.origin?.isMain && !node.writable && ["pre", "table", "figure"].includes(tag)) {
      const wrapper = el("div", "readonly-construct"); wrapper.append(element, button("此部分只读 · 查看源码", () => revealSource(node.origin!), "source-entry", "code")); return wrapper;
    }
    if (tag === "math" && doc.session.projectionCurrent) {
      const wrapper = el("div", "math-discussion-target");
      const isCurrent = (): boolean => doc === current && doc.session.projection === projection && doc.session.projectionCurrent && sourceConsumersAvailable(doc);
      wrapper.append(element, createMathSelectionControls({ math: node, tree: renderedTree, source: renderedSource,
        current: isCurrent, notify: message => toast(message, true), select: selection => {
          if (!isCurrent()) return;
          const structure = doc.writing.map?.structures.find(item => item.range.utf16From === selection.from && item.range.utf16To === selection.to || item.body?.utf16From === selection.from && item.body?.utf16To === selection.to);
          readingSelection = { sessionId: doc.session.opened.sessionId, serial: doc.session.serial, selection: { from: selection.from, to: selection.to, visibleExcerpt: selection.visibleExcerpt, nodeKey: selection.nodeKey, elementRole: structure?.kind } };
          doc.editor.select(selection.from, selection.to, false); toast("已选择准确公式范围，可在链接与批注中添加讨论。");
        } })); return wrapper;
    }
    return element;
  }
  host.append(draw(projection.tree));
  displayPersonalHeadings(host, personalNumberingVisible(localPersonalNumbering(doc)));
  if (restoreQuerySourceFocus && doc.editor.view.dom.isConnected) doc.editor.view.focus();
  host.onmouseup = captureReadingSelection; host.onkeyup = captureReadingSelection;
  void workspaceTools.projectionChanged(host, projectionMappings).catch(showError);
  if (doc.mode === "write" && !readOnlySource(doc)) {
    if (doc.session.projectionCurrent && doc.writing.map?.bodyInsertion?.writable) {
      const input = el("textarea", "literal-input append-input"); input.rows = 1; input.placeholder = writableCount ? "继续写作…" : "从这里开始写作…"; input.setAttribute("aria-label", "新段落文字输入");
      doc.writing.bindBodyInput(input); host.append(input);
    }
    if (!writableCount && !(projection.sourceInfo.bodyInsertion?.writable)) { const fallback = el("div", "rich-readonly"); fallback.append(icon("code"), el("p", "", "当前内容没有可安全回写的文字区域。源码保留完整，可切换源码继续编辑。"), button("编辑源码", () => changeMode("source"), "button-secondary")); host.append(fallback); }
    else host.append(el("p", "rich-scope-note", "可编辑文字直接输入；生成内容、复杂布局与未确认区域保持只读。"));
  }
  flowHasProjection=true;
  const scroller=flowScroll,mode=doc.mode;
  if(scroller&&preservedScroll!=null){scroller.scrollTop=preservedScroll;const installed=scroller.scrollTop;requestAnimationFrame(()=>{if(current!==doc||doc.mode!==mode||flowScroll!==scroller||renderHost!==host||!flowHasProjection||scroller.scrollTop!==installed)return;scroller.scrollTop=preservedScroll;doc.scroll[mode]=scroller.scrollTop;});}
}
function captureReadingSelection(): void {
  const doc = current; const selection = window.getSelection();
  if (!doc || !doc.session.projectionCurrent || !selection || selection.isCollapsed || selection.rangeCount !== 1) return;
  const range = selection.getRangeAt(0);
  // This UI path only supplies a direct character map. More complex generated
  // or escaped appearances remain source-selectable instead of guessing offsets.
  const endpoint = (node: Node, offset: number): number | null => {
    const matches = projectionMappings.filter(item => item.node === node || item.node.firstChild === node);
    if (matches.length !== 1) return null;
    const item = matches[0], origin = item.origin;
    if (!origin?.isMain || origin.raw !== item.text || doc.session.source.slice(origin.utf16From, origin.utf16To) !== item.text || offset < 0 || offset > item.text.length) return null;
    if (projectionMappings.filter(other => other.origin?.isMain && other.origin.utf16From === origin.utf16From && other.origin.utf16To === origin.utf16To).length !== 1) return null;
    return origin.utf16From + offset;
  };
  const from = endpoint(range.startContainer, range.startOffset), to = endpoint(range.endContainer, range.endOffset);
  if (from == null || to == null || from >= to) return;
  readingSelection = { sessionId: doc.session.opened.sessionId, serial: doc.session.serial, selection: { from, to, visibleExcerpt: selection.toString(), elementRole: "text" } };
}

function renderDiagnostics(doc: LiveDocument): void {
  content.querySelector(".diagnostics")?.remove();
  const details = el("details", "diagnostics"); const diagnostics = doc.session.projection!.diagnostics; const errors = diagnostics.filter(d => d.severity === "error").length;
  details.open = errors > 0;
  details.append(el("summary", "", `${errors ? `${errors} 个错误` : `${diagnostics.length} 项提示`} · 官方 Typst ${doc.session.projection!.compilerVersion}`));
  for (const diagnostic of diagnostics) { const item = button(diagnostic.message, () => diagnostic.origin ? revealSource(diagnostic.origin) : changeMode("source"), `diagnostic-item ${diagnostic.severity}`, diagnostic.severity === "error" ? "alert" : "code"); details.append(item); }
  content.append(details);
}

async function changeMode(mode: ViewMode): Promise<void> {
  if (!current) return;
  if (current.writing.state.composing) { const doc = current; await doc.writing.deferUntilCompositionEnds(() => { if (current === doc) return changeMode(mode); }); return; }
  if (mode === "source" || mode === "styled") await current.writing.settleForSessionChange();
  else await flushLiteral();
  if (flowScroll&&flowHasProjection) current.scroll[current.mode] = flowScroll.scrollTop;
  current.editor.clearReveal(); current.mode = mode; current.queries.modeChanged(); activeSourceOrigin = null; localStorage.setItem("weftext-view", mode); renderContent();
}
function revealSource(origin: Origin): void {
  if (!current || !origin.isMain || !current.session.projectionCurrent) return;
  current.queries.closeExpansion();
  if (current.mode !== "focus") current.mode = "source";
  else activeSourceOrigin = origin;
  if (current.mode === "focus") {
    const map = current.writing.map;
    const construct = map?.structures.filter(item => item.range.utf16From <= origin.utf16From && item.range.utf16To >= origin.utf16To).sort((a, b) => (a.range.to - a.range.from) - (b.range.to - b.range.from))[0]?.range;
    const slot = construct ?? { ...origin, writable: true };
    if (!current.editor.reveal({ from: slot.utf16From, to: slot.utf16To, raw: slot.raw }, current.session.source)) return;
  } else current.editor.clearReveal();
  renderContent(); current.editor.select(Math.min(origin.utf16From, current.session.source.length), Math.min(origin.utf16To, current.session.source.length));
}

function revealWritingSource(doc: LiveDocument, range: WritingRange): void {
  if (doc !== current) return;
  revealSource({ ...range, path: doc.session.path, syntaxKind: "structure", isMain: true });
}

function scheduleDraft(doc: LiveDocument): void { clearTimeout(doc.draftTimer); doc.draftTimer = setTimeout(() => { void persistDraft(doc).catch(showError); }, 450); }
async function persistDraft(doc: LiveDocument, proposedSource?: string): Promise<void> {
  clearTimeout(doc.draftTimer);
  const source = proposedSource ?? doc.writing.recoverySource() ?? doc.session.source; const serial = doc.session.serial;
  const request = doc.draftChain.catch(() => undefined).then(async () => {
    await api("/api/draft", { sessionId: doc.session.opened.sessionId, source });
    doc.session.acknowledgeDraft(source, serial); doc.draftFailure = null;
  });
  doc.draftChain = request;
  try { await request; } catch (error) {
    const failure = error instanceof ApiError && error.code === "tooLarge"
      ? new ApiError(error.code, "源码超过 2 MiB 上限；完整输入仅保留在当前窗口，这份超限输入未写入恢复草稿或文件。请撤销或缩短内容后重试。", error.details, error.currentSource)
      : error;
    doc.draftFailure = failure instanceof Error ? failure.message : String(failure); throw failure;
  } finally { if (doc === current) renderStatus(); }
}
function scheduleRender(doc: LiveDocument, delay = 300): void {
  doc.renderGeneration++; clearTimeout(doc.renderTimer);
  if (!sourceConsumersAvailable(doc)) { if (doc === current) refreshChrome(); return; }
  doc.renderTimer = setTimeout(() => { void compile(doc); }, delay);
}
async function compile(doc: LiveDocument): Promise<void> {
  clearTimeout(doc.renderTimer); if (!sourceConsumersAvailable(doc)) return;
  const source = doc.session.source; const serial = doc.session.serial; const generation = doc.renderGeneration; const sessionId = doc.session.opened.sessionId; const syntaxBasis = doc.editor.syntaxBasis(); doc.rendering++; if (doc === current) renderStatus();
  if (doc === current && doc.layout === "paged") void pagedPreview?.render({ sessionId: doc.session.opened.sessionId, source, serial });
  try {
    const basis = { sessionId: doc.session.opened.sessionId, source, serial };
    const hadQueries = doc.queries.hasQueries;
    const [result, editorMap] = await Promise.all([
      api<RenderOutput>("/api/render", basis),
      api<WritingMap>("/api/editor-map", basis),
      doc.queries.discover(basis),
    ]);
    if (!sourceConsumersAvailable(doc) || generation !== doc.renderGeneration || sessionId !== doc.session.opened.sessionId) return;
    if (result.sourceInfo.notes?.sourceHash === editorMap.sourceHash) editorMap.notes = result.sourceInfo.notes;
    if (doc.session.acceptProjection(result)) {
      // Map observers consume the accepted projection as well as its source.
      // Publish it before notifying them so initial notes controls can enable.
      doc.writing.installMap(editorMap, source, serial);
      doc.editor.tokens(result.sourceInfo.syntaxTokens, doc.mode === "styled", syntaxBasis);
      if (doc === current) { if (doc.layout === "paged" && hadQueries !== doc.queries.hasQueries) renderContent(); else renderProjection(doc); if (result.diagnostics.length) renderDiagnostics(doc); else content.querySelector(".diagnostics")?.remove(); scrollFragment(doc); }
    }
  } catch (error) { if (sourceConsumersAvailable(doc) && doc.session.serial === serial && generation === doc.renderGeneration && sessionId === doc.session.opened.sessionId) { doc.session.error = `预览失败：${error instanceof Error ? error.message : String(error)}`; if (doc === current) toast(doc.session.error, true); } }
  finally { doc.rendering--; if (doc === current) refreshChrome(); }
}

async function flushLiteral(): Promise<void> { if (current) await current.writing.flush(); }

async function chooseLibrary(): Promise<void> { const path = await pathDialog("openLibrary"); if (!path) return; await flushLiteral(); inventory = await api<Inventory>("/api/library/open", { path }); selectedFolder = null; collapsed.clear(); query = ""; renderSidebar(); renderContent(); }
async function creation<T>(route: string, payload: Record<string, unknown>): Promise<T> {
  if (pendingCreation && pendingCreation.route !== route) throw new Error("上一次创建结果仍待确认，请先重试该创建操作。");
  const request = pendingCreation ?? { route, payload: { ...payload, operationId: operationId() } };
  pendingCreation = request;
  try { const result = await api<T>(request.route, request.payload); pendingCreation = null; return result; }
  catch (error) { if (error instanceof ApiError && !["unknown", "pending", "resultUnknown"].includes(error.code)) pendingCreation = null; else toast("创建结果待确认，再次点击创建会确认同一操作。", true); throw error; }
}
async function createLibrary(): Promise<void> {
  if (pendingCreation && pendingCreation.route !== "/api/library/create") throw new Error("上一次创建结果仍待确认，请先重试原创建操作。");
  const receipt = pendingCreation
    ? await creation<LibraryCreationReceipt>("/api/library/create", pendingCreation.payload)
    : await openLibraryCreation({ defaults: () => api("/api/library/create-defaults"), create: request => api("/api/library/create", request), inspect: operationId => api("/api/recovery/inspect", { operationId }), resolve: (operationId, action) => api("/api/recovery/resolve", { operationId, action }), cancel: request => api("/api/library/create-cancel", request) });
  if (!receipt) return;
  inventory = await api<Inventory>("/api/library/open", { path: receipt.root }); query = ""; selectedFolder = null; renderSidebar(); renderContent(); toast("资料库已创建，可以新建笔记了；文档排版默认可在库样式中查看。");
  if (receipt.rootNotePath) await openDocument(receipt.rootNotePath, receipt.root);
}
async function openOutsideFile(): Promise<void> { const path = await pathDialog("file"); if (path) await openDocument(path); }
async function createNote(): Promise<void> {
  if (!inventory) { await chooseLibrary(); return; }
  if (!inventory.workspaceId) { toast("当前打开的是普通文件夹。创建资料库后可新建资料库笔记。", true); return; }
  const name = pendingCreation?.route === "/api/note/create" ? String(pendingCreation.payload.name) : await modal<string>("新建笔记", `保存到 ${selectedFolder ?? inventory.root}`, (form, close) => {
    const label = el("label", "field-label", "笔记名称"); const input = el("input", "text-input"); input.placeholder = "未命名笔记"; input.required = true; label.append(input); form.append(label);
    form.append(el("p", "field-help", "名称用于文件夹与主文件。正文可以不设标题。")); const actions = el("div", "modal-actions"); actions.append(button("取消", () => close(null), "button-secondary")); const submit = el("button", "button-primary", "创建并打开"); submit.type = "submit"; actions.append(submit); form.append(actions); form.onsubmit = event => { event.preventDefault(); if (input.value.trim()) close(input.value.trim()); };
  });
  if (!name) return; await prepareSwitch();
  const opened = await creation<OpenedDocument>("/api/note/create", { root: inventory.root, parent: selectedFolder ?? inventory.root, name });
  addDocument(opened); inventory = await api<Inventory>("/api/library/open", { path: inventory.root }); renderSidebar();
}
async function prepareSwitch(): Promise<void> { captureNavigation(); if (current) await current.writing.settleForSessionChange(); if (current) await persistDraft(current); }
async function openDocument(path: string, root?: string): Promise<void> {
  const existing = documents.find(doc => normalize(doc.session.path) === normalize(path)); if (existing && !existing.sourceTrashed) { await activateDocument(existing); return; }
  const generation=++navigationGeneration;
  await prepareSwitch(); if(generation!==navigationGeneration)return; const opened = await api<OpenedDocument>("/api/open", { path, root });
  if(generation!==navigationGeneration)return;
  if (opened.kind === "managed" && documents.some(doc => doc.sourceTrashed && doc.session.opened.nodeId === opened.nodeId && doc.session.opened.workspaceId === opened.workspaceId)) await acceptResolvedDocument(opened);
  else addDocument(opened);
  shell.classList.remove("sidebar-open");
}
function readOnlySource(doc: LiveDocument): boolean { return doc.session.opened.readOnly === true || doc.session.opened.kind === "trash" || doc.session.opened.kind === "managedTrash"; }
function addDocument(opened: OpenedDocument): void {
  const session = new DocumentSession(opened);
  let doc!: LiveDocument;
  const editor = new SourceEditor(session.source, source => { session.edit(source); doc.queries.invalidate(); readingSelection = null; workspaceTools.documentChanged(); if (current === doc) pagedPreview?.invalidate(); scheduleDraft(doc); scheduleRender(doc); if (current === doc) refreshChrome(); }, () => { void saveDocument(doc).catch(showError); });
  editor.observe(event => {
    if (event.kind === "reveal" && !editor.revealed && doc?.mode === "focus") { activeSourceOrigin = null; doc.mode = "source"; doc.queries.modeChanged(); queueMicrotask(() => { if (current === doc) renderContent(); }); }
    if (event.kind === "composition" && !event.composing && doc?.mode === "focus" && current === doc) queueMicrotask(() => { if (current === doc) renderProjection(doc); });
  });
  const preference = localStorage.getItem("weftext-view"); const mode = modes.some(m => m.id === preference) ? preference as ViewMode : "styled";
  const writing = new WritingController({
    actionAvailable: () => current===doc,
    refreshMap: basis => api("/api/editor-map",basis),
    editor, basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }),
    // A temporary consumer pause cannot revoke pending literal settlement.
    // Only the reviewed placement conflict blocks new native visual actions.
    editable: () => !sourceConsumerNeedsComparison(doc.session.opened, doc.comparisonTarget) && doc.mode !== "read" && !readOnlySource(doc), request: request => api("/api/edit", request),
    onState: () => {
      if (!doc) return;
      const retained = doc.writing.retainInput;
      if (doc.richRetained && !retained && current === doc) queueMicrotask(() => { if (current === doc && !doc.writing.retainInput) renderProjection(doc); });
      doc.richRetained = retained;
      if (current === doc) refreshChrome();
    }, onError: showError,
    onPendingSource: source => { clearTimeout(doc.draftTimer); doc.draftTimer = setTimeout(() => { void persistDraft(doc, source).catch(showError); }, 220); },
    onApplied: () => { scheduleDraft(doc); scheduleRender(doc, 100); },
  });
  const queries = new EmbeddedQueryController({
    preferenceIdentity: () => session.opened.nodeId && session.opened.workspaceId ? `weftext-node://${session.opened.workspaceId}/${session.opened.nodeId}` : null,
    catalog: async () => (await api<{ schema: import("./data-panel").DataSchema }>("/api/data/schema", { root: session.opened.root })).schema,
    openDefinition: async owner => { await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: session.opened.root, entityKey: owner, lifecycle: "live" })); },
    basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }), mode: () => doc.mode,
    editable: () => sourceConsumersAvailable(doc) && current === doc && doc.mode !== "read" && !readOnlySource(doc) && !doc.sourceTrashed && !writing.state.composing,
    request: (path, basis) => {
      if (!sourceConsumersAvailable(doc)) return Promise.reject(new Error("笔记位置正在核对或已变化，请明确比较后恢复正文查询。源码输入保留。"));
      return api(path, basis);
    }, prepareEdit: () => writing.flush(), notify: toast, output: query => workspaceTools.openQueryOutput(query),
    apply: async (patches, basis, event) => {
      if (current !== doc || doc.mode === "read" || readOnlySource(doc) || writing.state.composing || session.opened.sessionId !== basis.sessionId || session.serial !== basis.serial || session.source !== basis.source) throw new Error("查询提案的草稿或编辑资格已变化，未应用修改。");
      editor.applyPatches(basis.source, patches, undefined, event); scheduleDraft(doc); scheduleRender(doc, 0);
    },
    reveal: (occurrence, host) => {
      if (current !== doc || doc.mode !== "focus" || readOnlySource(doc)) return false;
      activeSourceOrigin = null; const range = occurrence.range;
      const revealed = editor.revealed;
      if (!(revealed?.from === range.utf16From && revealed.to === range.utf16To && revealed.raw === range.raw) && !editor.reveal({ from: range.utf16From, to: range.utf16To, raw: range.raw }, session.source)) return false;
      editor.readOnly(false); editor.mount(host); return true;
    },
    closeReveal: () => editor.clearReveal(),
    openOwner: async origin => { if (origin.isMain) return; await openDocument(origin.path, session.opened.root); },
    openRow: async (row, plan) => {
      const lifecycle = row.lifecycle ?? (plan?.advanced?.call || plan?.advanced?.related || plan?.advanced?.unionAll ? null : plan?.scope.lifecycle);
      if (!lifecycle || !row.path) throw new Error("此行缺少准确的导航来源，未按当前路径或同名目标替换。");
      if (row.entityKey) await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: session.opened.root, entityKey: row.entityKey, lifecycle, ...(lifecycle === "trash" ? { revision: row.revision } : {}) }));
      else if (lifecycle === "live") await openDocument(row.path, session.opened.root);
      else throw new Error("回收站结果缺少稳定身份，不能按路径打开。");
    },
  });
  const authoringAssets = new AuthoringAssetsAdapter({ request: api, prefix: "/api/editor/endnotes-module", context: () => doc && current === doc && sourceConsumersAvailable(doc) && !readOnlySource(doc) && !doc.sourceTrashed && doc.mode !== "read" ? { sessionId: doc.session.opened.sessionId, audience: localPersonalAudience() } : null, changed: async () => { doc.session.projection = null; doc.renderGeneration++; doc.queries.invalidate(); await compile(doc); } });
  doc = { navigationKey: opened.sessionId, session, editor, writing, queries, mode, authoringAssets, draftChain: Promise.resolve(), draftFailure: null, rendering: 0, renderGeneration: 0, scroll: {}, layout: "continuous", richRetained: false }; documents.push(doc); current = doc; activeSourceOrigin = null; renderSidebar(); renderContent(); scheduleRender(doc, 0);
  if (session.recovered) toast("已恢复上次未保存的草稿。磁盘文件保持原样。");
}
async function activateDocument(doc: LiveDocument): Promise<void> {
  const generation=++navigationGeneration;
  const activeLibrary = inventory;
  if (doc.sourceTrashed && activeLibrary && activeLibrary.workspaceId === doc.session.opened.workspaceId && activeLibrary.entries.some(entry => entry.identityStatus === "admitted" && entry.nodeId === doc.session.opened.nodeId)) {
    await prepareSwitch();
    await acceptResolvedDocument(await api<OpenedDocument>("/api/data/open", { root: activeLibrary.root, entityKey: `weftext-node://${doc.session.opened.workspaceId}/${doc.session.opened.nodeId}` }));
    return;
  }
  if (current === doc) return; await prepareSwitch(); if(generation!==navigationGeneration)return;
  const prior=doc.session.opened;
  const fresh=prior.nodeId&&prior.workspaceId&&!readOnlySource(doc)?await api<OpenedDocument>("/api/data/open",{root:prior.root,entityKey:`weftext-node://${prior.workspaceId}/${prior.nodeId}`}):await api<OpenedDocument>("/api/open",{path:doc.session.path,root:prior.root});
  if(generation!==navigationGeneration)return;
  const saved=navigationContexts.get(doc);
  if(saved?.inventory){
    const before=saved.inventory, latest=await api<Inventory>("/api/library/open",{path:before.root});if(generation!==navigationGeneration)return;
    const relocate=(path:string):string|null=>{const entry=before.entries.find(item=>normalize(item.path)===normalize(path));const primary=entry?.nodeId?entry:before.entries.find(item=>item.kind==="node"&&normalize(item.parentPath)===normalize(path));if(!primary?.nodeId)return null;const next=latest.entries.find(item=>item.kind==="node"&&item.nodeId===primary.nodeId&&item.identityStatus==="admitted");return next?(entry?.kind==="directory"?next.parentPath:next.path):null;};
    saved.selection=saved.selection.flatMap(path=>{const next=relocate(path);return next?[next]:[];});
    if(saved.folder){const next=relocate(saved.folder);if(next)saved.folder=next;else if(!latest.entries.some(item=>item.kind==="directory"&&normalize(item.path)===normalize(saved.folder!))){saved.folder=null;toast("原目录范围当前不可用，已返回当前资料库；未按同名目标恢复选择。");}}
    saved.inventory=latest;
  }
  if(fresh.source===doc.session.baseSource){doc.session.opened=fresh;doc.session.acknowledgeSave(fresh.source,fresh.baseId,fresh.path);doc.sourceTrashed=false;doc.editor.clearTokens();doc.queries.invalidate();doc.session.projection=null;await persistDraft(doc);}
  else{doc.externalSource=fresh.source;doc.comparisonTarget=fresh;doc.session.error="返回目标的磁盘内容已变化。原草稿、选区和撤销记录保留，请比较后继续。";}
  if(generation!==navigationGeneration)return;if (flowScroll && flowHasProjection && current) current.scroll[current.mode] = flowScroll.scrollTop; current = doc; restoreNavigation(doc); if(doc.mode==="write")doc.writing.restoreNavigationFocus(); activeSourceOrigin = null; renderSidebar(); renderContent();
  if(saved){const tree=sidebar.querySelector(".file-tree");if(tree)tree.scrollTop=saved.scroll;const contents=sidebar.querySelector(".node-contents");if(contents)contents.scrollTop=saved.contentScroll;}
  if (["source", "styled"].includes(doc.mode)) doc.editor.view.focus();
  if (!doc.session.projectionCurrent) scheduleRender(doc, 0);
}

async function saveDocument(doc: LiveDocument, path?: string): Promise<void> {
  if (doc.session.savePending) return;
  if ((doc.sourceTrashed || readOnlySource(doc)) && !path) { toast("笔记已移入回收站，请先恢复或将草稿另存为。", true); return; }
  if (doc.writing.state.composing) { await doc.writing.deferUntilCompositionEnds(() => saveDocument(doc, path)); return; }
  doc.session.savePending = true; refreshChrome();
  try { await flushLiteral(); await persistDraft(doc); }
  catch (error) { doc.session.savePending = false; refreshChrome(); throw error; }
  if (doc.session.opened.recoveredDraftConflict && !path) { doc.session.savePending = false; refreshChrome(); await showDraftComparison(doc); return; }
  let reservedOperation: string | null = null;
  let querySaveNotice=false;
  if (!doc.lastSave && !path && doc.session.opened.nodeId && doc.session.source.includes('"weftext.query"')) {
    const source = doc.session.source, serial = doc.session.serial;
    try { const pending=await api<{operationId:string|null;queryState?:string}>("/api/query/definition/pending", {sessionId:doc.session.opened.sessionId,source,serial});reservedOperation=pending.operationId;querySaveNotice=["unparsed","unadmitted","multiplePending"].includes(pending.queryState??""); }
    catch(error) { doc.session.savePending = false; refreshChrome(); throw error; }
    if (doc.session.source !== source || doc.session.serial !== serial) { doc.session.savePending = false; throw new Error("保存前查询预约核对期间出现新输入，请重新保存；输入已保留。"); }
  }
  const request = doc.lastSave ?? { source: doc.session.source, baseId: doc.session.baseId, operationId: reservedOperation ?? operationId(), path };
  doc.lastSave = request; doc.session.savePending = true; refreshChrome();
  let receiptReceived = false;
  try {
    const receipt = await api<Receipt>(request.path ? "/api/save-as" : "/api/save", request.path
      ? { sessionId: doc.session.opened.sessionId, path: request.path, source: request.source, operationId: request.operationId }
      : { sessionId: doc.session.opened.sessionId, baseId: request.baseId, source: request.source, operationId: request.operationId });
    if (!["saved", "committed", "complete", "after", "success"].includes(receipt.status)) throw new ApiError(receipt.status, receipt.message || "保存尚未确认，草稿已保留。");
    receiptReceived = true;
    if (request.path) {
      // Save As creates a new ordinary file. Its path, dependency root, and
      // subsequent writes belong to a newly opened Core session.
      await flushLiteral();
      const opened = await api<OpenedDocument>("/api/open", { path: receipt.path });
      if (!canRebindSaveAs(request.source, opened)) {
        doc.lastSave = undefined;
        doc.saveAsChangedTarget = receipt.path;
        throw new ApiError("saveAsTargetChanged", "另存为目标已创建，但重新打开时发现内容或恢复状态已变化。当前草稿保留在原文件会话中；请明确打开目标并比较。");
      }
      await doc.writing.settleForSessionChange();
      doc.session.opened = opened;
      doc.editor.clearTokens();
      doc.session.acknowledgeSave(request.source, opened.baseId, opened.path);
      doc.session.projection = null;
      scheduleRender(doc, 0);
    } else doc.session.acknowledgeSave(request.source, receipt.baseId, receipt.path);
    doc.saveAsChangedTarget = undefined;
    doc.lastSave = undefined;
    await persistDraft(doc); if (doc === current) { renderSidebar(); refreshChrome(); } toast(doc.session.dirty ? "提交内容已保存，后续输入仍在草稿中。" : "已保存。", false);
    if(querySaveNotice)toast("正文已按原文保存；部分查询定义尚不可运行，请核对其配置或预约。",true);
    if (inventory) { inventory = await api<Inventory>("/api/library/open", { path: inventory.root }); renderSidebar(); }
  } catch (error) {
    if (!receiptReceived && error instanceof ApiError && !["unknown", "resultUnknown", "pending"].includes(error.code)) doc.lastSave = undefined;
    doc.session.error = error instanceof Error ? error.message : String(error);
    if (error instanceof ApiError && error.currentSource != null) doc.externalSource = error.currentSource;
    if (error instanceof ApiError && error.code === "saveAsTargetChanged") doc.session.error = error.message;
    else if (receiptReceived) doc.session.error = doc.lastSave ? "文件已创建，重新打开尚未完成。请再次保存以确认同一操作。" : `文件已保存；后续恢复信息更新失败：${error instanceof Error ? error.message : String(error)}`;
    else if (!(error instanceof ApiError)) doc.session.error = "保存结果未知。草稿已保留；再次保存将确认同一操作。";
    if (doc === current) renderContent(); throw error;
  } finally { doc.session.savePending = false; if (doc === current) refreshChrome(); }
}
async function saveAs(): Promise<void> { if (!current) return; const path = await pathDialog("saveAs"); if (path) await saveDocument(current, path); }

async function showDraftComparison(doc = current): Promise<void> {
  if (!doc) return;
  await doc.writing.settleForSessionChange(); await persistDraft(doc);
  scheduleRender(doc, 0);
  const target = doc.comparisonTarget ?? doc.session.opened;
  const fresh = target.nodeId && target.workspaceId && target.kind === "managed"
    ? await api<OpenedDocument>("/api/data/open", { root: target.root, entityKey: `weftext-node://${target.workspaceId}/${target.nodeId}` })
    : await api<OpenedDocument>("/api/open", { path: target.path, root: target.root });
  await doc.writing.settleForSessionChange();
  const initialMergeSource = doc.session.source;
  const confirmed = { sessionId: doc.session.opened.sessionId, source: doc.session.source, serial: doc.session.serial, generation: doc.editor.generation };
  const result = await modal<{ action: "discardRecovered" | "mergeRecovered"; source: string }>("比较草稿与磁盘内容", "检查原始版本与磁盘现状，在合并框中保留需要的内容。应用合并只更新草稿，之后仍需保存。", (form, close) => {
    const columns = el("div", "comparison");
    for (const [label, source] of [["草稿原始版本", doc.session.opened.recoveredDraftBaseSource ?? doc.session.baseSource], ["当前磁盘内容", fresh.source]]) { const pane = el("label", "field-label", label); const text = el("textarea", "text-input comparison-source"); text.readOnly = true; text.value = source; text.rows = 10; pane.append(text); columns.append(pane); }
    form.append(columns);
    const variants = [...(doc.session.opened.recoveredDraftVariants ?? []), ...(fresh.recoveredDraftVariants ?? [])];
    for (const candidate of [doc.session.opened, fresh]) {
      if (candidate.recoveredDraft != null && candidate.recoveredDraft !== initialMergeSource) variants.push({ path: candidate.path, baseSource: candidate.recoveredDraftBaseSource ?? candidate.source, source: candidate.recoveredDraft, baseKnown: candidate.recoveredDraftBaseSource != null });
    }
    const seen = new Set<string>();
    for (const variant of variants) {
      const key = JSON.stringify(variant); if (seen.has(key)) continue; seen.add(key);
      const details = el("details", "recovery-member"); details.open = true;
      details.append(el("summary", "", `另一个保留草稿：${variant.path}`));
      const draft = el("textarea", "text-input comparison-source"); draft.readOnly = true; draft.rows = 8; draft.value = variant.source; draft.setAttribute("aria-label", `保留草稿：${variant.path}`); details.append(draft);
      if (variant.baseKnown) { const base = el("textarea", "text-input comparison-source"); base.readOnly = true; base.rows = 5; base.value = variant.baseSource; base.setAttribute("aria-label", `草稿原始版本：${variant.path}`); details.append(el("p", "field-help", "该草稿的原始版本"), base); }
      else details.append(el("p", "field-help", "该草稿的原始版本无法确认，请自行比较；不会自动覆盖当前内容。"));
      form.append(details);
    }
    const mergeLabel = el("label", "field-label", "合并后的草稿（可编辑）"); const merged = el("textarea", "text-input comparison-source"); merged.value = initialMergeSource; merged.rows = 10; mergeLabel.append(merged); form.append(mergeLabel);
    const actions = el("div", "modal-actions"); actions.append(button("关闭并保留草稿", () => close(null), "button-secondary"), button("采用磁盘版本", () => close({ action: "discardRecovered", source: fresh.source }), "button-secondary"), button("应用合并后的草稿", () => close({ action: "mergeRecovered", source: textareaSourceDraft(initialMergeSource, merged.value) }), "button-primary")); form.append(actions);
  });
  if (!result) { if (doc === current) renderContent(); return; }
  await api("/api/draft/resolve", { sessionId: fresh.sessionId, action: result.action, source: result.action === "mergeRecovered" ? result.source : undefined });
  await doc.writing.settleForSessionChange();
  if (doc.session.opened.sessionId !== confirmed.sessionId || doc.session.source !== confirmed.source || doc.session.serial !== confirmed.serial || doc.editor.generation !== confirmed.generation) {
    doc.session.opened.recoveredDraftVariants = [...(doc.session.opened.recoveredDraftVariants ?? []), { path: fresh.path, baseSource: fresh.source, source: result.source, baseKnown: true }];
    doc.comparisonTarget = fresh;
    await persistDraft(doc);
    doc.session.error = "确认合并期间又有新输入。新输入和撤销记录保持原样，已确认的合并稿也保留，请再次比较。";
    if (doc === current) renderContent();
    return;
  }
  fresh.recoveredDraftConflict = false; fresh.recoveredDraft = null; doc.session.opened = fresh; doc.editor.clearTokens(); doc.sourceTrashed = fresh.kind === "managedTrash";
  doc.session.acknowledgeSave(fresh.source, fresh.baseId, fresh.path); doc.session.projection = null; doc.externalSource = undefined; doc.comparisonTarget = undefined; doc.lastSave = undefined;
  doc.editor.apply(0, doc.editor.source.length, result.source); await persistDraft(doc); workspaceTools.documentChanged(); scheduleRender(doc, 0); renderContent(); toast(result.action === "discardRecovered" ? "已采用磁盘版本。原恢复草稿保留在恢复历史中。" : "已应用明确合并的草稿，保存后才会写入磁盘。");
}
async function showRecovery(): Promise<void> {
  const result = await api<RecoveryInfo[] | { recoveries: RecoveryInfo[] }>("/api/recovery"); recoveries = Array.isArray(result) ? result : result.recoveries;
  await modal<void>("恢复记录", "未保存草稿会在下次打开同一文件时恢复。这里显示需要检查的文件操作。", (form, close) => {
    const notes = el("section"); noteInputRecovery.render(notes, localPersonalAudience()); form.append(notes);
    if (!recoveries.length) form.append(el("p", "recovery-empty", "目前没有需要处理的文件操作。"));
    for (const record of recoveries) {
      const item = el("div", "recovery-item"); item.append(el("strong", "", record.path), el("p", "", record.message), el("span", "", `状态：${record.status}`));
      const inspect = button("比较并处理", async () => { await inspectRecovery(record.operationId); inspect.textContent = "再次查看"; }, "button-secondary compact"); item.append(inspect); form.append(item);
    }
    form.append(button("关闭", () => close(null), "button-primary"));
  });
}
async function inspectRecovery(id: string): Promise<void> {
  const record = await api<RecoveryInfo>("/api/recovery/inspect", { operationId: id });
  const decision = await modal<string>("检查文件恢复记录", "逐个核对参与文件。只有全部状态能够确认时，才提供保留、接纳或继续原操作；其他修改不会被覆盖。原始内容仍保留在恢复历史中。", (form, close) => {
    form.append(el("p", "field-help", `${record.path}\n${record.message}`));
    const states: Record<string, string> = { knownBefore: "完整的操作前状态", knownAfter: "完整的计划后状态", knownIntermediate: "已确认的中间位置，内容保留", mixedKnown: "部分文件已完成，所有参与文件仍可确认", thirdState: "存在其他修改", unavailable: "暂时无法读取" };
    if (record.coherence) form.append(el("p", "field-help", states[record.coherence] ?? record.coherence));
    if (record.members?.length) {
      for (const member of record.members) {
        const details = el("details", "recovery-member"); details.open = true;
        details.append(el("summary", "", `${member.path}${member.targetPath ? ` → ${member.targetPath}` : ""} · ${states[member.state] ?? member.state}`));
        const columns = el("div", "comparison recovery-comparison");
        for (const [label, image] of [["操作前内容", member.before], ["计划内容", member.after], ["当前内容", member.current]] as const) {
          const pane = el("div", "field-label"); pane.append(el("strong", "", label));
          if (!image?.present) pane.append(el("p", "field-help", "文件不存在或当前不可读取"));
          else if (image.source !== undefined) { const field = el("textarea", "text-input comparison-source"); field.readOnly = true; field.rows = 8; field.value = image.source; field.setAttribute("aria-label", `${label}：${member.path}`); pane.append(field); }
          else { pane.append(el("p", "field-help", `二进制文件 · ${image.byteLength.toLocaleString()} 字节`)); if (image.base64) pane.append(button("下载此版本供比较", () => {
            const bytes = Uint8Array.from(atob(image.base64!), value => value.charCodeAt(0)); const url = URL.createObjectURL(new Blob([bytes]));
            const link = el("a"); link.href = url; link.download = `${label}-${basename(member.path)}`; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
          }, "button-secondary compact")); }
          columns.append(pane);
        }
        details.append(columns); form.append(details);
      }
    } else {
      const columns = el("div", "comparison recovery-comparison");
      for (const [label, source] of [["操作前内容", record.beforeSource], ["操作提议内容", record.afterSource], ["当前磁盘内容", record.currentSource]]) { const pane = el("label", "field-label", label!); const text = el("textarea", "text-input comparison-source"); text.readOnly = true; text.rows = 10; text.value = source ?? "（当前没有可读取的文件内容）"; pane.append(text); columns.append(pane); }
      form.append(columns);
    }
    const actions = el("div", "modal-actions"); actions.append(button("关闭", () => close(null), "button-secondary"));
    const allowed = record.allowedActions ?? (!record.members?.length && !["resolved", "committed"].includes(record.status) ? ["keepCurrent"] : []);
    const labels: Record<string, string> = { keepCurrent: "保留完整原状态并解除阻塞", adoptAfterImages: "接纳已完成的全部计划内容", resumeOriginalPlan: "继续完成原操作的剩余文件" };
    for (const action of allowed) if (labels[action]) actions.append(button(labels[action], () => close(action), "button-primary"));
    form.append(actions);
  });
  if (decision) { await api("/api/recovery/resolve", { operationId: id, action: decision }); toast("恢复处理已确认。请重新打开文件；原操作和各版本仍保留在恢复历史中。"); await refreshInventory(); }
}

async function insertLink(): Promise<void> {
  if (!current || readOnlySource(current) || current.mode === "read") return; const doc = current;
  const target = await modal<{ target: string; label: string }>("插入链接", "选择已打开资料库的目标，或输入链接地址。链接将作为原生 Typst 插入，支持撤销。", (form, close) => {
    const targetLabel = el("label", "field-label", "目标地址"); const input = el("input", "text-input"); input.required = true; input.placeholder = "https://example.com 或相对文件路径"; targetLabel.append(input); form.append(targetLabel);
    if (inventory) { const options = el("select", "text-input link-targets"); const empty = el("option", "", "从资料库选择…"); empty.value = ""; options.append(empty); for (const entry of inventory.entries.filter(e => e.path.endsWith(".typ"))) { const option = el("option", "", entry.title || entry.relativePath); option.value = entry.path; options.append(option); } options.onchange = () => { if (options.value) input.value = relativeFileTarget(doc.session.path, options.value); }; form.append(options); }
    const textLabel = el("label", "field-label", "显示文字"); const label = el("input", "text-input"); label.placeholder = "链接文字"; const selection = doc.editor.view.state.selection.main; label.value = doc.editor.view.state.sliceDoc(selection.from, selection.to); textLabel.append(label); form.append(textLabel); const actions = el("div", "modal-actions"); actions.append(button("取消", () => close(null), "button-secondary")); const submit = el("button", "button-primary", "插入"); submit.type = "submit"; actions.append(submit); form.append(actions); form.onsubmit = event => { event.preventDefault(); if (input.value.trim()) close({ target: absoluteFileTarget(input.value.trim()), label: label.value || input.value.trim() }); };
  });
  if (!target) return;
  const quoted = quoteTypstString(target.target); const selection = doc.editor.view.state.selection.main; doc.editor.apply(selection.from, selection.to, `#link(${quoted})[${encodeLiteral(target.label)}]`); doc.editor.view.focus();
}
async function navigateLink(target: string): Promise<void> {
  if (!current) return;
  if (target.startsWith("#")) { const anchor = renderHost?.querySelector(`#${CSS.escape(`native-${target.slice(1)}`)}`); if (anchor) { anchor.scrollIntoView({ block: "center", behavior: "smooth" }); return; } throw new Error("当前预览中没有找到这个锚点。"); }
  const result = await api<{ path?: string; root?: string; fragment?: string; externalUrl?: string; entityKey?: string; resourceRef?: string; ownerEntityKey?: string; revision?: string; resourceFragment?: { status: string; pageIndex?: number } }>("/api/link/resolve", { sessionId: current.session.opened.sessionId, target });
  if (result.externalUrl) {
    await modal<void>("打开外部链接", result.externalUrl, (form, close) => { const link = el("a", "button-primary", "在浏览器打开"); link.href = result.externalUrl!; link.target = "_blank"; link.rel = "noopener noreferrer"; form.append(link, button("取消", () => close(null), "button-secondary")); });
  } else if (result.resourceRef) {
    if (!result.root || !result.ownerEntityKey || !result.revision) throw new Error("资源链接缺少准确身份或版本，请重新读取。");
    const owner = await api<OpenedDocument>("/api/data/open", { root: result.root, entityKey: result.ownerEntityKey });
    await acceptResolvedDocument(owner);
    workspaceTools.openResource(owner, { resource: result.resourceRef, revision: result.revision, pageIndex: result.resourceFragment?.status === "found" ? result.resourceFragment.pageIndex : undefined });
    if (result.fragment && result.resourceFragment?.status !== "found") toast("资源已打开；当前无法确认链接指定的位置，未猜测跳转。", true);
  } else if (result.entityKey) {
    const opened = await api<OpenedDocument>("/api/data/open", { root: result.root, entityKey: result.entityKey });
    await acceptResolvedDocument(opened);
    if (current && result.fragment) { current.pendingFragment = result.fragment; scrollFragment(current); }
  } else if (result.path) { await openDocument(result.path, result.root); if (current && result.fragment) { current.pendingFragment = result.fragment; scrollFragment(current); } }
}
function scrollFragment(doc: LiveDocument): void {
  if (doc !== current || !doc.pendingFragment || !doc.session.projectionCurrent || !renderHost) return;
  const fragment = doc.pendingFragment;
  let decoded = fragment; try { decoded = decodeURIComponent(fragment); } catch { /* Exact invalid escape remains identifiable. */ }
  const target = renderHost.querySelector(`#${CSS.escape(`native-${fragment}`)}`) ?? renderHost.querySelector(`#${CSS.escape(`native-${decoded}`)}`);
  if (target) { target.scrollIntoView({ block: "center" }); doc.pendingFragment = undefined; }
}

async function bootstrap(): Promise<void> {
  try {
    const result = await api<{ library?: Inventory | null; inventory?: Inventory | null; initialDocument?: OpenedDocument | null; initialDocumentError?: { message: string } | null; recoveries?: RecoveryInfo[]; recentLibraries?: string[] }>("/api/bootstrap");
    inventory = result.inventory ?? result.library ?? inventory; recoveries = result.recoveries ?? []; startupError = result.initialDocumentError?.message ?? null;
    if (result.initialDocument && !documents.length) { addDocument(result.initialDocument); try { inventory = await api<Inventory>("/api/library/open", { path: result.initialDocument.root }); } catch { /* The file session remains usable without an inventory. */ } }
  } catch (error) { startupError = error instanceof Error ? error.message : "无法连接本机服务"; }
  renderSidebar(); renderContent();
  if (!startupError) void ensureOfflineFonts().catch(error => toast(`离线字体未能完整加载：${error instanceof Error ? error.message : String(error)}。源码已保留，当前显示可能使用替代字体。`, true));
}
window.addEventListener("beforeunload", event => { if (documents.some(doc => doc.session.dirty || doc.lastSave || doc.draftFailure || doc.writing.state.pending)) { event.preventDefault(); event.returnValue = ""; } });
content.addEventListener("contextmenu",event=>{if(!(event.target instanceof Element)||!event.target.closest(".flow-document")||event.target.closest("input,textarea,a,button,[contenteditable=true]"))return;event.preventDefault();openEverydayMenu("当前正文操作",everydayWritingCommands(current?.writing,!!current&&sourceConsumersAvailable(current)&&current.mode!=="read"&&!readOnlySource(current)&&!current.editor.composing,insertEverydayQuery),toast,false);});
document.addEventListener("keydown", event => {
  if (event.isComposing || event.keyCode === 229 || current?.editor.composing || current?.writing.state.composing) return;
  if (event.target instanceof Element && event.target.closest("dialog")) return;
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "p") { event.preventDefault(); openEverydayMenu(event.shiftKey ? "命令" : "快捷打开", event.shiftKey ? everydayCommands() : everydayEntries().map(entry => ({ id:entry.key,label:entry.title,detail:entry.detail,unavailable:entry.unavailable,run:entry.open })), toast); return; }
  if (event.altKey && ["ArrowLeft","ArrowRight"].includes(event.key)) { event.preventDefault(); void stepNavigation(event.key === "ArrowLeft" ? -1 : 1).catch(showError); return; }
  if (event.target instanceof Element && event.target.closest('.embedded-query-dialog, .embedded-query-builder, .embedded-query-parameters, .document-notes-dialog, dialog[aria-label="当前笔记编号控制"]')) return;
  if (event.defaultPrevented) return;
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); if (current) void saveDocument(current).catch(showError); }
  if ((event.ctrlKey || event.metaKey) && ["z", "y"].includes(event.key.toLowerCase()) && current?.mode === "write" && !event.isComposing && !(event.target instanceof Element && event.target.closest("input,textarea,[contenteditable=true]"))) { event.preventDefault(); const doc = current; const redo = event.shiftKey || event.key.toLowerCase() === "y"; void flushLiteral().then(() => redo ? doc.editor.redo() : doc.editor.undo()).catch(showError); }
});
renderSidebar(); renderContent(); void bootstrap();
