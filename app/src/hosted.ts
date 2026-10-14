import "./hosted.css";
import { HostedFields } from "./hosted-fields";
import { HostedReview } from "./hosted-review";
import { HostedOffice } from "./hosted-office";
import { HostedEditor } from "./hosted-editor";
import { draftDisposition, applySaveResult, type HostedDraft } from "./hosted-state";

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
<section id="source" aria-label="托管文档编辑器"></section>
<section id="comparison" hidden><h2>核对并保留双方稿</h2><p>上方是你的草稿，下方是当前服务端稿。编辑上方内容，确认合并依据后再保存；确认本身不会改正文。</p><pre id="current"></pre><button id="rebase">已核对，以当前稿作为保存依据</button></section>
<section id="history-list" hidden><h2>保留的保存版本</h2><div id="versions"></div><pre id="historical"></pre></section>
<section id="fields" aria-label="获权字段与查询" hidden></section>
<section id="review" aria-label="托管批注与校对" hidden></section>
<section id="office" aria-label="Office 导入与输出" hidden></section>
<iframe id="render" title="获权内容预览" sandbox="" hidden></iframe></article></section>`;
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const fields = new HostedFields(el("fields"), request, (id, metadata) => office.setQuery(id, metadata));
const source = new HostedEditor(el("source"), request, sourceChanged, () => { void run(save); }, error => status(error.message));
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
let compareRequired = false;
let draftTimer: ReturnType<typeof setTimeout> | undefined;
const status = (message: string) => { el("status").textContent = message; };
const office = new HostedOffice(el("office"), request, () => doc ? {
  resource: doc.resource, source: source.value, dirty: source.value !== doc.source || compareRequired, pending: !!draft?.pending,
} : null, () => source.flush(), () => { void list().catch(e => status(e instanceof Error ? e.message : "刷新文档列表失败，请重试")); });

class HostedError extends Error { constructor(public code: string, message: string, public http: number) { super(message); } }
function clearDisplay(message: string, clearToken = false) {
  requestEpoch++; doc = undefined; draft = undefined; compareRequired = false;
  source.clear(); fields.clear(); review.clear(); office.clear(); el("documents").replaceChildren();
  el("title").textContent = "选择文档"; el("current").textContent = "";
  el("versions").replaceChildren(); el("historical").textContent = "";
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  el("comparison").hidden = true; el("history-list").hidden = true;
  for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-draft:")) sessionStorage.removeItem(key);
  if (clearToken) { token = ""; identity = undefined; sessionStorage.removeItem("weftext-hosted-token"); el("workspace").hidden = true; el("login").hidden = false; el("logout").hidden = true; }
  controls(); status(message);
}
async function request<T>(path: string, data?: unknown, bytes = false, attempt = 0): Promise<T> {
  const epoch = requestEpoch, credential = token;
  const response = await fetch(`/api/hosted/${path}`, {method:data === undefined ? "GET" : "POST", headers:{"Content-Type":"application/json",Authorization:`Bearer ${token}`},body:data === undefined ? undefined : JSON.stringify(data),cache:"no-store"});
  if (!response.ok) {
    const result = await response.json() as {error?:{code:string;message:string}};
    // Only a read-only preview can wait for the shared compiler. Author save
    // requests keep their explicit original-operation recovery path.
    if (path === "preview" && response.status === 429 && attempt < 5) {
      await new Promise(resolve => setTimeout(resolve, 500));
      if (epoch === requestEpoch && credential === token) return request(path, data, bytes, attempt + 1);
    }
    if (epoch === requestEpoch && credential === token) {
      if (response.status === 401) clearDisplay("凭据失效，已清除本页缓存。", true);
      if (response.status === 404) clearDisplay("对象或授权已变化，已清除旧内容。请刷新文档列表。");
    }
    throw new HostedError(result.error?.code ?? "request", result.error?.message ?? "请求失败", response.status);
  }
  return (bytes ? response.arrayBuffer() : response.json()) as Promise<T>;
}
function controls() {
  el<HTMLButtonElement>("save").disabled = busy || !doc?.rights.write || compareRequired || !!draft?.pending;
  for (const id of ["preview","reopen"]) el<HTMLButtonElement>(id).disabled = busy || !doc;
  el<HTMLButtonElement>("download").disabled = busy || !doc?.rights.download;
  el<HTMLButtonElement>("history").disabled = busy || !doc?.rights.history;
  el<HTMLButtonElement>("rebase").disabled = busy || !doc?.rights.write || !!draft?.pending;
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
  identity = next; cachePrefix = `weftext-hosted-draft:${next.deployment}:${next.actor}:`;
  sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(next));
  sessionStorage.setItem("weftext-hosted-token", token);
  el("login").hidden = true; el("workspace").hidden = false; el("logout").hidden = false;
  status(`${next.actor} · ${next.version} · 已连接`); await list(); await fields.connect(next.deployment, next.actor); await review.connect(next.deployment, next.actor); await office.connect(next.deployment, next.actor);
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
  if (draft.pending) {
    let outcome = await request<SaveStatus>("operation",{operationId:draft.pending.operationId});
    if (["notRegistered","notCommitted"].includes(outcome.status)) outcome = await request<SaveStatus>("operation/settle",{resource,operationId:draft.pending.operationId});
    if (epoch !== requestEpoch) return;
    if (outcome.status === "committed") { draft = applySaveResult(draft,draft.pending.source); }
    else if (["cancelled","resolved"].includes(outcome.status)) { draft = {source:draft.source,baseSource:draft.baseSource}; }
    else status("原保存尚需恢复，请保留草稿并联系管理员；不会重复提交。");
  }
  if (!draft.pending && draftDisposition(opened.source,draft) === "current") draft = {source:opened.source,baseSource:opened.source};
  source.open(opened.sessionId, opened.resource, draft.source); source.disabled = !opened.rights.write;
  el("title").textContent = opened.title;
  compareRequired = opened.recoveredDraftConflict || draftDisposition(opened.source,draft) === "compare";
  el("comparison").hidden = !compareRequired; el("current").textContent = compareRequired ? opened.source : "";
  el("history-list").hidden = true; el("historical").textContent = ""; el("versions").replaceChildren();
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  retain(); controls();
  status(draft.pending ? "原保存结果尚未确定，暂不允许再次保存。" : compareRequired ? "双方稿已保留，请核对后合并。" : draft.source !== opened.source ? "已恢复个人草稿，尚未保存正文。" : "已读取当前稿。");
}
async function save() {
  if (!doc || !draft || draft.pending || compareRequired) return;
  await source.flush();
  if (!doc || !draft || draft.pending || compareRequired) return;
  const sent = source.value;
  const epoch = requestEpoch;
  const op = crypto.randomUUID();
  draft.pending = {operationId:op,source:sent}; retain(); controls();
  try {
    const result = await request<SaveStatus>("save",{sessionId:doc.sessionId,baseId:doc.baseId,operationId:op,source:sent});
    if (epoch !== requestEpoch || !doc || !draft) return;
    if (result.status !== "committed") { status("保存尚未确认，请核对原操作结果。"); return; }
    doc.baseId = result.baseId!; doc.source = sent;
    draft = applySaveResult(draft,sent); retain(); status("已保存。" + (draft.source !== sent ? "新输入仍在草稿中。" : ""));
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
  clearTimeout(draftTimer);
  const captured = doc; const epoch = requestEpoch;
  draftTimer = setTimeout(() => {
    if (!captured || epoch !== requestEpoch || draft?.pending || busy) return;
    void request("draft",{sessionId:captured.sessionId,source:source.value}).catch(() => {});
  },800);
}
el<HTMLFormElement>("login").onsubmit = e => {e.preventDefault(); token = el<HTMLInputElement>("token").value.trim(); void run(connect);};
el("logout").onclick = () => { clearDisplay("已退出。",true); el<HTMLInputElement>("token").value = ""; };
el("refresh").onclick = () => void run(async () => { await list(); if (identity) { await fields.connect(identity.deployment, identity.actor); await review.connect(identity.deployment, identity.actor); await office.connect(identity.deployment, identity.actor); } });
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
el("history").onclick = () => void run(async () => {
  const epoch = requestEpoch; const selected = doc!;
  const timeline = await request<{versions:{versionId:string;savedAtUnixMs?:number;status:string}[]}>("history",{sessionId:selected.sessionId});
  if (epoch !== requestEpoch) return;
  el("versions").replaceChildren(...timeline.versions.map(v=>{
    const button=document.createElement("button");button.textContent=v.savedAtUnixMs ? new Date(v.savedAtUnixMs).toLocaleString() : "时间未知";button.disabled=v.status!=="available";
    button.onclick=()=>void run(async()=>{const result=await request<{source:string}>("history/read",{sessionId:selected.sessionId,versionId:v.versionId});if(epoch===requestEpoch)el("historical").textContent=result.source;});return button;
  }));el("history-list").hidden=false;status("显示本服务实际保留的版本，历史内容只读。");
});
window.addEventListener("offline",()=>status("连接中断，本页草稿仍保留；不会自动提交。"));
window.addEventListener("beforeunload", e=>{if(draft && (draft.pending || draft.source!==draft.baseSource)){e.preventDefault();}});
setInterval(()=>{
  if(!identity || busy) return;
  const old=identity, epoch=requestEpoch, credential=token;
  void request<Me>("me").then(next=>{
    if(epoch!==requestEpoch || credential!==token || identity!==old) return;
    if(next.generation!==old.generation || next.actor!==old.actor || next.deployment!==old.deployment){clearDisplay("授权策略已变化，旧展示与缓存已清除，请刷新文档列表。");identity=next;sessionStorage.setItem("weftext-hosted-identity",JSON.stringify(next));}
  }).catch(()=>{});
},5000);
if(token) void run(connect);
