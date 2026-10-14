import "./hosted.css";
import { HostedFields } from "./hosted-fields";
import { HostedReview } from "./hosted-review";
import { HostedOffice } from "./hosted-office";
import { HostedEditor } from "./hosted-editor";
import { HostedQuery } from "./hosted-query";
import { HostedHistory } from "./hosted-history";
import { HostedLibrary } from "./hosted-library";
import { HostedMedia } from "./hosted-media";
import { HostedStructure } from "./hosted-structure";
import { HostedKnowledgeLinks } from "./hosted-knowledge-links";
import { HostedCitations } from "./hosted-citations";
import { HostedNavigationRegistry } from "./hosted-navigation-registry";
import { HostedContentImport } from "./hosted-content-import";
import { HostedOwnerBackup } from "./hosted-owner-backup";
import type { StructureReceipt, StructureRequest } from "./hosted-structure-state";
import { nodeFragmentOrigin, nodeReferenceParts, nodeSourceHash, nodeTargetDraft, sameHostedNodeAudience, type HostedNodeAudience, type HostedNodeBasis, type HostedNodeFragmentResult, type HostedNodeResolution } from "./hosted-navigation-state";
import { currentHostedHistoryResult, hostedHistoryStatus, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, type HistoryContext } from "./history-state";
import type { HostedQueryRow } from "./hosted-query-state";
import { bindHostedDraftReference, draftDisposition, applySaveResult, freezeHostedSave, hostedOpenedDraft, mergeDefinitionReservations, hostedCachePrefix, sameHostedQueryAudience, sameHostedQueryIdentity, type HostedDraft } from "./hosted-state";

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
<section id="node-navigation" aria-label="稳定笔记片段导航" hidden></section>
<section id="source" aria-label="托管文档编辑器"></section>
<section id="comparison" hidden><h2>核对并保留双方稿</h2><p>上方是你的草稿，下方是当前服务端稿。编辑上方内容，确认合并依据后再保存；确认本身不会改正文。</p><pre id="current"></pre><button id="rebase">已核对，以当前稿作为保存依据</button></section>
<section id="history-list" aria-label="托管笔记时间线" hidden></section>
<section id="library" aria-label="托管库配置与样式" hidden></section>
<section id="structure" aria-label="托管结构操作" hidden></section>
<section id="fields" aria-label="获权字段与查询" hidden></section>
<section id="query" aria-label="托管高级查询" hidden></section>
<section id="knowledge-links" aria-label="托管链接与潜在提及" hidden></section>
<section id="citations" aria-label="托管正式文献引用" hidden></section>
<section id="navigation-registry" aria-label="托管目录与标签导航" hidden></section>
<section id="review" aria-label="托管批注与校对" hidden></section>
<section id="media" aria-label="托管获权资源与媒体" hidden></section>
<section id="office" aria-label="Office 导入与输出" hidden></section>
<section id="content-import" aria-label="托管原生资料与文献导入" hidden></section>
<section id="owner-backup" aria-label="托管完整恢复资料备份" hidden></section>
<iframe id="render" title="获权内容预览" sandbox="" hidden></iframe></article></section>`;
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const fields = new HostedFields(el("fields"), request, (id, metadata) => office.setQuery(id, metadata));
const source: HostedEditor = new HostedEditor(el("source"), request, sourceChanged, () => { void run(save); }, error => status(error.message), {
  canOpenRow: row => (row as HostedQueryRow).canOpen === true && !!(row as HostedQueryRow).resource,
  openRow: row => { const target = row as HostedQueryRow; if (target.canOpen !== true || !target.resource) return Promise.reject(new Error("此查询行没有独立正文读取资格。")); return openQueryResource(target.resource); },
  openOwner: async () => { throw new Error("此查询来源没有独立获权导航对象，未按路径猜打开。"); },
  output: (query, result) => office.setQuery(query.queryId, { columns: result.columns ?? [], rowCount: result.rows.length, coverage: result.coverage }),
  modeChanged: () => { queries.authorChanged(); knowledgeAuthorChanged(); },
  navigateReference,
  navigateResource,
  dependencies: () => queries.dependencies(),
});
const review: HostedReview = new HostedReview(el("review"), request, () => doc ? {
  resource: doc.resource, sessionId: doc.sessionId, path: doc.resource, root: "", source: source.value,
  serial: source.serial, baseId: doc.baseId, dirty: source.value !== doc.source || compareRequired,
  pending: !!draft?.pending, selection: source.selection,
} : null, () => source.flush(), (from, to) => source.select(from, to), (resource, changed) => {
  if (changed && doc?.resource === resource) { compareRequired = true; knowledgeAuthorChanged(); controls(); status("建议已提交到服务端；个人草稿保留，请重新读取与核对正文。"); }
}, { context: () => media.context(), open: (asset, revision, initial) => media.openAsset(asset, revision, initial) });
let token = sessionStorage.getItem("weftext-hosted-token") ?? "";
let identity: Me | undefined;
let doc: Doc | undefined;
let draft: HostedDraft | undefined;
let cachePrefix = "";
let busy = false;
let requestEpoch = 0;
let authorizationEpoch = 0;
let compareRequired = false;
let installingDocument = false;
let draftTimer: ReturnType<typeof setTimeout> | undefined;
const retiredStructureSessions = new Set<string>();
const retiredStructureSources = new Set<string>();
function sessionAudience() { return { authorization: authorizationEpoch, credential: token, actor: identity?.actor, generation: identity?.generation, deployment: identity?.deployment }; }
async function closeOwnedSession(sessionId: string, captured: Pick<HostedNodeAudience, "authorization" | "credential" | "actor" | "generation" | "deployment">): Promise<void> {
  const current = sessionAudience(); if ((["authorization", "credential", "actor", "generation", "deployment"] as const).some(key => captured[key] !== current[key])) return;
  // Disposing a former coordinate must never be interpreted as revoking a
  // different view. Use the captured audience and no global error delivery.
  try { const response = await fetch("/api/hosted/close", { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${captured.credential}` }, body: JSON.stringify({ sessionId }), cache: "no-store" }); await response.arrayBuffer(); } catch { /* No alternate source is installed by cleanup. */ }
}
const status = (message: string) => { el("status").textContent = message; };
const office: HostedOffice = new HostedOffice(el("office"), request, () => doc ? {
  resource: doc.resource, source: source.value, sessionId: doc.sessionId, baseId: doc.baseId, serial: source.serial, paused: source.pausedSession, dirty: source.value !== doc.source || compareRequired, pending: !!draft?.pending || history.hasPending(doc.resource),
} : null, () => source.flush(), () => { void list().catch(e => status(e instanceof Error ? e.message : "刷新文档列表失败，请重试")); }, {
  identity: hostedIdentity, basis: () => source.historyBasis(true), embeddedQueries: () => source.outputSelections(), authorizationChanged: authorizationsChanged,
});
const queries = new HostedQuery(el("query"), request, openQueryResource, (id, metadata) => office.setQuery(id, metadata), {
  context: () => doc ? { resource: doc.resource, sessionId: doc.sessionId, baseId: doc.baseId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending, compare: compareRequired, definitionReservations: draft?.definitionReservations } : null,
  prepareBasis: () => source.prepareQueryBasis(),
  apply: (patches, basis) => source.applyQueryPatches(patches, basis, "input.query.definition.proposal"),
  reservations: (resource, receipts, includeReserved = false) => {
    if (!doc || !draft || resource !== doc.resource) return;
    draft.definitionReservations = mergeDefinitionReservations(draft.definitionReservations ?? [], receipts, includeReserved); retain();
  },
}, () => source.queryInputsChanged(), navigateResource);
const knowledgeLinks = new HostedKnowledgeLinks(el("knowledge-links"), (path, body, signal) => request(path, body, false, 0, signal), {
  identity: hostedIdentity,
  context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
  flush: async () => { await source.historyBasis(true); },
  apply: async (patches, basis, event) => { retain(); await source.applyQueryPatches(patches, basis, event); },
  navigate: reference => reference.startsWith("weftext-resource://") ? navigateResource(reference) : navigateReference(reference),
  select: (from, to) => source.selectSourceRange(from, to),
  authorizationChanged: (message, current) => { clearDisplay(message); identity = { ...identity!, ...current }; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(identity)); },
});
function hostedIdentity() { return identity ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, authorization: authorizationEpoch } : null; }
function authorizationsChanged(message: string, current: { actor: string; deployment: string; generation: number }) { clearDisplay(message); if (identity) identity = { ...identity, ...current }; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(identity)); }
const citations = new HostedCitations(el("citations"), request, {
  identity: hostedIdentity,
  context: () => doc && identity && !source.pausedSession ? { actor: identity.actor, deployment: identity.deployment, generation: identity.generation, credential: token, epoch: requestEpoch, authorization: authorizationEpoch, resource: doc.resource, sessionId: doc.sessionId, source: source.value, serial: source.serial, writable: doc.rights.write && source.viewMode !== "read", pending: !!draft?.pending || history.hasPending(doc.resource), compare: compareRequired } : null,
  basis: () => source.historyBasis(true), apply: async (patches, basis) => { retain(); await source.applyQueryPatches(patches, basis, "input.citation.proposal"); }, authorizationChanged: authorizationsChanged,
});
function knowledgeAuthorChanged() { knowledgeLinks.documentChanged(); citations.documentChanged(); }
const navigationRegistry = new HostedNavigationRegistry(el("navigation-registry"), request, { identity: hostedIdentity, authorizationChanged: authorizationsChanged });
const contentImport = new HostedContentImport(el("content-import"), request, { identity: hostedIdentity, authorizationChanged: authorizationsChanged, open: openQueryResource });
const ownerBackup = new HostedOwnerBackup(el("owner-backup"), request, { identity: hostedIdentity, authorizationChanged: authorizationsChanged });
const history = new HostedHistory(el("history-list"), request, {
  context: () => doc && draft ? { resource: doc.resource, sessionId: doc.sessionId, path: `resource:${doc.resource}`, root: "", baseId: doc.baseId, baseSource: doc.source, source: source.value, serial: source.serial, dirty: source.value !== doc.source, readOnly: !doc.rights.write || source.viewMode === "read", savePending: !!draft.pending, unresolvedSave: compareRequired } : null,
  basis: settle => source.historyBasis(settle), pendingChanged: () => { controls(); queries.authorChanged(); knowledgeAuthorChanged(); scheduleCurrentDraft(); }, restored: acknowledgeHistory, finishUnavailable: finishUnavailableHistory,
});
const library = new HostedLibrary(el("library"), request, async role => {
  const epoch = requestEpoch; await source.flush(); if (epoch !== requestEpoch) return;
  source.refreshEnvironment(); queries.environmentChanged(); knowledgeLinks.environmentChanged(); el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  if (role === "schema" || role === "rules") await queries.refreshCatalog();
  if (epoch === requestEpoch) status("库配置原操作已提交，受影响预览重新编译；正文草稿与撤销历史保留，未保存正文。");
});
const media: HostedMedia = new HostedMedia(el("media"), (path, body, signal) => request(path, body, false, 0, signal), context => review.mediaChanged(context), context => review.openMedia(context));
let structureDocuments: { resource: string; title: string }[] = [];
const structures = new HostedStructure(el("structure"), request, {
  protect: async (intent, phase) => {
    const epoch = requestEpoch, selected = doc; await source.flush(); if (epoch !== requestEpoch || doc !== selected) throw new Error("保护期间正文或用户已变化，未执行旧结构操作。"); retain();
    if (selected && draft && intent.source === selected.resource) {
      if (draft.pending || history.hasPending(selected.resource)) throw new Error("原正文保存或历史恢复仍未核对，请先处理原操作。");
      if (source.pausedSession) { if (phase === "commit" || intent.kind === "restore") return; throw new Error("原正文结构会话已暂停，请明确重新读取与核对后操作。"); }
      const ref = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: selected.sessionId });
      if (epoch !== requestEpoch || doc !== selected) throw new Error("核对期间原正文会话已变化。");
      draft = bindHostedDraftReference(draft, ref.reference); retain(); await request("draft", { sessionId: selected.sessionId, source: source.value });
      if (epoch !== requestEpoch || doc !== selected) throw new Error("草稿保护期间用户或对象已变化。"); retain();
      if (phase === "commit" && ["rename", "move", "trash", "restore"].includes(intent.kind)) { retiredStructureSessions.add(selected.sessionId); retiredStructureSources.add(selected.resource); clearTimeout(draftTimer); clearNodeNavigation(); source.pauseSession(); knowledgeAuthorChanged(); compareRequired = true; controls(); }
    }
  },
  changed: async (receipt, intent) => {
    const epoch = requestEpoch;
    if (doc && draft && intent.source === doc.resource && ["rename", "move", "trash", "restore"].includes(intent.kind)) { retiredStructureSessions.add(doc.sessionId); retiredStructureSources.add(doc.resource); retain(); clearTimeout(draftTimer); source.pauseSession(); knowledgeAuthorChanged(); compareRequired = true; controls(); }
    await list(); if (epoch !== requestEpoch) return; await queries.refreshCatalog(); if (epoch !== requestEpoch) return; await media.refresh(); if (epoch === requestEpoch) status("原结构操作已提交；个人草稿保留，结果仍需独立获权后明确打开。");
  },
  open: openStructureResult,
});
async function openStructureResult(result: NonNullable<StructureReceipt["result"]>, intent: StructureRequest): Promise<void> {
  if (!result.canOpen) throw new Error("结构结果尚无独立正文读取资格。");
  if (!["rename", "move", "restore"].includes(intent.kind)) { await openQueryResource(result.resource); return; }
  const epoch = requestEpoch, audience = sessionAudience(), selected = doc; await source.flush(); if (epoch !== requestEpoch || audience.credential !== token) return; retain(); const expected = `weftext-node://${result.nodeRef.workspaceId}/${result.nodeRef.nodeId}`;
  const original = selected?.resource === intent.source && draft ? { ...draft, source: source.value } : intent.source ? cached(intent.source) : undefined;
  const fresh = await request<Doc>("open", { resource: result.resource }); let adopted = false;
  try {
    if (epoch !== requestEpoch || doc !== selected) return;
    const ref = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: fresh.sessionId });
    if (epoch !== requestEpoch || doc !== selected || ref.reference !== expected || ref.resource !== fresh.resource) throw new Error("原结构对象的新位置资格已变化，未采用另一对象。");
    if (original?.reference === expected) {
      const retained = selected?.resource === intent.source && draft ? { ...draft, source: source.value } : original;
      if (selected?.resource === intent.source && source.pausedSession) source.rebindResourceAlias(fresh.resource);
      requestEpoch++; installDocument(fresh, retained); adopted = true;
      if (fresh.rights.write && retained.source !== fresh.source) { await request("draft", { sessionId: fresh.sessionId, source: retained.source }); if (doc === fresh) scheduleCurrentDraft(); }
    } else { requestEpoch++; installDocument(fresh, cached(fresh.resource) ?? { source: fresh.recoveredDraft ?? fresh.source, baseSource: fresh.recoveredDraftBaseSource ?? fresh.source, reference: ref.reference }); adopted = true; }
    if (selected) await closeOwnedSession(selected.sessionId, audience);
  } finally { if (!adopted) await closeOwnedSession(fresh.sessionId, audience); }
}
let stableReference: { resource: string; sessionId: string; reference: string } | undefined;
let nodeNavigationVersion = 0;
function clearNodeNavigation() { nodeNavigationVersion++; el("node-navigation").replaceChildren(); el("node-navigation").hidden = true; }
function nodeAudience(): HostedNodeAudience { return { epoch: requestEpoch, authorization: authorizationEpoch, credential: token, actor: identity?.actor, deployment: identity?.deployment, generation: identity?.generation, sessionId: doc?.sessionId, source: source.value, serial: source.serial, navigation: nodeNavigationVersion }; }
function nodeNavigationMessage(reference: string, message: string, onlyOpen = false) {
  const host = el("node-navigation"); host.hidden = false; host.replaceChildren();
  const original = document.createElement("input"); original.readOnly = true; original.value = reference; original.setAttribute("aria-label", "原稳定笔记片段链接");
  const text = document.createElement("p"); text.textContent = message; text.setAttribute("role", "status"); text.setAttribute("aria-label", "稳定笔记片段定位结果"); host.append(original, text);
  if (onlyOpen) { const base = reference.split("#", 1)[0], button = document.createElement("button"); button.textContent = "仅打开笔记，不定位原片段"; button.type = "button"; button.onclick = () => { void navigateReference(base).catch(error => status(error instanceof Error ? error.message : "目标当前不可用。")); }; host.append(button); }
}
async function navigateResource(reference: string) {
  const epoch = authorizationEpoch;
  if (doc) { await source.flush(); if (epoch !== authorizationEpoch) return; retain(); }
  if (epoch === authorizationEpoch) await media.openReference(reference);
}
async function navigateReference(reference: string) {
  if (busy) throw new Error("当前操作仍在核对，请保留原链接与正文草稿。");
  const epoch = requestEpoch; await source.flush(); retain(); if (epoch !== requestEpoch) return;
  clearNodeNavigation(); const captured = nodeAudience(); let staged: Doc | undefined, adopted = false;
  nodeNavigationMessage(reference, "正在获权解析原链接；标签准确定位前保留当前正文。");
  try {
      const parts = nodeReferenceParts(reference), resolved = await request<HostedNodeResolution>("navigation/resolve", { reference });
      if (!sameHostedNodeAudience(captured, nodeAudience())) return;
      if (!resolved.resource || resolved.reference !== reference) throw new Error("稳定链接回执与原地址不符，未打开其他对象。");
      if (parts.fragment === undefined) { if (resolved.canOpen !== true) throw new Error("稳定链接目标当前不可用，个人草稿保持保留。"); await run(() => open(resolved.resource)); return; }
      // The saved label result cannot qualify an actor's recovered or local Draft.
      const currentTarget = doc?.resource === resolved.resource;
      if (currentTarget && draft?.pending) throw new Error("当前目标的原保存操作尚未核对，请先按原编号恢复，未定位旧范围。");
      const prior = doc;
      let target: Doc, targetDraft: HostedDraft;
      if (currentTarget) { target = doc!; targetDraft = draft!; }
      else {
        staged = await request<Doc>("open", { resource: resolved.resource }); target = staged;
        if (!sameHostedNodeAudience(captured, nodeAudience())) return;
        if (target.resource !== resolved.resource) throw new Error("暂存目标与获权对象不符，未安装正文。");
        const raw = sessionStorage.getItem(cachePrefix + target.resource); let local: HostedDraft | undefined;
        try { local = raw === null ? undefined : JSON.parse(raw) as HostedDraft; } catch { throw new Error("目标的本页恢复稿无法解析；原记录保留，未安装另一份正文。"); }
        targetDraft = nodeTargetDraft(target.source, target.recoveredDraft, target.recoveredDraftBaseSource, source.pausedSession && draft?.reference === parts.base ? { ...draft, source: source.value } : local);
      }
      const retainedAlias = !currentTarget && source.pausedSession && draft?.reference === parts.base;
      const basis: HostedNodeBasis = { sessionId: target.sessionId, source: currentTarget ? source.value : targetDraft.source, serial: currentTarget || retainedAlias ? source.serial : 0, sourceHash: await nodeSourceHash(currentTarget ? source.value : targetDraft.source) };
      if (!sameHostedNodeAudience(captured, nodeAudience())) return;
      await request("editor/map", basis); if (!sameHostedNodeAudience(captured, nodeAudience())) return;
      const result = await request<HostedNodeFragmentResult>("navigation/fragment", { ...basis, reference });
      if (!sameHostedNodeAudience(captured, nodeAudience())) return;
      const origin = nodeFragmentOrigin(result, target.resource, reference, basis);
      if (!origin) { const reason = { missing: "目标当前个人草稿中缺少此标签", ambiguous: "目标当前个人草稿中标签有歧义", unavailable: "目标当前个人草稿无法准确定位此标签", found: "标签没有准确范围" }[result.fragment.status]; nodeNavigationMessage(reference, `${reason}；当前正文与 Undo 保留，未丢弃片段打开整篇。`, true); return; }
      if (busy) { nodeNavigationMessage(reference, "当前操作仍在核对，未采用迟到的片段范围；请操作完成后重新点击原链接。", true); return; }
      if (!currentTarget) {
        if (retainedAlias) source.rebindResourceAlias(target.resource);
        requestEpoch++; installDocument(target, targetDraft); adopted = true;
        if (prior) void closeOwnedSession(prior.sessionId, captured);
        scheduleCurrentDraft();
      }
      source.locateNodeFragment(origin, { sessionId: target.sessionId, source: basis.source, serial: basis.serial });
      nodeNavigationMessage(reference, `已按目标当前个人草稿唯一定位标签「${result.fragment.label}」的准确源码范围；正文与 Undo 未修改。`);
      status("已准确定位原片段，未保存正文。");
  } catch (error) {
      if (sameHostedNodeAudience(captured, nodeAudience())) { const message = `${error instanceof Error ? error.message : "片段定位未完成"} 当前正文保留。`; nodeNavigationMessage(reference, message, reference.includes("#")); status(message); }
  } finally {
      // Never send an old actor's temporary session to the newly connected actor.
      if (staged && !adopted) await closeOwnedSession(staged.sessionId, captured);
  }
}
async function readReference(selected: Doc, epoch: number) {
  stableReference = undefined; el("stable-link").hidden = true; el<HTMLInputElement>("node-reference").value = "";
  try { const result = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: selected.sessionId }); if (epoch !== requestEpoch || doc !== selected || !draft) return; if (result.resource !== selected.resource || !/^weftext-node:\/\//.test(result.reference)) throw new Error("当前笔记没有获权稳定身份。"); const qualified = bindHostedDraftReference({ ...draft, source: source.value }, result.reference); sessionStorage.setItem(cachePrefix + selected.resource, JSON.stringify(qualified)); draft = qualified; stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference: result.reference }; el<HTMLInputElement>("node-reference").value = result.reference; el("stable-link").hidden = false; controls(); } catch (error) { if (epoch === requestEpoch && doc === selected) status(error instanceof Error ? error.message : "稳定身份当前不可用。"); }
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
    el("title").textContent = fresh.title; adopted = true; retain(); history.documentChanged(); controls(); queries.authorChanged(true); knowledgeAuthorChanged(); void queries.refreshCatalog(); void readReference(fresh, epoch);
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
    compareRequired = true; el("comparison").hidden = false; el("current").textContent = fresh.source; el("title").textContent = fresh.title; adopted = true; retain(); history.documentChanged(); knowledgeAuthorChanged(); controls(); void readReference(fresh, epoch);
    try { await request("close", { sessionId: selected.sessionId }); } catch (error) { if (!(error instanceof HostedError) || error.http !== 410) throw error; }
  } finally { if (!adopted) { try { await request("close", { sessionId: fresh.sessionId }); } catch { /* Preserve original evidence when cleanup is unavailable. */ } } }
}
async function openQueryResource(resource: string) { const epoch = requestEpoch; await source.flush(); if (epoch !== requestEpoch) return; await run(() => open(resource)); }

class HostedError extends Error { constructor(public code: string, message: string, public http: number) { super(message); } }
function clearDisplay(message: string, clearToken = false) {
  retiredStructureSessions.clear(); retiredStructureSources.clear(); structureDocuments = [];
  clearNodeNavigation();
  requestEpoch++; authorizationEpoch++; doc = undefined; draft = undefined; compareRequired = false; cachePrefix = ""; stableReference = undefined;
  const clearErrors: unknown[] = [];
  // A blocked storage provider must never stop the remaining protected views
  // from disposing their in-memory capabilities and visible content.
  for (const clear of [() => source.clear(), () => knowledgeLinks.clear(), () => citations.clear(), () => navigationRegistry.clear(), () => contentImport.clear(), () => ownerBackup.clear(), () => fields.clear(), () => review.clear(), () => queries.clear(), () => office.clear(), () => history.clear(), () => library.clear(), () => media.clear(), () => structures.clear()]) {
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
async function request<T>(path: string, data?: unknown, bytes = false, attempt = 0, signal?: AbortSignal, failureScope?: "stagedTarget"): Promise<T> {
  const epoch = requestEpoch, credential = token;
  const audience = () => ({ epoch: authorizationEpoch, credential: token, actor: identity?.actor, deployment: identity?.deployment, generation: identity?.generation, sessionId: doc?.sessionId, queryVersion: queries.version }), captured = audience();
  const response = await fetch(`/api/hosted/${path}`, {method:data === undefined ? "GET" : "POST", headers:{"Content-Type":"application/json",Authorization:`Bearer ${token}`},body:data === undefined ? undefined : JSON.stringify(data),cache:"no-store", signal});
  if (!response.ok) {
    const result = await response.json() as {error?:{code:string;message:string}};
    const requestTarget = data as { sessionId?: string; resource?: string } | undefined, retired = !!requestTarget && (typeof requestTarget.sessionId === "string" && retiredStructureSessions.has(requestTarget.sessionId) || typeof requestTarget.resource === "string" && retiredStructureSources.has(requestTarget.resource));
    // Only a read-only preview can wait for the shared compiler. Author save
    // requests keep their explicit original-operation recovery path.
    if (path === "preview" && response.status === 429 && attempt < 5) {
      await new Promise(resolve => setTimeout(resolve, 500));
      if (epoch === requestEpoch && credential === token) return request(path, data, bytes, attempt + 1, signal);
    }
    if (response.status === 401 && sameHostedQueryIdentity(captured, audience())) clearDisplay("凭据失效，已清除本页缓存。", true);
    if ([404, 409, 410].includes(response.status) && retired && sameHostedQueryIdentity(captured, audience())) {
      try { const current = await request<Me>("me"); if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) { clearDisplay("授权策略已变化，旧展示与缓存已清除。"); identity = current; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current)); } } catch { /* A retired structural coordinate never clears a newer buffer or actor. */ }
    } else if (response.status === 404 && path === "query" && sameHostedQueryIdentity(captured, audience())) {
      try { const current = await request<Me>("me"); if (sameHostedQueryIdentity(captured, audience())) { if (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation) { clearDisplay("授权策略已变化，旧展示与缓存已清除，请刷新文档列表。"); identity = current; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current)); } else if (sameHostedQueryAudience(captured, audience())) queries.refuseResult("此查询对象或范围不可用；原计划、参数和个人正文草稿保留，未交付结果。"); } }
      catch { if (sameHostedQueryAudience(captured, audience())) queries.refuseResult("查询资格核对未完成，结果暂不交付；原计划与个人草稿保留，请重新连接核对。"); }
    } else if ([404, 410].includes(response.status) && path.startsWith("media/") && sameHostedQueryIdentity(captured, audience())) {
      // A hidden/unknown asset is not a revocation of the independently open
      // note. Only a current identity's changed actor epoch clears all views.
      try { const current = await request<Me>("me"); if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) { clearDisplay("授权策略已变化，旧媒体与缓存已清除，请刷新目录。"); identity = current; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current)); } } catch { /* Caller stops this resource delivery; personal source remains protected. */ }
    } else if ([404, 409, 410].includes(response.status) && (failureScope === "stagedTarget" || path === "open" || path.startsWith("navigation/") || path.startsWith("structure/") || path.startsWith("knowledge-links/") || path.startsWith("citations/") || path.startsWith("export/") || path.startsWith("import/") || path.startsWith("owner-backup/")) && sameHostedQueryIdentity(captured, audience())) {
      try { const current = await request<Me>("me"); if (sameHostedQueryIdentity(captured, audience()) && (current.actor !== captured.actor || current.deployment !== captured.deployment || current.generation !== captured.generation)) { clearDisplay("授权策略已变化，旧链接定位与缓存已清除，请刷新列表。"); identity = current; sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current)); } } catch { /* Stop this link delivery without treating a hidden same-epoch target as source revocation. */ }
    } else if (response.status === 404 && failureScope !== "stagedTarget" && !retired && path !== "query" && path !== "open" && !path.startsWith("navigation/") && !path.startsWith("structure/") && !path.startsWith("knowledge-links/") && !path.startsWith("citations/") && !path.startsWith("export/") && !path.startsWith("import/") && !path.startsWith("owner-backup/") && !path.startsWith("media/") && epoch === requestEpoch && credential === token) clearDisplay("对象或授权已变化，已清除旧内容。请刷新文档列表。");
    throw new HostedError(result.error?.code ?? "request", result.error?.code === "basisRequired" && path.startsWith("query/") ? "保存带范围的定义需要已安装的规则基线；请由管理员明确安装库规则后重试，原草稿与提案保留，未自动创建规则。" : result.error?.message ?? "请求失败", response.status);
  }
  return (bytes ? response.arrayBuffer() : response.json()) as Promise<T>;
}
function controls() {
  el<HTMLButtonElement>("save").disabled = busy || !doc?.rights.write || source.pausedSession || compareRequired || !!draft?.pending || !!doc && history.hasPending(doc.resource);
  for (const id of ["preview","reopen"]) el<HTMLButtonElement>(id).disabled = busy || !doc || source.pausedSession && id === "preview";
  el<HTMLButtonElement>("download").disabled = busy || !doc?.rights.download || source.pausedSession;
  el<HTMLButtonElement>("history").disabled = busy || !doc?.rights.history || source.pausedSession;
  el<HTMLButtonElement>("rebase").disabled = busy || !doc?.rights.write || source.pausedSession || !!draft?.pending;
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
  status(`${next.actor} · ${next.version} · 已连接`); await list(); await fields.connect(next.deployment, next.actor, next.generation); await review.connect(next.deployment, next.actor, next.generation); await queries.connect(next.deployment, next.actor, next.generation); await office.connect(next.deployment, next.actor, next.generation); await history.connect(next.deployment, next.actor, next.generation); await library.connect(next.deployment, next.actor, next.generation); await media.connect(next.deployment, next.actor, next.generation); await structures.connect(next.deployment, next.actor, next.generation); structures.setDocuments(structureDocuments); knowledgeLinks.connect(); await citations.connect(identity!.deployment, identity!.actor, identity!.generation); await navigationRegistry.connect(identity!.deployment, identity!.actor, identity!.generation);
  if (identity?.actor !== next.actor || identity.deployment !== next.deployment || identity.generation !== next.generation) return;
  await contentImport.connect(next.deployment, next.actor, next.generation); await ownerBackup.connect(next.deployment, next.actor, next.generation);
}
async function list(term?: string) {
  const epoch = requestEpoch;
  const rows = term ? (await request<{rows: {resource:string;title:string}[];complete:boolean}>("search",{text:term})) : undefined;
  const items = rows?.rows ?? (await request<{documents:{resource:string;title:string}[]}>("documents")).documents;
  if (epoch !== requestEpoch) return;
  structureDocuments = items; structures.setDocuments(items);
  el("documents").replaceChildren(...items.map(item => { const button = document.createElement("button"); button.textContent = item.title; button.onclick = () => run(() => open(item.resource)); return button; }));
  if (rows) status(rows.complete ? `在可访问范围找到 ${items.length} 项` : `搜索未完整，仅显示已取得的 ${items.length} 项`);
}
async function open(resource: string) {
  clearNodeNavigation();
  const prior = doc, captured = nodeAudience();
  const current = () => doc === prior && sameHostedNodeAudience(captured, nodeAudience());
  const sameIdentity = () => { const now = sessionAudience(); return (["authorization", "credential", "actor", "generation", "deployment"] as const).every(key => captured[key] === now[key]); };
  const authority = async () => { if (!sameIdentity()) return false; const next = await request<Me>("me"); if (!sameIdentity()) return false; if (next.actor !== captured.actor || next.deployment !== captured.deployment || next.generation !== captured.generation) { authorizationsChanged("授权策略已更新，旧展示与缓存已清除。请重新选择文档。", next); return false; } return true; };
  await source.flush(); if (!current()) return; retain();
  status("正在核验目标；现有草稿与 Undo 保留，尚未切换文档。");
  const currentIdentity = await request<Me>("me");
  if (!sameIdentity()) return;
  if (identity && (currentIdentity.generation !== identity.generation || currentIdentity.actor !== identity.actor || currentIdentity.deployment !== identity.deployment)) {
    authorizationsChanged("授权策略已更新，旧展示与缓存已清除。请重新选择文档。", currentIdentity); await list(); return;
  }
  if (!current()) return;
  let opened: Doc | undefined, adopted = false;
  try {
    opened = await request<Doc>("open", { resource }); if (!await authority() || !current()) return;
    if (opened.resource !== resource) throw new Error("打开回执不对应原目标，当前草稿保留。");
    let targetDraft = cached(resource) ?? { source: opened.recoveredDraft ?? opened.source, baseSource: opened.recoveredDraftBaseSource ?? opened.source };
    let alias = false, recoveredSave = false;
    if (prior && draft?.reference && source.pausedSession && prior.resource !== resource) {
      const ref = await request<{ resource: string; reference: string }>("navigation/ref", { sessionId: opened.sessionId }); if (!current()) return;
      if (ref.resource === opened.resource && ref.reference === draft.reference) { targetDraft = { ...draft, source: source.value }; alias = true; }
    }
    if (targetDraft.pending) {
      let outcome = await request<SaveStatus>("operation", { operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget"); if (!current()) return;
      if (["notRegistered", "notCommitted"].includes(outcome.status)) outcome = await request<SaveStatus>("operation/settle", { resource, operationId: targetDraft.pending.operationId }, false, 0, undefined, "stagedTarget"); if (!current()) return;
      if (outcome.status === "committed") { targetDraft = applySaveResult(targetDraft, targetDraft.pending.source); recoveredSave = true; }
      else if (["cancelled", "resolved"].includes(outcome.status)) delete targetDraft.pending;
    }
    if (!await authority() || !current()) return; retain();
    targetDraft = hostedOpenedDraft(opened.source, targetDraft);
    // A quota failure must happen while the original editor and session still
    // exist. This exact prepared cache is not written again during install.
    try { sessionStorage.setItem(cachePrefix + opened.resource, JSON.stringify(targetDraft)); }
    catch { throw new Error("浏览器无法可靠保留目标恢复稿；当前草稿、Undo 与原会话保留，未切换文档。"); }
    // Install only after every target qualification/recovery succeeds. Closing
    // the former session uses its captured actor and cannot clear the new view.
    requestEpoch++; if (alias) source.rebindResourceAlias(resource); installDocument(opened, targetDraft, recoveredSave, true); adopted = true;
    if (alias) scheduleCurrentDraft(); if (prior) await closeOwnedSession(prior.sessionId, captured);
  } finally { if (opened && !adopted) await closeOwnedSession(opened.sessionId, captured); }
}
function installDocument(opened: Doc, retainedDraft: HostedDraft, recoveredSave = false, stagedCache = false) {
  clearNodeNavigation(); doc = opened; draft = stagedCache ? retainedDraft : hostedOpenedDraft(opened.source, retainedDraft); const epoch = requestEpoch;
  installingDocument = true;
  try { source.open(opened.sessionId, opened.resource, draft.source); source.disabled = !opened.rights.write; } finally { installingDocument = false; }
  el("title").textContent = opened.title;
  compareRequired = opened.recoveredDraftConflict || draftDisposition(opened.source,draft) === "compare";
  el("comparison").hidden = !compareRequired; el("current").textContent = compareRequired ? opened.source : "";
  history.documentChanged();
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  if (!stagedCache) retain(); controls(); queries.authorChanged(true); knowledgeAuthorChanged();
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
  draft.pending = {operationId:op,source:sent,request:exactRequest}; retain(); controls(); queries.authorChanged(); knowledgeAuthorChanged();
  try {
    const result = await request<SaveStatus>("save",exactRequest);
    if (epoch !== requestEpoch || !doc || !draft) return;
    if (result.status !== "committed") { status("保存尚未确认，请核对原操作结果。"); return; }
    doc.baseId = result.baseId!; doc.source = sent;
    draft = applySaveResult(draft,sent); retain(); source.refreshQueries(); queries.authorChanged(true); knowledgeAuthorChanged(); void queries.refreshCatalog(); status("已保存。" + (draft.source !== sent ? "新输入仍在草稿中。" : ""));
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
  if (installingDocument) return;
  clearNodeNavigation();
  el<HTMLIFrameElement>("render").srcdoc = ""; el("render").hidden = true;
  try { retain(); status(navigator.onLine ? "个人草稿，尚未保存正文。" : "连接中断，本页草稿仍保留。"); } catch { status("浏览器无法保留恢复稿，请先复制你的输入。"); }
  queries.authorChanged(); knowledgeAuthorChanged();
  scheduleCurrentDraft();
}
function scheduleCurrentDraft() {
  clearTimeout(draftTimer);
  const captured = doc; const epoch = requestEpoch;
  draftTimer = setTimeout(() => {
    if (!captured || doc !== captured || epoch !== requestEpoch || source.pausedSession || !captured.rights.write || compareRequired || draft?.pending || busy || history.hasPending(captured.resource)) return;
    void request("draft",{sessionId:captured.sessionId,source:source.value}).catch(() => {});
  },800);
}
el<HTMLFormElement>("login").onsubmit = e => {e.preventDefault(); token = el<HTMLInputElement>("token").value.trim(); void run(connect);};
el("logout").onclick = () => { clearDisplay("已退出。",true); el<HTMLInputElement>("token").value = ""; };
el("refresh").onclick = () => void run(async () => {
  const epoch = requestEpoch, current = await request<Me>("me"); if (epoch !== requestEpoch) return;
  if (!identity || current.generation !== identity.generation || current.actor !== identity.actor || current.deployment !== identity.deployment) clearDisplay("授权策略已更新，旧展示与缓存已清除。");
  identity = current; cachePrefix = hostedCachePrefix("draft", current.deployment, current.actor, current.generation); sessionStorage.setItem("weftext-hosted-identity", JSON.stringify(current));
  await list(); await fields.connect(current.deployment, current.actor, current.generation); await review.connect(current.deployment, current.actor, current.generation); await queries.connect(current.deployment, current.actor, current.generation); await office.connect(current.deployment, current.actor, current.generation); await history.connect(current.deployment, current.actor, current.generation); await library.connect(current.deployment, current.actor, current.generation); await media.connect(current.deployment, current.actor, current.generation); await structures.connect(current.deployment, current.actor, current.generation); structures.setDocuments(structureDocuments); knowledgeLinks.connect(); await citations.connect(identity!.deployment, identity!.actor, identity!.generation); await navigationRegistry.connect(identity!.deployment, identity!.actor, identity!.generation);
  if (identity?.actor !== current.actor || identity.deployment !== current.deployment || identity.generation !== current.generation) return;
  await contentImport.connect(current.deployment, current.actor, current.generation); await ownerBackup.connect(current.deployment, current.actor, current.generation);
});
el<HTMLFormElement>("search").onsubmit = e => {e.preventDefault();void run(() => list(el<HTMLInputElement>("term").value.trim()));};
el("save").onclick = () => void run(save);
el("reopen").onclick = () => void run(() => open(doc!.resource));
el("rebase").onclick = () => void run(async () => {
  if (!doc || !draft) return;
  await source.flush();
  await request("draft/resolve",{sessionId:doc.sessionId,source:source.value});
  draft.baseSource = doc.source; compareRequired = false; knowledgeAuthorChanged(); retain(); el("comparison").hidden = true; status("合并依据已确认，原草稿已保留；点击保存才提交正文。");
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
window.addEventListener("beforeunload", e=>{if(structures.hasPending || media.hasPending || library.hasPending || history.hasPending() || draft && (draft.pending || draft.source!==draft.baseSource)){e.preventDefault();}});
setInterval(()=>{
  if(!identity || busy) return;
  const old=identity, epoch=requestEpoch, credential=token;
  void request<Me>("me").then(next=>{
    if(epoch!==requestEpoch || credential!==token || identity!==old) return;
    if(next.generation!==old.generation || next.actor!==old.actor || next.deployment!==old.deployment){clearDisplay("授权策略已变化，旧展示与缓存已清除，请刷新文档列表。");identity=next;sessionStorage.setItem("weftext-hosted-identity",JSON.stringify(next));}
  }).catch(()=>{});
},5000);
if(token) void run(connect);
