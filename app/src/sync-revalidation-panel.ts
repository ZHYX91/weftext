import { objectKey, type ObjectKey, type SyncIdentity, type SyncObject, type SyncRequest, type SyncRevalidation, type SyncRevalidationCut, type SyncRevalidationReview } from "./sync-state";
import { SyncRevalidationSession, type SyncExactText } from "./sync-revalidation";

function el<K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] { const node = document.createElement(tag); node.textContent = text; return node; }
const kindNames: Record<ObjectKey["kind"], string> = { node: "笔记", directory: "目录", rootDirectory: "资料库书签", schema: "属性定义", settings: "库设置", rules: "库规则", tags: "标签" };
function nameOf(object: ObjectKey, objects: SyncObject[], resolutions: SyncRevalidation[] = []): string {
  if (!["node", "directory", "rootDirectory"].includes(object.kind)) return kindNames[object.kind];
  return resolutions.find(r => objectKey(r.object) === objectKey(object))?.state.name || objects.find(o => objectKey(o.object) === objectKey(object))?.view.versions.map(v => v.state.name).filter((v, i, all) => all.indexOf(v) === i).join(" / ") || kindNames[object.kind];
}
function short(revision: string, candidates: string[] = []): string { let length = 12; while (length < revision.length && candidates.some(other => other !== revision && other.slice(0, length) === revision.slice(0, length))) length++; return revision.length > length ? `${revision.slice(0, length)}…` : revision; }
function decisionText(item: SyncRevalidation, objects: SyncObject[]): string {
  const revision = item.decision.kind === "preserve" ? item.decision.revision : item.decision.baseRevision;
  if (item.decision.kind === "preserve") return `保留原版本 ${short(revision, item.revisions)}`;
  const original = objects.find(o => objectKey(o.object) === objectKey(item.object))?.view.versions.find(v => v.revision === revision)?.state;
  const changed = original ? Object.keys(item.state.files).filter(path => original.files[path] !== item.state.files[path]) : [];
  return `采用修复内容${changed.length ? `：${changed.join("、")}` : ""}（原版本 ${short(revision, item.revisions)}）`;
}
export function appendSyncRevalidationSummary(container: HTMLElement, review: Pick<SyncRevalidationReview, "expectedInstalled" | "reviewedVersions" | "resolutions">, objects: SyncObject[]): void {
  const candidates = [...review.expectedInstalled, ...review.reviewedVersions].flatMap(dep => dep.revisions), versionText = (revision: string): string => short(revision, candidates);
  const table = el("table"), header = el("tr");
  for (const text of ["对象", "本次决定", "本机原版本", "已审阅版本"]) header.append(el("th", text)); table.append(header);
  for (const received of review.reviewedVersions) {
    const row = el("tr"), resolution = review.resolutions.find(r => objectKey(r.object) === objectKey(received.object)), installed = review.expectedInstalled.find(v => objectKey(v.object) === objectKey(received.object));
    row.append(el("td", nameOf(received.object, objects, review.resolutions)), el("td", resolution ? decisionText(resolution, objects) : "随同一事务或依赖核对"), el("td", installed ? installed.revisions.map(versionText).join("、") : "未安装"), el("td", received.revisions.map(versionText).join("、"))); table.append(row);
  }
  container.append(table);
  for (const item of review.resolutions) container.append(el("p", `${nameOf(item.object, objects, review.resolutions)}的实际语义依赖：${item.reviewedDependencies.length ? item.reviewedDependencies.map(dep => `${nameOf(dep.object, objects, review.resolutions)}（${dep.revisions.map(versionText).join("、")}）`).join("；") : "没有其他对象"}。`));
  const technical = el("details"); technical.append(el("summary", "技术详情：完整版本与准确请求"), el("pre", JSON.stringify({ expectedInstalled: review.expectedInstalled, reviewedVersions: review.reviewedVersions, resolutions: review.resolutions }, null, 2))); container.append(technical);
}
interface Host {
  request<T>(path: string, body: unknown): Promise<T>;
  run(action: () => Promise<void>): Promise<void>;
  prepare(request: SyncRequest): Promise<void>;
  beforeAction(): Promise<void>;
  id(): string;
}
export function mountSyncRevalidation(container: HTMLElement, root: string, identity: SyncIdentity, cut: SyncRevalidationCut, initial: { object: ObjectKey; revision: string }[], host: Host): { render(busy: boolean, pending: boolean): void; dispose(): void } {
  const session = new SyncRevalidationSession(root, identity, cut, host.id), summary = el("section"), choices = el("section"), actions = el("div"), pendingEdits = new Set<HTMLTextAreaElement>();
  let disposed = false, blocked = false;
  for (const selected of initial) session.select(selected.object, selected.revision);
  container.dataset.syncRevalidation = "review";
  container.append(el("h3", "重新验证所选同步版本"), el("p", "明确保留某个原分支，或修复它的准确文件。集中 Schema、原生绑定和查询基线不会自动改成最新值。普通暂时无法定位的链接仍可保留。编辑器草稿与撤销记录保留。"),
    el("p", "下面是已收到的候选版本。只有明确选择的对象会生成重新验证决定；单分支依赖由 Core 按实际读取闭包选取。并发依赖需要明确选择一个完整分支并一并重新验证。"), choices, actions, summary);
  if (cut.coverage !== "complete") container.insertBefore(el("p", "候选历史中存在缺件；实际选择是否完整，以只读审阅结果为准。"), choices);
  function button(text: string, action: () => Promise<void>): HTMLButtonElement { const b = el("button", text); b.type = "button"; b.onclick = () => { if (!blocked && !disposed) void host.run(action); }; return b; }
  function clearSummary(): void { summary.replaceChildren(); delete summary.dataset.syncRevalidationSummary; confirm.disabled = true; read.disabled = blocked || pendingEdits.size > 0; }
  const read = button("读取实际语义依赖", async () => {
    if (pendingEdits.size) throw new Error("请先采用或放弃尚未确认的文件修复内容。");
    clearSummary(); read.disabled = true;
    const accepted = await session.inspect(input => host.request<SyncRevalidationReview>("/api/sync/revalidation-review", input));
    if (disposed || !accepted) throw new Error("审阅期间选择或收到的版本已变化，请重新审阅。");
    renderSummary();
  }); read.dataset.syncRevalidationAction = "review";
  const confirm = button("确认依赖并预览重新验证", async () => { await host.beforeAction(); await host.prepare(session.request()); }); confirm.dataset.syncRevalidationAction = "prepare";
  actions.append(read, confirm);
  for (const item of cut.objects) {
    const key = objectKey(item.object), row = el("details"), selected = session.selection(item.object), label = el("label", "本次决定"), select = el("select"), files = el("section"), decision = el("p");
    row.dataset.syncRevalidationObject = key; row.open = !!selected;
    row.append(el("summary", `${nameOf(item.object, cut.objects)} · ${kindNames[item.object.kind]}`),
      el("p", `当前收到 ${item.view.versions.length} 个分支；本次审阅会核对完整分支集合。`));
    const none = el("option", "仅作为依赖候选，不重新验证此对象"); none.value = ""; select.append(none);
    for (const version of item.view.versions) { const option = el("option", `保留完整版本 ${short(version.revision, item.view.versions.map(v => v.revision))}`); option.value = version.revision; option.title = version.revision; select.append(option); }
    select.value = selected?.decision.kind === "preserve" ? selected.decision.revision : ""; select.dataset.syncRevalidationChoice = key; label.append(select); row.append(label, decision, files);
    const original = el("details"); original.append(el("summary", "技术详情：原提交依赖与未决项"), el("pre", JSON.stringify({ object: item.object, revisions: item.view.versions.map(v => v.revision), originalDependencies: item.originalDependencies, unsettledDependencies: item.unsettledDependencies }, null, 2))); row.append(original);
    select.onchange = () => {
      for (const input of files.querySelectorAll("textarea")) pendingEdits.delete(input);
      if (select.value) session.select(item.object, select.value); else session.unselect(item.object);
      clearSummary(); renderFiles();
    };
    function renderFiles(): void {
      files.replaceChildren();
      const chosen = session.selection(item.object); decision.textContent = chosen ? `已选择：${decisionText(chosen, cut.objects)}。` : "未选择，不为此候选生成新的重新验证决定。"; if (!chosen) return;
      files.append(el("p", "保持原分支会保留准确字节、位置、资源和批注。若要修复，请读取原文件，编辑后明确采用该文件内容。"));
      for (const path of Object.keys(chosen.state.files)) {
        const section = el("details"), feedback = el("p"), content = el("section"); section.append(el("summary", path));
        const load = button(`读取原文件以检查或修复 ${path}`, async () => {
          const earlier = content.querySelector("textarea"); if (earlier && pendingEdits.has(earlier)) throw new Error("请先采用或放弃此文件未确认的修复输入。");
          const revision = chosen.decision.kind === "preserve" ? chosen.decision.revision : chosen.decision.baseRevision;
          const exact = await host.request<SyncExactText>("/api/sync/file", { identity, object: item.object, revision, path });
          if (disposed) return;
          const source = await session.verifyText(item.object, exact);
          if (disposed || select.value !== revision || !container.contains(section)) throw new Error("文件读取依据已变化，请重新读取。");
          content.replaceChildren(); const textLabel = el("label", `修复内容：${path}`), text = el("textarea"); text.rows = 10; text.spellcheck = false; const appliedSource = session.repairedText(item.object, path) ?? source; text.value = appliedSource; text.dataset.syncRevalidationRepair = path; textLabel.append(text);
          const apply = button("采用此文件的修复内容", async () => {
            await session.repairText(item.object, path, text.value); pendingEdits.delete(text); apply.disabled = true;
            decision.textContent = `已选择：${decisionText(session.selection(item.object)!, cut.objects)}。`;
            feedback.textContent = session.selection(item.object)?.decision.kind === "repair" ? "已采用明确修复内容；请重新读取实际语义依赖。" : "内容与原文件相同，保留原文件修订。"; clearSummary();
          }); apply.dataset.syncRevalidationAction = "repair"; apply.disabled = true;
          text.oninput = () => { session.touch(); pendingEdits.add(text); feedback.textContent = "修复尚未采用；原文件和编辑器草稿保留。"; clearSummary(); apply.disabled = blocked; };
          const discard = button("放弃此文件未采用的输入", async () => { text.value = session.repairedText(item.object, path) ?? source; pendingEdits.delete(text); apply.disabled = true; session.touch(); clearSummary(); });
          content.append(textLabel, apply, discard);
        });
        section.append(load, feedback, content); files.append(section);
      }
    }
    renderFiles(); choices.append(row);
  }
  function renderSummary(): void {
    const review = session.review; summary.replaceChildren(); if (!review) return;
    summary.dataset.syncRevalidationSummary = "ready";
    summary.append(el("h4", "实际审阅依据"), el("p", "请核对对象、保留或修复决定及实际依赖，再预览同步计划。当前文件与编辑器草稿尚未改变。"));
    appendSyncRevalidationSummary(summary, review, cut.objects);
    for (const target of review.annotationTargets ?? []) summary.append(el("p", `批注 ${target.annotationId}：${target.state === "exact" ? "目标已核对" : "保留原讨论，目标不可用／待重接"}。`));
    confirm.disabled = blocked || pendingEdits.size > 0;
  }
  function render(busy: boolean, pending: boolean): void {
    blocked = busy || pending || disposed;
    container.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = blocked; });
    read.disabled = blocked || pendingEdits.size > 0; confirm.disabled = blocked || pendingEdits.size > 0 || !session.review;
    // File repair controls remain explicit, even after the outer sync panel
    // unlocks all controls following a read-only request.
    for (const section of container.querySelectorAll<HTMLElement>("section")) {
      const text = section.querySelector<HTMLTextAreaElement>(":scope > label > textarea"), apply = section.querySelector<HTMLButtonElement>(":scope > button[data-sync-revalidation-action='repair']");
      if (text && apply) apply.disabled = blocked || !pendingEdits.has(text);
    }
  }
  render(false, false);
  return { render, dispose: () => { disposed = true; session.invalidate(); } };
}
