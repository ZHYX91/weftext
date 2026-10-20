import type { Inventory, InventoryEntry } from "./types";
import { nodeSourceHash } from "./hosted-navigation-state";
import { mountListThumbnail, type ListPreview } from "./list-thumbnail";
import { decodeContentsPreference, decodePendingOrder, defaultContentsPreference, insertedSibling, movedSibling, type AuthoredOrder, type ContentsPreference, type OrderPlan, type PendingOrder } from "./node-contents-state";

type Request = <T>(path: string, body?: unknown) => Promise<T>;
interface ContentsHost {
  request: Request;
  readBlob(path: string, body: unknown): Promise<Blob>;
  open(entry: InventoryEntry): void | Promise<void>;
  browse(path: string | null): void;
  menu(entries: InventoryEntry[], folder?: string): void;
  preview(entries: InventoryEntry[]): void | Promise<void>;
  changed(): Promise<void>;
  notify(message: string): void;
}
const normalized = (path: string) => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
const leaf = (path: string) => path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ?? path;
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text = "", className = ""): HTMLElementTagNameMap[K] { const item = document.createElement(tag); item.textContent = text; item.className = className; return item; }

/** Navigation rows are projections. Only the explicit original Core plan writes
 * shared sibling order; list settings and previews never save author content. */
export function mountNodeContents(panel: HTMLElement, inventory: Inventory, folder: string, query: string, selections: Set<string>, host: ContentsHost): { dispose(): void } {
  let disposed = false, busy = false, order: AuthoredOrder | undefined, pending: PendingOrder | undefined, parentRef: string | undefined;
  let preference = defaultContentsPreference(), persistenceError = "", dragging: string | undefined, preferenceEdited = false, recoveryBlocked = false;
  const dialogs = new Set<HTMLDialogElement>();
  let thumbnails: { dispose(): void }[] = [];
  const header = el("header"), rows = el("div"), controls = el("div", "", "node-contents-controls"), notice = el("p"), orderHost = el("div", "", "node-order-controls");
  const direct = inventory.entries.filter(entry => normalized(entry.parentPath) === normalized(folder) && normalized(entry.path) !== normalized(folder));
  const visible = direct.filter(entry => !query || `${entry.title ?? ""} ${entry.name} ${entry.relativePath}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const button = (label: string, run: () => void | Promise<void>) => { const control = el("button", label); control.type = "button"; control.onclick = () => { void Promise.resolve().then(run).catch(error => host.notify(error instanceof Error ? error.message : String(error))); }; return control; };
  const key = () => `weftext-node-contents:${encodeURIComponent(parentRef!)}`;
  const pendingKey = () => `${key()}:operation`;
  const retainPreference = () => { preferenceEdited = true; if (!parentRef) return; try { localStorage.setItem(key(), JSON.stringify(preference)); } catch { persistenceError = "本机偏好暂不能持久保存，当前显示仍可使用。"; } };
  const retainPending = () => { if (!parentRef) throw new Error("缺少准确父级，未发送作者写操作"); if (pending) localStorage.setItem(pendingKey(), JSON.stringify(pending)); else localStorage.removeItem(pendingKey()); };
  const member = (entry: InventoryEntry) => order?.members.find(item => normalized(item.path) === normalized(entry.path));
  const stopThumbnails = () => { thumbnails.forEach(item => item.dispose()); thumbnails = []; };
  const runBusy = async (work: () => Promise<void>) => { if (busy || disposed) return; busy = true; renderOrder(); try { await work(); } finally { busy = false; if (!disposed) renderOrder(); } };

  header.append(button(leaf(inventory.root), () => host.browse(null)));
  const relative = folder.slice(inventory.root.length).replaceAll("\\", "/").split("/").filter(Boolean); let ancestor = inventory.root;
  for (const name of relative) { ancestor += "/" + name; const target = ancestor; header.append(el("span", "›"), button(name, () => host.browse(target))); }
  const sorting = el("select"); sorting.setAttribute("aria-label", "节点内容个人排序");
  for (const [value, text] of [["name", "名称（个人）"], ["path", "路径（个人）"], ["authored", "作者手动顺序"]]) { const option = el("option", text); option.value = value; sorting.append(option); }
  sorting.onchange = () => { preference.sort = sorting.value as ContentsPreference["sort"]; retainPreference(); renderRows(); renderOrder(); };
  controls.append(sorting);
  for (const [property, label] of [["thumbnails", "节点内容缩略图"], ["summaries", "节点内容正文摘要"]] as const) {
    const wrapper = el("label", label), check = el("input"); check.type = "checkbox"; check.setAttribute("aria-label", label);
    check.onchange = () => { preference[property] = check.checked; retainPreference(); renderRows(); }; wrapper.prepend(check); controls.append(wrapper);
  }
  const density = el("select"); density.setAttribute("aria-label", "节点内容密度");
  for (const [value, label] of [["compact", "紧凑"], ["comfortable", "详细"]]) { const option = el("option", label); option.value = value; density.append(option); }
  density.onchange = () => { preference.density = density.value as ContentsPreference["density"]; retainPreference(); renderRows(); }; controls.append(density);
  controls.append(button("恢复节点列表默认", () => { preference = defaultContentsPreference(); retainPreference(); renderRows(); renderOrder(); }));
  const selectedAction = button("所选项目操作", () => host.menu(visible.filter(entry => selections.has(entry.path))));
  panel.replaceChildren(header, controls, notice, selectedAction, orderHost, rows);

  function updateSelection() {
    rows.querySelectorAll<HTMLElement>("[data-entry-path]").forEach(row => { const selected = selections.has(row.dataset.entryPath!); row.setAttribute("aria-selected", String(selected)); row.querySelector<HTMLInputElement>("input")!.checked = selected; });
    selectedAction.disabled = !visible.some(entry => selections.has(entry.path));
  }
  function renderRows() {
    if (disposed) return; stopThumbnails();
    const active = document.activeElement as HTMLElement | null, focusedRow = active?.closest<HTMLElement>("[data-entry-path]"), focusedPath = focusedRow?.dataset.entryPath;
    const focusedLabel = focusedRow && panel.contains(focusedRow) ? active?.getAttribute("aria-label") ?? active?.textContent : undefined;
    const scroll = panel.scrollTop; rows.replaceChildren(); sorting.value = preference.sort; density.value = preference.density;
    controls.querySelectorAll<HTMLInputElement>("input[type=checkbox]").forEach(check => { check.checked = preference[check.getAttribute("aria-label") === "节点内容缩略图" ? "thumbnails" : "summaries"]; });
    panel.dataset.density = preference.density;
    const rank = new Map(order?.orderedRefs.map((ref, index) => [ref, index]) ?? []);
    const entries = [...visible].sort((a, b) => {
      if (preference.sort === "authored") { const difference = (rank.get(member(a)?.targetRef ?? "") ?? Number.MAX_SAFE_INTEGER) - (rank.get(member(b)?.targetRef ?? "") ?? Number.MAX_SAFE_INTEGER); if (difference) return difference; }
      return (preference.sort === "path" ? a.relativePath : a.title || a.name).localeCompare(preference.sort === "path" ? b.relativePath : b.title || b.name, "zh-CN");
    });
    for (const entry of entries) {
      const row = el("div", "", "node-content-row"); row.tabIndex = 0; row.dataset.entryPath = entry.path;
      const check = el("input"); check.type = "checkbox"; check.setAttribute("aria-label", `选择 ${entry.name}`); check.onchange = () => { check.checked ? selections.add(entry.path) : selections.delete(entry.path); updateSelection(); };
      const open = () => { if (entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ")) { host.notify("此文件不支持笔记正文入口；请在所属笔记的获权资源查看器中打开资源。"); return; } return host.open(entry); }, title = button(`${entry.title || entry.name} · ${entry.identityStatus === "admitted" ? "笔记" : entry.kind === "directory" ? "目录" : entry.identityStatus}`, () => { selections.clear(); selections.add(entry.path); updateSelection(); }); title.ondblclick = () => { void Promise.resolve(open()).catch(error => host.notify(String(error))); };
      const menu = () => { if (!selections.has(entry.path)) { selections.clear(); selections.add(entry.path); updateSelection(); } host.menu(visible.filter(item => selections.has(item.path)), entry.kind === "directory" ? entry.path : undefined); };
      row.oncontextmenu = event => { event.preventDefault(); menu(); };
      row.onkeydown = event => { if (event.isComposing) return; if (event.key === "Enter" && event.target === row) { event.preventDefault(); void Promise.resolve(open()).catch(error => host.notify(String(error))); } if (event.key === "ContextMenu" || event.shiftKey && event.key === "F10") { event.preventDefault(); menu(); } };
      const openButton = button("打开", open); openButton.disabled = entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ"); row.append(check, title, openButton, button("预览", () => host.preview([entry])));
      const source = entry.kind === "directory" ? inventory.entries.find(item => item.kind === "node" && normalized(item.parentPath) === normalized(entry.path)) : entry;
      if (preference.summaries) row.append(el("p", source?.excerpt ?? "当前获权扫描没有正文摘要。", "node-content-summary"));
      if (preference.thumbnails) {
        const thumbnail = el("div", "", "node-content-thumbnail"); row.append(thumbnail);
        if (source?.nodeId && source.identityStatus === "admitted" && source.source !== undefined && inventory.workspaceId) {
          const entityKey = `weftext-node://${inventory.workspaceId}/${source.nodeId}`, rowSource = source.source;
          thumbnails.push(mountListThumbnail(thumbnail, { viewport: panel,
            resolve: async () => host.request<ListPreview>("/api/navigation/preview", { root: inventory.root, entityKey, sourceHash: await nodeSourceHash(rowSource) }),
            read: (descriptor, revision) => host.readBlob("/api/query/preview-resource", { root: inventory.root, entityKey, revision, resource: descriptor.resource, resourceRevision: descriptor.revision }) }));
        } else { thumbnail.dataset.previewState = source?.identityStatus === "admitted" ? "unavailable" : "unsupported"; thumbnail.textContent = source?.identityStatus === "admitted" ? "缩略图不可用：当前扫描缺少准确笔记依据。" : "此项目没有可独立验证的获权笔记资源缩略图。"; }
      }
      const target = member(entry)?.targetRef;
      if (preference.sort === "authored" && order?.complete && target && !pending && !recoveryBlocked) {
        const actions = el("div", "", "node-sibling-actions");
        for (const [offset, label] of [[-1, "上移"], [1, "下移"]] as const) { const move = button(label, () => propose(movedSibling(order!.orderedRefs, target, offset))); move.setAttribute("aria-label", `${entry.name} 作者顺序${label}`); const index = order.orderedRefs.indexOf(target); move.disabled = busy || offset === -1 && index === 0 || offset === 1 && index === order.orderedRefs.length - 1; actions.append(move); } row.append(actions);
        row.draggable = true; row.ondragstart = event => { dragging = target; event.dataTransfer?.setData("text/plain", target); };
        row.ondragover = event => { if (dragging && dragging !== target) { event.preventDefault(); row.dataset.insert = event.clientY > row.getBoundingClientRect().top + row.offsetHeight / 2 ? "after" : "before"; } };
        row.ondragleave = () => { delete row.dataset.insert; }; row.ondragend = () => { dragging = undefined; rows.querySelectorAll<HTMLElement>("[data-insert]").forEach(item => { delete item.dataset.insert; }); };
        row.ondrop = event => { event.preventDefault(); const sourceRef = dragging, after = row.dataset.insert === "after"; delete row.dataset.insert; dragging = undefined; if (sourceRef) propose(insertedSibling(order!.orderedRefs, sourceRef, target, after)); };
      }
      rows.append(row);
    }
    if (!entries.length) rows.append(el("p", query ? "当前筛选没有匹配项目。" : "此节点暂无内容。"));
    updateSelection(); panel.scrollTop = scroll;
    if (focusedPath && focusedLabel !== undefined) { const replacement = [...rows.querySelectorAll<HTMLElement>("[data-entry-path]")].find(item => item.dataset.entryPath === focusedPath); const control = [...replacement?.querySelectorAll<HTMLElement>("button,input") ?? []].find(item => (item.getAttribute("aria-label") ?? item.textContent) === focusedLabel); control?.focus({ preventScroll: true }); }
  }
  function renderOrder() {
    if (disposed) return; orderHost.replaceChildren();
    notice.textContent = `${preference.sort === "authored" ? "作者顺序按共享设置显示；新成员按名称追加。只有明确准备并确认才写入。" : "个人列表排序；拖动不会改写作者顺序。"}${persistenceError}`;
    if (!parentRef) { if (preference.sort === "authored") orderHost.append(el("p", "此目录尚无获权稳定导航身份；请先在导航入口明确登记。个人列表仍可使用。")); return; }
    if (order && !order.complete) orderHost.append(el("p", "直接子级身份或扫描尚不完整，作者重排不可用；先核对未登记、重复或不可用目录。"));
    if (pending) {
      orderHost.append(el("p", `原作者顺序操作 ${pending.request.operationId}：${pending.status}`));
      const inspect = button("检查原作者顺序操作", () => runBusy(async () => { receive(await host.request<OrderPlan>("/api/operations/status", { operationId: pending!.request.operationId })); })); inspect.disabled = busy; orderHost.append(inspect);
      if (!pending.plan && !pending.attempted) { const retry = button("重试原顺序准备", () => runBusy(prepare)); retry.disabled = busy; orderHost.append(retry); }
      if (pending.plan && pending.status === "prepared") {
        const confirm = button("查看原顺序变更并确认", () => showProposal(pending!.request.children, true)); confirm.disabled = busy; orderHost.append(confirm);
        const cancel = button("取消原作者顺序计划", () => runBusy(async () => { receive(await host.request<OrderPlan>("/api/navigation/order/cancel", { root: inventory.root, operationId: pending!.request.operationId, planId: pending!.plan!.planId })); })); cancel.disabled = busy; orderHost.append(cancel);
      }
      if (["committed", "resolved", "cancelled"].includes(pending.status)) orderHost.append(button("结束已确认顺序操作", () => { pending = undefined; retainPending(); renderOrder(); renderRows(); }));
    }
  }
  function receive(plan: OrderPlan) {
    if (!pending || plan.operationId !== pending.request.operationId || pending.plan && plan.planId && pending.plan.planId !== plan.planId) throw new Error("顺序操作回执与原计划不一致，原输入已保留");
    if (plan.planId) pending.plan = plan; pending.status = plan.status; retainPending();
    if (plan.status === "committed") { host.notify("作者同级顺序已提交；笔记正文与个人草稿未保存。"); void host.changed().catch(error => host.notify(String(error))); }
  }
  async function prepare() {
    const selected = pending!; receive(await host.request<OrderPlan>("/api/navigation/order/prepare", structuredClone(selected.request)));
  }
  function propose(refs: string[]) { if (busy || pending || recoveryBlocked || !order?.complete || refs.join("\n") === order.orderedRefs.join("\n")) return; showProposal(refs, false); }
  function showProposal(refs: string[], prepared: boolean) {
    const selectedOrder = order, original = pending; if (!selectedOrder) return;
    const trigger = document.activeElement as HTMLElement | null, dialog = el("dialog", "", "everyday-dialog"); dialog.setAttribute("aria-label", "作者同级顺序变更");
    const originalOrder = prepared ? pending!.beforeRefs : selectedOrder.orderedRefs, names = (items: string[]) => items.map(ref => `${selectedOrder.members.find(item => item.targetRef === ref)?.name ?? "原稳定对象（当前位置不可用）"}\n${ref}`).join("\n\n");
    dialog.append(el("h2", "作者同级顺序变更"), el("p", `父级：${selectedOrder.parentRef}`), el("p", "只更新同一库设置中的作者导航顺序；不改目录名、不移动节点，也不保存正文草稿。普通文件不参与作者子级排序；以下含当前个人筛选之外的直接子级。"), el("h3", "原读取顺序"), el("pre", names(originalOrder)), el("h3", "拟应用顺序"), el("pre", names(refs)));
    if (prepared) dialog.append(el("pre", pending?.plan?.changes?.map(change => `${change.action} ${change.path} · ${change.bytes} 字节`).join("\n") ?? "请核对原计划。"));
    const close = () => { dialog.close(); dialog.remove(); dialogs.delete(dialog); if (trigger?.isConnected) trigger.focus({ preventScroll: true }); };
    dialog.append(button(prepared ? "保留原计划，暂不提交" : "取消", close));
    const apply = button(prepared ? "确认提交原作者顺序" : "准备此顺序变更", async () => {
      apply.disabled = true;
      try {
        if (prepared) { if (pending !== original || pending?.status !== "prepared" || !pending.plan) throw new Error("原操作已变化，未提交"); await runBusy(async () => { pending!.attempted = true; pending!.status = "unknown"; retainPending(); receive(await host.request<OrderPlan>("/api/operations/commit", { operationId: pending!.request.operationId, planId: pending!.plan!.planId })); }); }
        else { if (pending || order !== selectedOrder || recoveryBlocked) throw new Error("父级依据或原恢复记录已变化，请重新选择"); pending = { request: { root: inventory.root, parentRef: selectedOrder.parentRef, children: refs, baseId: selectedOrder.baseId, operationId: crypto.randomUUID() }, beforeRefs:[...selectedOrder.orderedRefs], status: "preparing" }; try { retainPending(); } catch (error) { pending = undefined; throw error; } await runBusy(prepare); }
        close(); renderOrder(); renderRows();
      } catch (error) { apply.disabled = false; host.notify(error instanceof Error ? error.message : String(error)); renderOrder(); }
    }); dialog.append(apply); dialog.oncancel = event => { event.preventDefault(); close(); }; document.body.append(dialog); dialogs.add(dialog); dialog.showModal();
  }
  async function refreshOrder() { if (!parentRef) return; const read = await host.request<AuthoredOrder>("/api/navigation/order/read", { root: inventory.root, parentRef }); if (disposed) return; if (read.parentRef !== parentRef || !read.baseId || !Array.isArray(read.members) || !Array.isArray(read.orderedRefs)) throw new Error("作者顺序不对应原父级，未替换列表"); order = read; renderRows(); renderOrder(); }
  renderRows(); renderOrder();
  void (async () => {
    try {
      const target = await host.request<{ targetRef?: string; status: string }>("/api/navigation/capture", { root: inventory.root, path: folder }); if (disposed) return;
      parentRef = target.targetRef; if (!parentRef || target.status !== "managed") { parentRef = undefined; renderOrder(); return; }
      try { const raw = localStorage.getItem(key()); if (!preferenceEdited && raw && raw.length <= 4096) preference = decodeContentsPreference(JSON.parse(raw)); else if (preferenceEdited) retainPreference(); }
      catch { persistenceError = "本机列表偏好无法读取，使用当前显示设置。"; }
      try { const original = localStorage.getItem(pendingKey()); if (original) pending = decodePendingOrder(original, inventory.root, parentRef); }
      catch (error) { recoveryBlocked = true; persistenceError = `${error instanceof Error ? error.message : String(error)}；原记录保留，作者重排暂停。`; }
      await refreshOrder();
    } catch (error) { if (!disposed) { persistenceError = error instanceof Error ? error.message : String(error); renderOrder(); } }
  })();
  return { dispose() { disposed = true; stopThumbnails(); for (const dialog of dialogs) { dialog.close(); dialog.remove(); } dialogs.clear(); } };
}
