import "./hosted.css";
import { HostedFields } from "./hosted-fields";
import { HostedReview } from "./hosted-review";
import { HostedOffice } from "./hosted-office";
import { HostedEditor } from "./hosted-editor";
import { HostedQuery } from "./hosted-query";
import { HostedHistory } from "./hosted-history";
import { HostedLibrary } from "./hosted-library";
import { currentHostedHistoryResult, hostedHistoryStatus, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, type HistoryContext } from "./history-state";
import type { HostedQueryRow } from "./hosted-query-state";
import { draftDisposition, applySaveResult, freezeHostedSave, mergeDefinitionReservations, hostedCachePrefix, sameHostedQueryAudience, sameHostedQueryIdentity, type HostedDraft } from "./hosted-state";

type Rights = { read: boolean; write: boolean; history: boolean; download: boolean };
type Doc = { resource: string; title: string; sessionId: string; baseId: string; source: string; recoveredDraft?: string; recoveredDraftBaseSource?: string; recoveredDraftConflict: boolean; rights: Rights };
type Me = { actor: string; deployment: string; generation: number; version: string };
type SaveStatus = { status: string; operationId: string; baseId?: string };
const root = document.querySelector<HTMLElement>("#hosted")!;
root.innerHTML = `<header><span class="brand">Weftext</span><span>远端写作 · 0.6 预览版</span><button id="logout" hidden>退出并清除本页缓存</button></header>
<form id="login"><h1>连接写作空间</h1><p>输入管理员提供的个人访问凭据。</p><label>访问凭据<input id="token" type="password" autocomplete="off" required minlength="32"></label><button>连接</button></form>
<p id="status" role="status" aria-live="polite">尚未连接</p>
<section id="workspace" hidden><aside><h2>可访问文档</h2><form id="search"><input id="term" aria-label="搜索获权文档" placeholder="按标题或源码搜索"><button>搜索</button></form><button id="refresh">全部文档</button><nav id="documents" aria-label="文档"></nav></aside>
<article><h1 id="title">选择文档</h1><p class="hint">五档共用同一份 Typst 草稿与撤销历史；断线输入保留，重新连接后核对原保存结果。</p>
<div class="actions"><button id="save" disabled>保存</button><button id="preview" disabled>预览</button><button id="download" disabled>下载已保存源码</button><button id="history" disabled>保存历史</button><button id="reopen" disabled>重新读取与核对</button></div>
<p id="stable-link" hidden><label>当前获权稳定笔记链接<input id="node-reference" aria-label="当前获权稳定笔记链接" readonly></label><button id="copy-reference" disabled>复制稳定笔记链接</button></p>
<section id="source" aria-label="托管文档编辑器"></section>
<section id="comparison" hidden><h2>核对并保留双方稿</h2><p>上方是你的草稿，下方是当前服务端稿。编辑上方内容，确认合并依据后再保存；确认本身不会改正文。</p><pre id="current"></pre><button id="rebase">已核对，以当前稿作为保存依据</button></section>
<section id="history-list" aria-label="托管笔记时间线" hidden></section>
<section id="library" aria-label="托管库配置与样式" hidden></section>
<section id="fields" aria-label="获权字段与查询" hidden></section>
<section id="query" aria-label="托管高级查询" hidden></section>
<section id="review" aria-label="托管批注与校对" hidden></section>
<section id="office" aria-label="Office 导入与输出" hidden></section>
<iframe id="render" title="获权内容预览" sandbox="" hidden></iframe></article></section>`;
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const fields = new HostedFields(el("fields"), request, (id, metadata) => office.setQuery(id, metadata));
const source: HostedEditor = new HostedEditor(el("source"), request, sourceChanged, () => { void run(save); }, error => status(error.message), {
  canOpenRow: row => (row as HostedQueryRow).canOpen === true && !!(row as HostedQueryRow).resource,
  openRow: row => { const target = row as HostedQueryRow; if (target.canOpen !== true || !target.resource) return Promise.reject(new Error("此查询行没有独立正文读取资格。")); return openQueryResource(target.resource); },
  openOwner: async () => { throw new Error("此查询来源没有独立获权导航对象，未按路径猜打开。"); },
  output: (query, result) => office.setQuery(query.queryId, { columns: result.columns ?? [], rowCount: result.rows.length, coverage: result.coverage }),
  modeChanged: () => queries.authorChanged(),
  navigateReference,
  dependencies: () => queries.dependencies(),
});
const review = new HostedReview(el("review"), request, () => doc ? {
  resource: doc.resource, sessionId: doc.sessionId, path: doc.resource, root: "", source: source.value,
  serial: source.serial, baseId: doc.baseId, dirty: source.value !== doc.source || compareRequired,
  pending: !!draft?.pending, selection: source.selection,
} : null, () => source.flush(), (from, to) => source.select(from, to), (resource, changed) => {
  if (changed && doc?.resource === resource) { compareRequired = true; controls(); status("建议已提交到服务端；个人草稿保留，请重新读取与核对正文。"); }
});
let token = sessionStorage.getItem("weftext-hosted-token") ?? "";
let identity: Me | undefined;
let doc: Doc | undefined;
let draft: HostedDraft | undefined;
let cachePrefix = "";
let busy = false;
let requestEpoch = 0;
let authorizationEpoch = 0;
let compareRequired = false;
let draftTimer: ReturnType<typeof setTimeout> | undefined;
const status = (message: string) => { el("status").textContent = message; };
const office: HostedOffice = new HostedOffice(el("office"), request, () => doc ? {
  resource: doc.resource, source: source.value, dirty: source.value !== doc.source || compareRequired, pending: !!draft?.pending,
} : null, () => source.flush(), () => { void list().catch(e => status(e instanceof Error ? e.message : "刷新文档列表失败，请重试")); });
const queries = new HostedQuery(el("query"), request, openQueryResource, (id, metadata) => office.setQuery(id, metadata), {
  context: () => doc ? { resource: doc.resource, sessionId: doc.sessionId, baseId: doc.baseId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending, compare: compareRequired, definitionReservations: draft?.definitionReservations } : null,
  prepareBasis: () => source.prepareQueryBasis(),
  apply: (patches, basis) => source.applyQueryPatches(patches, basis, "input.query.definition.proposal"),
  reservations: (resource, receipts, includeReserved = false) => {
    if (!doc || !draft || resource !== doc.resource) return;
    draft.definitionReservations = mergeDefinitionReservations(draft.definitionReservations ?? [], receipts, includeReserved); retain();
  },
}, () => source.queryInputsChanged());
const history = new HostedHistory(el("history-list"), request, {
  context: () => doc && draft ? { resource: doc.resource, sessionId: doc.sessionId, path: `resource:${doc.resource}`, root: "", baseId: doc.baseId, baseSource: doc.source, source: source.value, serial: source.serial, dirty: source.value !== doc.source, readOnly: !doc.rights.write || source.viewMode === "read", savePending: !!draft.pending, unresolvedSave: compareRequired } : null,
  basis: settle => source.historyBasis(settle), pendingChanged: () => { controls(); queries.authorChanged(); scheduleCurrentDraft(); }, restored: acknowledgeHistory, finishUnavailable: finishUnavailableHistory,
});
const library = new HostedLibrary(el("library"), request, async role => {
  const epoch = requestEpoch; await source.flush(); if (epoch !== requestEpoch) return;
  source.refreshEnvironment(); queries.environmentChanged(); el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  if (role === "schema" || role === "rules") await queries.refreshCatalog();
  if (epoch === requestEpoch) status("库配置原操作已提交，受影响预览重新编译；正文草稿与撤销历史保留，未保存正文。");
});
let stableReference: { resource: string; sessionId: string; reference: string } | undefined;
async function navigateReference(reference: string) {
  const epoch = requestEpoch; await source.flush(); retain();
  if (epoch !== requestEpoch) return;
  const resolved = await request<{ resource: string; canOpen: boolean; reference: string }>("navigation/resolve", { reference });
  if (epoch !== requestEpoch) return;
  if (!resolved.resource || resolved.canOpen !== true) throw new Error("稳定链接目标当前不可用，个人草稿保持保留。");
  await run(() => open(resolved.resource));
}
async function readReference(selected: Doc, epoch: number) {
  stableReference = undefined; el("stable-link").hidden = true; el<HTMLInputElement>("node-reference").value = "";
  try { const result = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: selected.sessionId }); if (epoch !== requestEpoch || doc !== selected) return; if (result.resource !== selected.resource || !/^weftext-node:\/\//.test(result.reference)) throw new Error("当前笔记没有获权稳定身份。"); stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference: result.reference }; el<HTMLInputElement>("node-reference").value = result.reference; el("stable-link").hidden = false; controls(); } catch (error) { if (epoch === requestEpoch && doc === selected) status(error instanceof Error ? error.message : "稳定身份当前不可用。"); }
}
async function acknowledgeHistory(record: HostedHistoryRecord): Promise<"applied" | "superseded"> {
  const selected = doc, epoch = requestEpoch;
  if (!selected || !draft || selected.resource !== record.request.resource || !record.status?.result) throw new Error("请打开原对象后核对恢复回执；不会按另一篇文档应用。");
  await source.flush();
  if (epoch !== requestEpoch || doc !== selected || !draft) throw new Error("对象已变化，原恢复回执仍保留待核对。");
  const fresh = await request<Doc>("open", { resource: selected.resource }); let adopted = false;
  try {
    const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
    if (epoch !== requestEpoch || doc !== selected || !draft) throw new Error("核对期间对象或用户已变化，未应用旧回执。");
    record.status = hostedHistoryStatus(record, confirmed);
    if (!confirmed.result) throw new Error("原恢复事实仍保留，当前资格不允许交付原结果；未采用旧缓存。");
    const installed = currentHostedHistoryResult(record, fresh.source, confirmed), buffer = { sessionId: selected.sessionId, source: source.value, serial: source.serial };
    const current: HistoryContext = { ...buffer, path: `resource:${selected.resource}`, root: "", baseId: selected.baseId, baseSource: selected.source, dirty: buffer.source !== selected.source, nodeId: record.basis.nodeId, workspaceId: record.basis.workspaceId };
    if (installed && restoreBufferDisposition(record.basis, current) === "replaceClean") source.applyHistorySource(confirmed.result.source, buffer);
    const retainedSource = source.value, originalDraftBase = draft.baseSource;
    doc = fresh; source.open(fresh.sessionId, fresh.resource, retainedSource); source.disabled = !fresh.rights.write;
    draft = { ...draft, source: retainedSource, baseSource: installed ? fresh.source : originalDraftBase };
    compareRequired = !installed; el("comparison").hidden = installed; el("current").textContent = installed ? "" : fresh.source;
    el("title").textContent = fresh.title; adopted = true; retain(); history.documentChanged(); controls(); queries.authorChanged(true); void queries.refreshCatalog(); void readReference(fresh, epoch);
    try { await request("close", { sessionId: selected.sessionId }); } catch (error) { if (!(error instanceof HostedError) || error.http !== 410) throw error; }
    if (installed && retainedSource !== fresh.source && fresh.rights.write) await request("draft", { sessionId: fresh.sessionId, source: retainedSource });
    return installed ? "applied" : "superseded";
  } finally {
    if (!adopted) { try { await request("close", { sessionId: fresh.sessionId }); } catch { /* Keep the original receipt even if session cleanup became unavailable. */ } }
  }
}
async function finishUnavailableHistory(record: HostedHistoryRecord): Promise<void> {
  const selected = doc, epoch = requestEpoch;
  if (!selected || !draft || selected.resource !== record.request.resource || !unavailableCommittedHistory(record)) throw new Error("请先打开原配置对象并查询原已提交事实，未结束未知操作。");
  await source.flush(); if (epoch !== requestEpoch || doc !== selected || !draft) throw new Error("当前对象已变化，原事实保留。");
  const fresh = await request<Doc>("open", { resource: selected.resource }); let adopted = false;
  try {
    const confirmed = await request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
    if (epoch !== requestEpoch || doc !== selected || !draft) throw new Error("重新读取期间对象或用户已变化，未采用旧回执。");
    record.status = hostedHistoryStatus(record, confirmed); if (!unavailableCommittedHistory(record)) throw new Error("原结果交付状态已变化，请重新查询并核对原结果。");
    const retainedSource = source.value; doc = fresh; draft = { ...draft, source: retainedSource }; source.open(fresh.sessionId, fresh.resource, retainedSource); source.disabled = !fresh.rights.write;
    compareRequired = true; el("comparison").hidden = false; el("current").textContent = fresh.source; el("title").textContent = fresh.title; adopted = true; retain(); history.documentChanged(); controls(); void readReference(fresh, epoch);
    try { await request("close", { sessionId: selected.sessionId }); } catch (error) { if (!(error instanceof HostedError) || error.http !== 410) throw error; }
  } finally { if (!adopted) { try { await request("close", { sessionId: fresh.sessionId }); } catch { /* Preserve original evidence when cleanup is unavailable. */ } } }
}
async function openQueryResource(resource: string) { const epoch = requestEpoch; await source.flush(); if (epoch !== requestEpoch) return; await run(() => open(resource)); }

class HostedError extends Error { constructor(public code: string, message: string, public http: number) { super(message); } }
function clearDisplay(message: string, clearToken = false) {
  requestEpoch++; authorizationEpoch++; doc = undefined; draft = undefined; compareRequired = false; cachePrefix = ""; stableReference = undefined;
  const clearErrors: unknown[] = [];
  // A blocked storage provider must never stop the remaining protected views
  // from disposing their in-memory capabilities and visible content.
  for (const clear of [() => source.clear(), () => fields.clear(), () => review.clear(), () => queries.clear(), () => office.clear(), () => history.clear(), () => library.clear()]) {
    try { clear(); } catch (error) { clearErrors.push(error); }
  }
  el("documents").replaceChildren();
  el("title").textContent = "选择文档"; el("current").textContent = "";
  el("stable-link").hidden = true; el<HTMLInputElement>("node-reference").value = "";
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  el("comparison").hidden = true; el("history-list").hidden = true;
  const remove = (key: string) => { try { sessionStorage.removeItem(key); } catch (error) { clearErrors.push(error); } };
  try { for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-draft:")) remove(key); } catch (error) { clearErrors.push(error); }
  if (clearToken) { token = ""; identity = undefined; remove("weftext-hosted-token"); remove("weftext-hosted-identity"); el("workspace").hidden = true; el("login").hidden = false; el("logout").hidden = true; }
  controls(); status(clearErrors.length ? `${message.split(/[，。]/)[0]}。受保护展示已清除；浏览器拒绝删除部分恢复缓存，请在浏览器中清除此站点数据。` : message);
}
async function request<T>(path: string, data?: unknown, bytes = false, attempt = 0): Promise<T> {
  const epoch = requestEpoch, credential = token;
  const audience = () => ({ epoch: authorizationEpoch, credential: token, actor: identity?.actor, deployment: identity?.deployment, generation: identity?.generation, sessionId: doc?.sessionId, queryVersion: queries.version }), captured = audience();
  const response = await fetch(`/api/hosted/${path}`, {method:data === undefined ? "GET" : "POST", headers:{"Content-Type":"application/json",Authorization:`Bearer ${token}`},body:data === undefined ? undefined : JSON.stringify(data),cache:"no-store"});
  if (!response.ok) {
    const result = await response.json() as {error?:{code:string;message:string}};
    // Only a read-only preview can wait for the shared compiler. Author save
    // requests keep their explicit original-operation recovery path.
    if (path === "preview" && response.status === 429 && attempt < 5) {
      await new Promise(resolve => setTimeout(resolve, 500));
      if (epoch === requestEpoch && credential === token) return request(path, data, bytes, attempt + 1);
    }
    if (response.status === 401 && sameHostedQueryIdentity(captured, audience())) clearDisplay("凭据失效，已清除本页缓存。", true);
    if (response.status === 404 && path === "query" && sameHostedQueryIdentity(captured, audience())) {
      try { const current = await request<Me>("me"); if (sameHostedQueryIdentity(captured, audience())) { if (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation) { clearDisplay("授权策略已变化，旧展示与缓存已清除，请刷新文档列表。"); identity = current; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current)); } else if (sameHostedQueryAudience(captured, audience())) queries.refuseResult("此查询对象或范围不可用；原计划、参数和个人正文草稿保留，未交付结果。"); } }
      catch { if (sameHostedQueryAudience(captured, audience())) queries.refuseResult("查询资格核对未完成，结果暂不交付；原计划与个人草稿保留，请重新连接核对。"); }
    } else if (response.status === 404 && path !== "query" && !path.startsWith("navigation/") && epoch === requestEpoch && credential === token) clearDisplay("对象或授权已变化，已清除旧内容。请刷新文档列表。");
    throw new HostedError(result.error?.code ?? "request", result.error?.code === "basisRequired" && path.startsWith("query/") ? "保存带范围的定义需要已安装的规则基线；请由管理员明确安装库规则后重试，原草稿与提案保留，未自动创建规则。" : result.error?.message ?? "请求失败", response.status);
  }
  return (bytes ? response.arrayBuffer() : response.json()) as Promise<T>;
}
function controls() {
  el<HTMLButtonElement>("save").disabled = busy || !doc?.rights.write || compareRequired || !!draft?.pending || !!doc && history.hasPending(doc.resource);
  for (const id of ["preview","reopen"]) el<HTMLButtonElement>(id).disabled = busy || !doc;
  el<HTMLButtonElement>("download").disabled = busy || !doc?.rights.download;
  el<HTMLButtonElement>("history").disabled = busy || !doc?.rights.history;
  el<HTMLButtonElement>("rebase").disabled = busy || !doc?.rights.write || !!draft?.pending;
  el<HTMLButtonElement>("copy-reference").disabled = busy || !stableReference || stableReference.sessionId !== doc?.sessionId;
}
function retain() {
  if (!doc || !draft) return;
  draft.source = source.value;
  // Storage failure is visible and prevents a save request that could lose an
  // unknown operation receipt on reload.
  sessionStorage.setItem(cachePrefix + doc.resource, JSON.stringify(draft));
}
function cached(resource: string): HostedDraft | undefined {
  try { const raw = sessionStorage.getItem(cachePrefix + resource); return raw ? JSON.parse(raw) as HostedDraft : undefined; } catch { return undefined; }
}
async function connect() {
  const previous = identity ?? JSON.parse(sessionStorage.getItem("weftext-hosted-identity") ?? "null") as Me | undefined;
  const next = await request<Me>("me");
  if (previous && (next.actor !== previous.actor || next.deployment !== previous.deployment || next.generation !== previous.generation)) clearDisplay("授权策略已更新，请重新打开文档。");
  identity = next; cachePrefix = hostedCachePrefix("draft", next.deployment, next.actor, next.generation);
  sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(next));
  sessionStorage.setItem("weftext-hosted-token", token);
  el("login").hidden = true; el("workspace").hidden = false; el("logout").hidden = false;
  status(`${next.actor} · ${next.version} · 已连接`); await list(); await fields.connect(next.deployment, next.actor, next.generation); await review.connect(next.deployment, next.actor, next.generation); await queries.connect(next.deployment, next.actor, next.generation); await office.connect(next.deployment, next.actor, next.generation); await history.connect(next.deployment, next.actor, next.generation); await library.connect(next.deployment, next.actor, next.generation);
}
async function list(term?: string) {
  const epoch = requestEpoch;
  const rows = term ? (await request<{rows: {resource:string;title:string}[];complete:boolean}>("search",{text:term})) : undefined;
  const items = rows?.rows ?? (await request<{documents:{resource:string;title:string}[]}>("documents")).documents;
  if (epoch !== requestEpoch) return;
  el("documents").replaceChildren(...items.map(item => { const button = document.createElement("button"); button.textContent = item.title; button.onclick = () => run(() => open(item.resource)); return button; }));
  if (rows) status(rows.complete ? `在可访问范围找到 ${items.length} 项` : `搜索未完整，仅显示已取得的 ${items.length} 项`);
}
async function open(resource: string) {
  await source.flush(); retain();
  source.disabled = true;
  status("正在重新读取；现有草稿已保留。");
  const currentIdentity = await request<Me>("me");
  if (identity && currentIdentity.generation !== identity.generation) {
    clearDisplay("授权策略已更新，旧展示与缓存已清除。请重新选择文档。"); identity = currentIdentity; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(identity)); await list(); return;
  }
  cachePrefix = hostedCachePrefix("draft", currentIdentity.deployment, currentIdentity.actor, currentIdentity.generation);
  const prior = doc;
  const local = cached(resource);
  if (prior) {
    try { await request("close",{sessionId:prior.sessionId}); }
    catch (e) { if (!(e instanceof HostedError) || e.http !== 410) throw e; }
  }
  const epoch = ++requestEpoch;
  const opened = await request<Doc>("open",{resource});
  if (epoch !== requestEpoch) return;
  doc = opened;
  draft = local ?? {source:opened.recoveredDraft ?? opened.source,baseSource:opened.recoveredDraftBaseSource ?? opened.source};
  let recoveredSave = false;
  if (draft.pending) {
    let outcome = await request<SaveStatus>("operation",{operationId:draft.pending.operationId});
    if (["notRegistered","notCommitted"].includes(outcome.status)) outcome = await request<SaveStatus>("operation/settle",{resource,operationId:draft.pending.operationId});
    if (epoch !== requestEpoch) return;
    if (outcome.status === "committed") { draft = applySaveResult(draft,draft.pending.source); recoveredSave = true; }
    else if (["cancelled","resolved"].includes(outcome.status)) { delete draft.pending; }
    else status("原保存尚需恢复，请保留草稿并联系管理员；不会重复提交。");
  }
  if (!draft.pending && draftDisposition(opened.source,draft) === "current") draft = { ...draft, source:opened.source, baseSource:opened.source };
  source.open(opened.sessionId, opened.resource, draft.source); source.disabled = !opened.rights.write;
  el("title").textContent = opened.title;
  compareRequired = opened.recoveredDraftConflict || draftDisposition(opened.source,draft) === "compare";
  el("comparison").hidden = !compareRequired; el("current").textContent = compareRequired ? opened.source : "";
  history.documentChanged();
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  retain(); controls(); queries.authorChanged(true);
  void readReference(opened, epoch);
  if (recoveredSave) void queries.refreshCatalog();
  status(draft.pending ? "原保存结果尚未确定，暂不允许再次保存。" : compareRequired ? "双方稿已保留，请核对后合并。" : draft.source !== opened.source ? "已恢复个人草稿，尚未保存正文。" : "已读取当前稿。");
}
async function save() {
  if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource)) return;
  await source.flush();
  if (!doc || !draft || draft.pending || compareRequired || history.hasPending(doc.resource)) return;
  draft.source = source.value;
  const sent = draft.source;
  const epoch = requestEpoch;
  const op = crypto.randomUUID();
  const exactRequest = freezeHostedSave(draft, doc.sessionId, doc.baseId, op);
  draft.pending = {operationId:op,source:sent,request:exactRequest}; retain(); controls(); queries.authorChanged();
  try {
    const result = await request<SaveStatus>("save",exactRequest);
    if (epoch !== requestEpoch || !doc || !draft) return;
    if (result.status !== "committed") { status("保存尚未确认，请核对原操作结果。"); return; }
    doc.baseId = result.baseId!; doc.source = sent;
    draft = applySaveResult(draft,sent); retain(); source.refreshQueries(); queries.authorChanged(true); void queries.refreshCatalog(); status("已保存。" + (draft.source !== sent ? "新输入仍在草稿中。" : ""));
  } catch (error) {
    if (epoch !== requestEpoch || !draft) throw error;
    // Network/unknown failure keeps the exact original ID until status lookup.
    if (error instanceof HostedError && error.http === 409) {
      let result = await request<SaveStatus>("operation",{operationId:op});
      if (["notRegistered","notCommitted"].includes(result.status)) result = await request<SaveStatus>("operation/settle",{resource:doc!.resource,operationId:op});
      if (result.status === "cancelled") { delete draft.pending; compareRequired = true; }
    }
    retain(); throw error;
  }
}
async function run(action: () => Promise<void>) {
  if (busy) return; busy = true; controls();
  try { await action(); } catch (e) { status(e instanceof Error ? e.message : "请求未完成；草稿仍保留。"); }
  finally { busy = false; source.disabled = !doc?.rights.write; controls(); }
}
function sourceChanged() {
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  try { retain(); status(navigator.onLine ? "个人草稿，尚未保存正文。" : "连接中断，本页草稿仍保留。"); } catch { status("浏览器无法保留恢复稿，请先复制你的输入。"); }
  queries.authorChanged();
  scheduleCurrentDraft();
}
function scheduleCurrentDraft() {
  clearTimeout(draftTimer);
  const captured = doc; const epoch = requestEpoch;
  draftTimer = setTimeout(() => {
    if (!captured || doc !== captured || epoch !== requestEpoch || !captured.rights.write || compareRequired || draft?.pending || busy || history.hasPending(captured.resource)) return;
    void request("draft",{sessionId:captured.sessionId,source:source.value}).catch(() => {});
  },800);
}
el<HTMLFormElement>("login").onsubmit = e => {e.preventDefault(); token = el<HTMLInputElement>("token").value.trim(); void run(connect);};
el("logout").onclick = () => { clearDisplay("已退出。",true); el<HTMLInputElement>("token").value = ""; };
el("refresh").onclick = () => void run(async () => {
  const epoch = requestEpoch, current = await request<Me>("me"); if (epoch !== requestEpoch) return;
  if (!identity || current.generation !== identity.generation || current.actor !== identity.actor || current.deployment !== identity.deployment) clearDisplay("授权策略已更新，旧展示与缓存已清除。");
  identity = current; cachePrefix = hostedCachePrefix("draft", current.deployment, current.actor, current.generation); sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current));
  await list(); await fields.connect(current.deployment, current.actor, current.generation); await review.connect(current.deployment, current.actor, current.generation); await queries.connect(current.deployment, current.actor, current.generation); await office.connect(current.deployment, current.actor, current.generation); await history.connect(current.deployment, current.actor, current.generation); await library.connect(current.deployment, current.actor, current.generation);
});
el<HTMLFormElement>("search").onsubmit = e => {e.preventDefault();void run(() => list(el<HTMLInputElement>("term").value.trim()));};
el("save").onclick = () => void run(save);
el("reopen").onclick = () => void run(() => open(doc!.resource));
el("rebase").onclick = () => void run(async () => {
  if (!doc || !draft) return;
  await source.flush();
  await request("draft/resolve",{sessionId:doc.sessionId,source:source.value});
  draft.baseSource = doc.source; compareRequired = false; retain(); el("comparison").hidden = true; status("合并依据已确认，原草稿已保留；点击保存才提交正文。");
});
el("preview").onclick = () => void run(async () => {
  const epoch = requestEpoch;
  await source.flush();
  const renderedSource = source.value;
  const rendered = await request<{ok:boolean;html:string;message:string}>("preview",{sessionId:doc!.sessionId,source:renderedSource});
  if (epoch !== requestEpoch) return;
  if (source.value !== renderedSource) { status("草稿已变化，请重新生成预览。"); return; }
  if (!rendered.ok) {status(rendered.message);return;}
  // A fresh browsing context avoids reusing a hidden empty srcdoc document,
  // which some Chromium builds retain with a zero-sized layout after reload.
  const frame = document.createElement("iframe");
  frame.id = "render"; frame.title = "获权内容预览"; frame.setAttribute("sandbox", "");
  frame.srcdoc = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:;"><style>body{padding:24px;line-height:1.8;font-family:serif}img,svg{max-width:100%}</style>${rendered.html}`;
  el("render").replaceWith(frame); status("预览仅使用当前获权的显式依赖。");
});
el("download").onclick = () => void run(async () => {
  const epoch = requestEpoch; const resource = doc!.resource;
  const prepared = await request<{ticket:string;length:number;sha256:string;chunkSize:number}>("download/prepare",{resource});
  const chunks: ArrayBuffer[] = [];
  for (let offset=0;offset<prepared.length;offset+=prepared.chunkSize) {
    chunks.push(await request<ArrayBuffer>("download/chunk",{ticket:prepared.ticket,offset},true));
    if (epoch !== requestEpoch) return;
  }
  const blob = new Blob(chunks,{type:"text/plain;charset=utf-8"});
  const hash = [...new Uint8Array(await crypto.subtle.digest("SHA-256",await blob.arrayBuffer()))].map(b=>b.toString(16).padStart(2,"0")).join("");
  if (hash !== prepared.sha256) throw new Error("下载内容不完整，未交付文件。");
  if (epoch !== requestEpoch) return;
  const url = URL.createObjectURL(blob); const link = document.createElement("a");link.href = url;link.download = `${resource}.typ`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);status("已下载确认时的已保存源码。");
});
el("history").onclick = () => void history.open();
el("copy-reference").onclick = () => void run(async () => { const selected = stableReference, epoch = requestEpoch; if (!selected || selected.sessionId !== doc?.sessionId) return; const latest = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: selected.sessionId }); if (epoch !== requestEpoch || stableReference !== selected || latest.resource !== selected.resource || latest.reference !== selected.reference) return; await navigator.clipboard.writeText(latest.reference); if (epoch === requestEpoch) status("已复制当前获权稳定笔记链接。"); });
window.addEventListener("offline",()=>status("连接中断，本页草稿仍保留；不会自动提交。"));
window.addEventListener("beforeunload", e=>{if(library.hasPending || history.hasPending() || draft && (draft.pending || draft.source!==draft.baseSource)){e.preventDefault();}});
setInterval(()=>{
  if(!identity || busy) return;
  const old=identity, epoch=requestEpoch, credential=token;
  void request<Me>("me").then(next=>{
    if(epoch!==requestEpoch || credential!==token || identity!==old) return;
    if(next.generation!==old.generation || next.actor!==old.actor || next.deployment!==old.deployment){clearDisplay("授权策略已变化，旧展示与缓存已清除，请刷新文档列表。");identity=next;sessionStorage.setItem("weftext-hosted-identity",JSON.stringify(next));}
  }).catch(()=>{});
},5000);
if(token) void run(connect);
