import { api, operationId } from "./api";
import { SyncFlow, objectKey, type SyncBinding, type SyncIdentity, type SyncObject, type SyncPending, type SyncStatus, type SyncVersion } from "./sync-state";
import "./sync-panel.css";

interface SyncHost { beforeAction(): Promise<void>; applied(root: string): Promise<void> }
const savedKey = "weftext-sync-operation-v1";
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] { const e = document.createElement(tag); e.textContent = text; return e; }
function field(label: string, value = ""): { label: HTMLLabelElement; input: HTMLInputElement } {
  const wrapper = el("label", label), input = el("input"); input.value = value; wrapper.append(input); return { label: wrapper, input };
}
export async function openSyncPanel(root: string, host: SyncHost): Promise<void> {
  const saved = localStorage.getItem(savedKey);
  const restored: SyncPending | null = saved ? JSON.parse(saved) : null;
  if (restored && (typeof restored.request?.operationId !== "string" || typeof restored.request?.root !== "string" || typeof restored.attempted !== "boolean" || typeof restored.status !== "string" || (restored.plan && (restored.plan.operationId !== restored.request.operationId || typeof restored.plan.planId !== "string")) || !["cloneSync", "applySync", "mergeSync"].includes(restored.request?.intent?.kind))) throw new Error("保存的同步操作记录无法读取；请保留浏览器记录并检查恢复记录。");
  const flow = new SyncFlow({ request: api, store: p => { if (p) localStorage.setItem(savedKey, JSON.stringify(p)); else localStorage.removeItem(savedKey); } }, restored);
  const dialog = el("dialog"); dialog.className = "sync-panel"; dialog.setAttribute("aria-label", "资料库同步");
  const rootField = field("本机资料库路径", restored?.request.root ?? root);
  const pairLabel = el("label", "配对身份（从原副本复制）"), pair = el("textarea"); pair.rows = 3; pairLabel.append(pair);
  if (restored) pair.value = JSON.stringify(restored.request.intent.identity);
  const transport = field("交换目录路径（新目录或已配对目录）");
  const original = field("原同步操作编号（地址变化或浏览器记录丢失时）");
  const statusLine = el("p"); statusLine.setAttribute("role", "status");
  const info = el("p"), objectsHost = el("section"), pendingHost = el("section"), controls = el("div"), inspectHost = el("section");
  let binding: SyncBinding | null = null, status: SyncStatus | null = null, busy = false;
  const selected = new Set<string>(); let selectedInitialized = false;
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
  function lock(): void { dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement>("input,button,textarea").forEach(e => { e.disabled = true; }); }
  async function load(): Promise<void> {
    binding = await api<SyncBinding | null>("/api/sync/binding", { root: targetRoot() });
    if (binding) pair.value = JSON.stringify(binding.identity);
    status = binding ? await api<SyncStatus>("/api/sync/status", { identity: binding.identity }) : null;
    selectedInitialized = false;
  }
  async function published(): Promise<void> {
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
    await host.beforeAction();
    await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind, identity: identity(), objects: picked().map(o => o.object) } });
  }
  async function inspectVersion(o: SyncObject, version: SyncVersion, common: boolean): Promise<void> {
    const title = el("h3", `${common ? "共同基础" : "版本"} ${version.revision}`);
    inspectHost.replaceChildren(title, el("pre", JSON.stringify({ parent: version.state.parent, name: version.state.name, retired: version.state.retired }, null, 2)));
    for (const [path, digest] of Object.entries(version.state.files)) {
      const section = el("details"); section.append(el("summary", path), el("code", digest));
      const content = el("pre"); section.append(content, button(`读取 ${path}`, async () => {
        const result = await api<{ text: string | null; bytes: number }>("/api/sync/file", { identity: identity(), object: o.object, revision: version.revision, path });
        content.textContent = result.text ?? `二进制文件或超过文本预览预算：${result.bytes} 字节。`;
      })); inspectHost.append(section);
    }
    if (!common && binding && o.view.versions.length > 1) {
      inspectHost.append(el("p", "采用此分支会选择它的完整正文、批注、资源和位置。双方原版本仍保留在历史中。"), button("预览采用此完整分支", async () => {
        await host.beforeAction();
        await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind: "mergeSync", identity: identity(), resolutions: [{ object: o.object, revisions: o.view.versions.map(v => v.revision), state: version.state }], blobs: {} } });
      }));
    }
  }
  function render(): void {
    dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement>("input,button,textarea").forEach(e => { e.disabled = false; });
    rootField.input.disabled = !!flow.pending; pair.readOnly = !!binding || !!flow.pending;
    info.textContent = binding ? `副本 ${binding.replicaId} · ${binding.partial ? "部分同步" : "完整副本"} · 已安装 ${binding.installedObjects.length} 个对象 · 待发布 ${binding.pendingPublications} 笔本地提交` : "此路径尚未启用同步。可以启用现有资料库，或接收历史后在尚不存在的路径创建独立副本。";
    controls.replaceChildren(button("读取本机状态", load), button("启用同步", async () => {
      const path = targetRoot(), key = `weftext-sync-enable:${path}`;
      const id = localStorage.getItem(key) ?? operationId(); localStorage.setItem(key, id);
      binding = await api<SyncBinding>("/api/sync/enable", { root: path, operationId: id }); pair.value = JSON.stringify(binding.identity);
      await published(); localStorage.removeItem(key);
    }), button("刷新已保存版本", published), button("接收交换目录", async () => {
      status = await api<SyncStatus>("/api/sync/receive", { identity: identity(), directory: transport.input.value }); selectedInitialized = false; inspectHost.replaceChildren();
    }), button("导出所选对象", async () => {
      await api("/api/sync/export", { root: targetRoot(), directory: transport.input.value, objects: picked().map(o => o.object) }); await published();
    }), button("预览应用所选对象", () => prepare("applySync")), button("预览创建独立副本", () => prepare("cloneSync")));
    const buttons = Array.from(controls.querySelectorAll("button"));
    buttons[1].disabled = !!binding || !!flow.pending; buttons[2].disabled = !binding || !!flow.pending;
    buttons[4].disabled = !binding || !!flow.pending; buttons[5].disabled = !binding || !status || !!flow.pending; buttons[6].disabled = !!binding || !status || !!flow.pending;
    buttons[0].disabled = !!flow.pending; buttons[3].disabled = !!flow.pending;
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
        for (const v of o.view.versions) row.append(button(`查看版本 ${v.revision}`, () => inspectVersion(o, v, false)));
        for (const v of o.view.commonBases) row.append(button(`查看共同基础 ${v.revision}`, () => inspectVersion(o, v, true)));
        objectsHost.append(row);
      }
    }
    pendingHost.replaceChildren();
    const p = flow.pending;
    if (p) {
      pendingHost.append(el("h3", `原同步操作：${p.status}`), el("code", p.request.operationId), el("p", `目标：${p.request.root} · ${p.request.intent.kind}`));
      if (p.plan) { const list = el("ul"); for (const c of p.plan.changes) list.append(el("li", `${c.action} · ${c.path} · ${c.bytes} 字节`)); pendingHost.append(list); for (const loss of p.plan.losses ?? []) pendingHost.append(el("p", loss.message)); }
      pendingHost.append(button("检查原操作结果", async () => { await flow.inspect(); await refreshCommitted(); }));
      if (!p.attempted && !p.plan) pendingHost.append(button("重试原准备请求", () => flow.retryPrepare()));
      if (p.status === "prepared") pendingHost.append(button("确认应用此预览", async () => { await host.beforeAction(); await flow.commit(); await refreshCommitted(); }));
      if (["committed", "resolved"].includes(p.status) || !p.attempted) pendingHost.append(button(["committed", "resolved"].includes(p.status) ? "结束已确认操作" : "取消本次预览", () => { flow.forget(); inspectHost.replaceChildren(); }));
      if (p.status === "prepared" && p.attempted) pendingHost.append(button("取消尚未开始写入的原提案", async () => { await flow.cancelPrepared(); await refreshCommitted(); }));
      if (["unknown", "recoveryPending"].includes(p.status)) pendingHost.append(el("p", "如需处理已发生的部分步骤，请关闭此窗口进入“恢复记录”，按原操作编号核对；不重新提交。"));
    }
    if (flow.pending) inspectHost.querySelectorAll("button").forEach(e => { if (e.textContent === "预览采用此完整分支") e.disabled = true; });
    if (busy) lock();
  }
  const close = el("button", "关闭同步"); close.type = "button"; close.onclick = () => { dialog.close(); dialog.remove(); };
  dialog.oncancel = event => { if (busy) event.preventDefault(); else dialog.remove(); };
  rootField.input.oninput = () => { binding = null; status = null; inspectHost.replaceChildren(); render(); };
  pair.oninput = () => { status = null; inspectHost.replaceChildren(); render(); };
  dialog.append(el("h2", "资料库同步"), el("p", "每个副本有独立工作目录和私有状态。复制配对身份后，通过交换目录传递已保存版本；编辑器里的未保存草稿会保留。"), rootField.label, pairLabel, transport.label, info, controls, statusLine, pendingHost, objectsHost, inspectHost, original.label, button("读取原同步操作", async () => {
    await flow.recover(original.input.value.trim());
    const p = flow.pending!; rootField.input.value = p.request.root; pair.value = JSON.stringify(p.request.intent.identity); inspectHost.replaceChildren(); await load(); await refreshCommitted();
  }), close);
  document.body.append(dialog); dialog.showModal(); render();
  if (rootField.input.value) await run(load);
}
