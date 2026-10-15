import { api, operationId } from "./api";
import { SyncFlow, freezeSyncAdoptionRequest, objectKey, sameSyncIdentity, type ObjectKey, type ReceivedSyncVersions, type SyncAdoptionCut, type SyncBinding, type SyncIdentity, type SyncObject, type SyncPending, type SyncRevalidationCut, type SyncStatus, type SyncVersion } from "./sync-state";
import "./sync-panel.css";
import { mountRemoteSync } from "./sync-remote";
import { appendSyncRevalidationSummary, mountSyncRevalidation } from "./sync-revalidation-panel";

interface SyncHost { beforeAction(): Promise<void>; applied(root: string): Promise<void> }
const savedKey = "weftext-sync-operation-v1";
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] { const e = document.createElement(tag); e.textContent = text; return e; }
function field(label: string, value = ""): { label: HTMLLabelElement; input: HTMLInputElement } {
  const wrapper = el("label", label), input = el("input"); input.value = value; wrapper.append(input); return { label: wrapper, input };
}
export async function openSyncPanel(root: string, host: SyncHost): Promise<void> {
  const saved = localStorage.getItem(savedKey);
  const restored: SyncPending | null = saved ? JSON.parse(saved) : null;
  if (restored && (typeof restored.request?.operationId !== "string" || typeof restored.request?.root !== "string" || typeof restored.attempted !== "boolean" || typeof restored.status !== "string" || (restored.plan && (restored.plan.operationId !== restored.request.operationId || typeof restored.plan.planId !== "string")) || !["cloneSync", "applySync", "mergeSync", "adoptSyncExternal", "revalidateSync"].includes(restored.request?.intent?.kind))) throw new Error("保存的同步操作记录无法读取；请保留浏览器记录并检查恢复记录。");
  const flow = new SyncFlow({ request: api, store: p => { if (p) localStorage.setItem(savedKey, JSON.stringify(p)); else localStorage.removeItem(savedKey); } }, restored);
  const dialog = el("dialog"); dialog.className = "sync-panel"; dialog.setAttribute("aria-label", "资料库同步");
  const rootField = field("本机资料库路径", restored?.request.root ?? root);
  const pairLabel = el("label", "配对身份（从原副本复制）"), pair = el("textarea"); pair.rows = 3; pairLabel.append(pair);
  if (restored) pair.value = JSON.stringify(restored.request.intent.identity);
  const transport = field("交换目录路径（新目录或已配对目录）");
  const original = field("原同步操作编号（地址变化或浏览器记录丢失时）");
  const statusLine = el("p"); statusLine.setAttribute("role", "status");
  const info = el("p"), objectsHost = el("section"), pendingHost = el("section"), controls = el("div"), historyHost = el("section"), inspectHost = el("section"), revalidationHost = el("section");
  historyHost.setAttribute("aria-label", "收到的对象历史");
  let binding: SyncBinding | null = null, status: SyncStatus | null = null, busy = false;
  const selected = new Set<string>(); let selectedInitialized = false;
  let remoteControls: ReturnType<typeof mountRemoteSync> | undefined;
  let revalidation: ReturnType<typeof mountSyncRevalidation> | undefined, reviewGeneration = 0;
  function clearReview(): void { reviewGeneration++; revalidation?.dispose(); revalidation = undefined; revalidationHost.replaceChildren(); }
  const identity = (): SyncIdentity => {
    const value = JSON.parse(pair.value) as SyncIdentity;
    if (!value || typeof value.workspaceId !== "string" || typeof value.genesis !== "string") throw new Error("请输入原副本的完整配对身份。");
    return { workspaceId: value.workspaceId, genesis: value.genesis };
  };
  const targetRoot = (): string => { if (!rootField.input.value.trim()) throw new Error("请输入本机资料库路径。"); return rootField.input.value; };
  const run = async (action: () => void | Promise<void>): Promise<void> => {
    if (busy) return;
    busy = true; lock(); statusLine.textContent = "正在读取或处理…";
    try { await action(); statusLine.textContent = "操作已完成。"; }
    catch (error) { statusLine.textContent = `${error instanceof Error ? error.message : String(error)}${flow.pending ? " 原操作记录保留，可检查原结果。" : ""}`; }
    finally { busy = false; render(); }
  };
  function button(label: string, action: () => void | Promise<void>): HTMLButtonElement {
    const b = el("button", label); b.type = "button"; b.onclick = () => { void run(action); }; return b;
  }
  function lock(): void { dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = true; }); }
  async function load(): Promise<void> {
    clearReview();
    binding = await api<SyncBinding | null>("/api/sync/binding", { root: targetRoot() });
    if (binding) pair.value = JSON.stringify(binding.identity);
    status = binding ? await api<SyncStatus>("/api/sync/status", { identity: binding.identity }) : null;
    selectedInitialized = false;
    historyHost.replaceChildren(); inspectHost.replaceChildren();
  }
  async function published(): Promise<void> {
    clearReview();
    status = await api<SyncStatus>("/api/sync/publish", { root: targetRoot() });
    binding = await api<SyncBinding>("/api/sync/binding", { root: targetRoot() });
  }
  async function refreshCommitted(): Promise<void> {
    if (flow.pending?.status === "committed") {
      await host.applied(flow.pending.request.root); await load(); inspectHost.replaceChildren();
    }
  }
  function picked(): SyncObject[] {
    const values = status?.objects.filter(o => selected.has(objectKey(o.object))) ?? [];
    if (!values.length) throw new Error("请至少选择一个对象。"); return values;
  }
  async function prepare(kind: "cloneSync" | "applySync"): Promise<void> {
    clearReview();
    await host.beforeAction();
    await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind, identity: identity(), objects: picked().map(o => o.object) } });
  }
  async function prepareAdoption(): Promise<void> {
    clearReview();
    const currentRoot = targetRoot(), paired = identity(), objects = picked().map(o => o.object);
    await host.beforeAction();
    const cut = await api<SyncAdoptionCut>("/api/sync/adoption-cut", { root: currentRoot, identity: paired, objects });
    await flow.prepare(freezeSyncAdoptionRequest(currentRoot, paired, cut, operationId()));
  }
  async function beginRevalidation(objects: ObjectKey[], initial: { object: ObjectKey; revision: string }[]): Promise<void> {
    if (flow.pending) throw new Error("请先处理原同步操作。");
    clearReview(); const generation = reviewGeneration, currentRoot = targetRoot(), paired = identity();
    const cut = await api<SyncRevalidationCut>("/api/sync/revalidation-cut", { root: currentRoot, identity: paired, objects });
    if (generation !== reviewGeneration || currentRoot !== targetRoot() || !sameSyncIdentity(paired, identity()) || flow.pending) throw new Error("读取期间同步依据已变化，请重新打开语义审阅。");
    revalidation = mountSyncRevalidation(revalidationHost, currentRoot, paired, cut, initial, { request: api, run, beforeAction: host.beforeAction, id: operationId, prepare: request => flow.prepare(request, clearReview) });
  }
  async function inspectVersion(o: SyncObject, version: SyncVersion, kind: "head" | "base" | "history"): Promise<void> {
    const title = el("h3", `${kind === "base" ? "共同基础" : kind === "history" ? "历史版本（只读）" : "版本"} ${version.revision}`);
    inspectHost.replaceChildren(title, el("pre", JSON.stringify({ parent: version.state.parent, name: version.state.name, retired: version.state.retired }, null, 2)));
    for (const [path, digest] of Object.entries(version.state.files)) {
      const section = el("details"); section.append(el("summary", path), el("code", digest));
      const content = el("pre"); section.append(content, button(`读取 ${path}`, async () => {
        const result = await api<{ text: string | null; bytes: number }>("/api/sync/file", { identity: identity(), object: o.object, revision: version.revision, path });
        content.textContent = result.text ?? `二进制文件或超过文本预览预算：${result.bytes} 字节。`;
      })); inspectHost.append(section);
    }
    if (kind === "head" && binding && o.view.versions.length > 1) {
      inspectHost.append(el("p", "采用此分支会选择它的完整正文、批注、资源和位置。双方原版本仍保留在历史中。"), button("预览采用此完整分支", async () => {
        await host.beforeAction();
        await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind: "mergeSync", identity: identity(), resolutions: [{ object: o.object, revisions: o.view.versions.map(v => v.revision), state: version.state }], blobs: {} } });
      }));
    }
    if (kind === "head" && binding) {
      const review = button("审阅此版本的语义", () => beginRevalidation([o.object], [{ object: o.object, revision: version.revision }])); review.dataset.syncRevalidationStart = version.revision; inspectHost.append(review);
    }
  }
  async function inspectHistory(o: SyncObject, afterCommit?: string): Promise<void> {
    const result = await api<ReceivedSyncVersions>("/api/sync/history", { identity: identity(), object: o.object, afterCommit, limit: 25 });
    const installed = binding?.installedVersions.find(([key]) => objectKey(key) === objectKey(o.object))?.[1];
    historyHost.replaceChildren(el("h3", `收到的对象历史 · ${objectKey(o.object)}`),
      el("p", "此处只读检查已收到的版本，不改变本机文档或草稿。列表按提交编号分页，不代表时间先后；因果关系见各版本的父提交与前置版本。接收新记录后可重新读取首页。"),
      el("p", result.coverage === "complete" ? "当前已收到的对象记录均已接纳；这不表示本机已经安装这些版本，也不表示远端历史已经全部收到。" : "当前已收到的对象记录中有尚未接纳的版本；缺件或拒绝记录不可读取正文。"),
      button("重新读取历史首页", () => inspectHistory(o)));
    if (!result.versions.length) historyHost.append(el("p", "当前未收到此对象的历史记录。"));
    for (const version of result.versions) {
      const entry = el("details"), accepted = version.admission.status === "accepted";
      entry.append(el("summary", `${version.revision} · ${accepted ? "已接纳" : version.admission.status}${installed === version.revision ? " · 本机已安装" : " · 本机未安装此版本"}`),
        el("pre", JSON.stringify({ commit: version.commit, parents: version.parents, before: version.before, admission: version.admission, name: version.state.name, retired: version.state.retired }, null, 2)));
      if (accepted) entry.append(button(`只读查看历史 ${version.revision}`, () => inspectVersion(o, version, "history")));
      historyHost.append(entry);
    }
    if (result.nextCommit) historyHost.append(button("下一页历史", () => inspectHistory(o, result.nextCommit!)));
    inspectHost.replaceChildren();
  }
  function render(): void {
    dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = false; });
    rootField.input.disabled = !!flow.pending; pair.readOnly = !!binding || !!flow.pending;
    info.textContent = binding ? `副本 ${binding.replicaId} · ${binding.partial ? "部分同步" : "完整副本"} · 已安装 ${binding.installedObjects.length} 个对象 · 待发布 ${binding.pendingPublications} 笔本地提交` : "此路径尚未启用同步。可以启用现有资料库，或接收历史后在尚不存在的路径创建独立副本。";
    controls.replaceChildren(button("读取本机状态", load), button("启用同步", async () => {
      const path = targetRoot(), key = `weftext-sync-enable:${path}`;
      const id = localStorage.getItem(key) ?? operationId(); localStorage.setItem(key, id);
      binding = await api<SyncBinding>("/api/sync/enable", { root: path, operationId: id }); pair.value = JSON.stringify(binding.identity);
      await published(); localStorage.removeItem(key);
    }), button("刷新已保存版本", published), button("接收交换目录", async () => {
      clearReview();
      status = await api<SyncStatus>("/api/sync/receive", { identity: identity(), directory: transport.input.value }); selectedInitialized = false; historyHost.replaceChildren(); inspectHost.replaceChildren();
    }), button("导出所选对象", async () => {
      await api("/api/sync/export", { root: targetRoot(), directory: transport.input.value, objects: picked().map(o => o.object) }); await published();
    }), button("预览应用所选对象", () => prepare("applySync")), button("预览创建独立副本", () => prepare("cloneSync")), button("预览采纳当前外部修改", prepareAdoption), button("审阅所选对象的语义", () => { const objects = picked(); return beginRevalidation(objects.map(o => o.object), objects.filter(o => o.view.versions.length === 1).map(o => ({ object: o.object, revision: o.view.versions[0].revision }))); }));
    const buttons = Array.from(controls.querySelectorAll("button"));
    buttons[1].disabled = !!binding || !!flow.pending; buttons[2].disabled = !binding || !!flow.pending;
    buttons[4].disabled = !binding || !!flow.pending; buttons[5].disabled = !binding || !status || !!flow.pending; buttons[6].disabled = !!binding || !status || !!flow.pending;
    buttons[0].disabled = !!flow.pending; buttons[3].disabled = !!flow.pending;
    buttons[7].disabled = !binding || !status || !!flow.pending;
    buttons[8].disabled = !binding || !status || !!flow.pending;
    objectsHost.replaceChildren();
    if (status) {
      if (!selectedInitialized) { selected.clear(); for (const o of status.objects) selected.add(objectKey(o.object)); selectedInitialized = true; }
      objectsHost.append(el("h3", `接收历史：${status.commits.length} 笔提交 · ${status.heads.length} 个因果分支头`), el("p", "接收不改变本机文件。所选对象会带上原事务及依赖闭包；准确影响范围以应用预览为准。版本编号不表示时间顺序。"));
      const unsettled = status.commits.filter(c => c.admission.status !== "accepted" || c.unsettledDependencies.length);
      if (unsettled.length) { const details = el("details"); details.append(el("summary", `${unsettled.length} 笔历史提交有缺件、拒绝或依赖与当前版本不同`), el("p", "旧提交的依赖与当前不同并不表示所有对象被阻止；实际所选版本能否应用，以准备结果为准。"), el("pre", JSON.stringify(unsettled, null, 2))); objectsHost.append(details); }
      const installed = new Map(binding?.installedVersions.map(([key, revision]) => [objectKey(key), revision]) ?? []);
      for (const o of status.objects) {
        const row = el("div"), label = el("label"), check = el("input"); row.className = "sync-object"; check.type = "checkbox"; check.checked = selected.has(objectKey(o.object)); check.disabled = !!flow.pending;
        check.onchange = () => { if (check.checked) selected.add(objectKey(o.object)); else selected.delete(objectKey(o.object)); };
        label.append(check, document.createTextNode(`${o.view.versions.map(v => v.state.name).filter((n, i, a) => a.indexOf(n) === i).join(" / ") || o.object.kind} (${objectKey(o.object)})`)); row.append(label);
        row.append(el("p", `本机已安装版本：${installed.get(objectKey(o.object)) ?? "未安装"}${o.view.versions.length > 1 ? " · 冲突：保留全部分支" : ""}`));
        for (const v of o.view.versions) row.append(button(`查看版本 ${v.revision}`, () => inspectVersion(o, v, "head")));
        for (const v of o.view.commonBases) row.append(button(`查看共同基础 ${v.revision}`, () => inspectVersion(o, v, "base")));
        row.append(button("查看收到的全部历史", () => inspectHistory(o)));
        objectsHost.append(row);
      }
    }
    pendingHost.replaceChildren();
    const p = flow.pending;
    if (p) {
      pendingHost.append(el("h3", `原同步操作：${p.status}`), el("code", p.request.operationId), el("p", `目标：${p.request.root} · ${p.request.intent.kind}`));
      if (p.request.intent.kind === "adoptSyncExternal") {
        pendingHost.append(el("p", "明确采纳本机已存在的外部修改，将核对后的文件内容与资源状态登记为新的本地版本。此入口不修改正文、不同时修复内容，也不覆盖个人草稿；尚未收到或未安装的版本仍独立保留。"),
          el("p", "本次核对的已安装对象与版本："), el("pre", JSON.stringify(p.request.intent.expectedInstalled, null, 2)));
        for (const file of p.request.intent.changes) {
          const details = el("details"); details.append(el("summary", `${file.path} · ${file.after.length} 字节`));
          try { details.append(el("pre", new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array(file.after)))); }
          catch { details.append(el("p", "此文件不是 UTF-8 文本；原请求保留准确字节。")); }
          pendingHost.append(details);
        }
        if (p.request.intent.removedMembers?.length) {
          const list = el("ul");
          for (const member of p.request.intent.removedMembers) list.append(el("li", member.path));
          pendingHost.append(el("p", "以下资源已在外部移除；确认采纳会记录这一现状："), list);
        }
      }
      if (p.request.intent.kind === "revalidateSync") {
        const intent = p.request.intent;
        pendingHost.append(el("p", "本次明确重新验证选择与准确修复内容已保留。准备、提交和恢复继续使用此原操作；个人草稿与撤销记录保留。"));
        appendSyncRevalidationSummary(pendingHost, intent, status?.objects ?? []);
        for (const resolution of intent.resolutions) for (const [path, digest] of Object.entries(resolution.state.files)) if (intent.blobs[digest]) {
          const detail = el("details"); detail.append(el("summary", `明确修复 ${objectKey(resolution.object)} / ${path}`));
          try { detail.append(el("pre", new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(new Uint8Array(intent.blobs[digest])))); }
          catch { detail.append(el("p", "原请求保留准确二进制字节。")); } pendingHost.append(detail);
        }
      }
      if (p.plan) { const list = el("ul"); for (const c of p.plan.changes) list.append(el("li", `${c.action} · ${c.path} · ${c.bytes} 字节`)); pendingHost.append(list); for (const loss of p.plan.losses ?? []) pendingHost.append(el("p", loss.message)); }
      pendingHost.append(button("检查原操作结果", async () => { await flow.inspect(); await refreshCommitted(); }));
      if (!p.attempted && !p.plan) pendingHost.append(button("重试原准备请求", () => flow.retryPrepare()));
      if (p.status === "prepared") pendingHost.append(button("确认应用此预览", async () => { await host.beforeAction(); await flow.commit(); await refreshCommitted(); }));
      if (["committed", "resolved"].includes(p.status) || !p.attempted) pendingHost.append(button(["committed", "resolved"].includes(p.status) ? "结束已确认操作" : "放弃本次预览", () => { flow.forget(); inspectHost.replaceChildren(); }));
      if (!p.attempted && !["committed", "resolved"].includes(p.status)) pendingHost.append(el("p", "放弃预览不会应用内容，也不代表服务端原操作已结束。明确取消需要原准备结果。"));
      if (p.status === "prepared") pendingHost.append(button("取消尚未开始写入的原提案", async () => { await flow.cancelPrepared(); await refreshCommitted(); }));
      if (["unknown", "recoveryPending"].includes(p.status)) pendingHost.append(el("p", "如需处理已发生的部分步骤，请关闭此窗口进入“恢复记录”，按原操作编号核对；不重新提交。"));
    }
    if (flow.pending) inspectHost.querySelectorAll("button").forEach(e => { if (e.textContent === "预览采用此完整分支" || e.dataset.syncRevalidationStart) e.disabled = true; });
    revalidation?.render(busy, !!flow.pending);
    if (busy) lock();
    remoteControls?.render();
  }
  const close = el("button", "关闭同步"); close.type = "button"; close.onclick = () => { clearReview(); remoteControls?.dispose(); dialog.close(); dialog.remove(); };
  dialog.oncancel = event => { if (busy) event.preventDefault(); else { clearReview(); remoteControls?.dispose(); dialog.remove(); } };
  rootField.input.oninput = () => { clearReview(); binding = null; status = null; historyHost.replaceChildren(); inspectHost.replaceChildren(); render(); };
  pair.oninput = () => { clearReview(); status = null; historyHost.replaceChildren(); inspectHost.replaceChildren(); render(); };
  dialog.append(el("h2", "资料库同步"), el("p", "每个副本有独立工作目录和私有状态。复制配对身份后，通过交换目录传递已保存版本；编辑器里的未保存草稿会保留。"), rootField.label, pairLabel, transport.label, info, controls, statusLine, pendingHost, objectsHost, historyHost, inspectHost, revalidationHost, original.label, button("读取原同步操作", async () => {
    await flow.recover(original.input.value.trim());
    const p = flow.pending!; rootField.input.value = p.request.root; pair.value = JSON.stringify(p.request.intent.identity); inspectHost.replaceChildren(); await load(); await refreshCommitted();
  }), close);
  const remoteHost = el("section"); dialog.insertBefore(remoteHost, original.label);
  remoteControls = mountRemoteSync(remoteHost, {
    identity, root: targetRoot, objects: () => picked().map(o => o.object), blocked: () => busy || !!flow.pending,
    received: async paired => {
      if (binding && !sameSyncIdentity(binding.identity, paired)) return;
      if (pair.value) { try { if (!sameSyncIdentity(identity(), paired)) return; } catch { return; } }
      clearReview();
      pair.value = JSON.stringify(paired); status = await api<SyncStatus>("/api/sync/status", { identity: paired }); selectedInitialized = false;
      if (binding) binding = await api<SyncBinding>("/api/sync/binding", { root: targetRoot() }); render();
    },
  });
  document.body.append(dialog); dialog.showModal(); render();
  if (rootField.input.value) await run(load);
}
