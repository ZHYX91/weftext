import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { registryIntent, registryReceipt, type RegistryCatalog, type RegistryDraft, type RegistryReceipt, type RegistryRequest, type RegistryResult } from "./hosted-navigation-registry-state";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
type Job = { request: RegistryRequest; receipt?: RegistryReceipt; commit?: { operationId: string; planId: string }; phase?: "prepare" | "commit" | "settle" };
type Cache = { draft: RegistryDraft; job?: Job };
const initial = (): Cache => ({ draft: { kind: "admitDirectory", slot: "", reference: "", displayName: "", parentRef: null } });
const ended = (status?: string) => ["committed", "resolved", "cancelled"].includes(status ?? "");
export interface HostedRegistryHost { authorInputs?: AuthorInputFactory; identity(): HostedLinksIdentity | null; authorizationChanged(message: string, current: { actor: string; deployment: string; generation: number }): void; registryChanged?(): void }
function selection(label: string, values: [string, string][], value: string) { const select = ui("select", "data-input"); select.setAttribute("aria-label", label); if (value && !values.some(([id]) => id === value)) values.push([value, "原选择（当前不可用）"]); for (const [id, text] of values) { const option = ui("option", "", text); option.value = id; select.append(option); } select.value = value; return select; }
/** Directory navigation is a capability of its own, never a Query scan. */
export class HostedNavigationRegistry {
  private epoch = 0;
  private inputScopes: AuthorInputScopes;
  private key = "";
  private cache = initial();
  private catalog?: RegistryCatalog;
  private catalogTree?: TagTree;
  private busy = false;
  private verified = false;
  private unreadable?: string;
  private catalogSerial = 0;
  private resolveSerial = 0;
  private message = ui("p", "hint");
  private editor = ui("section");
  private resolved = ui("section");
  constructor(private host: HTMLElement, private request: Request, private owner: HostedRegistryHost) { this.inputScopes = new AuthorInputScopes(owner.authorInputs); this.clear(false); }
  clear(removeCache = true): void {
    this.inputScopes.clear(); this.catalogSerial++; this.resolveSerial++;
    this.epoch++; this.key = ""; this.cache = initial(); this.catalog = undefined; this.catalogTree = undefined; this.busy = false; this.verified = false; this.unreadable = undefined; this.message.textContent = ""; this.editor.replaceChildren(); this.resolved.replaceChildren(); this.host.replaceChildren(); this.host.hidden = true;
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-navigation-registry:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number): Promise<void> {
    this.clear(false); const epoch = this.epoch; this.key = hostedCachePrefix("navigation-registry", deployment, actor, generation) + "state";
    const identity = this.owner.identity(); if (!identity || identity.deployment !== deployment || identity.actor !== actor || identity.generation !== generation) { this.clear(false); return; }
    try { const raw = sessionStorage.getItem(this.key); if (raw) { const parsed = JSON.parse(raw) as Cache; if (!parsed.draft || !["admitDirectory", "createTag", "renameTag", "moveTag"].includes(parsed.draft.kind) || [parsed.draft.slot, parsed.draft.reference, parsed.draft.displayName].some(v => typeof v !== "string") || parsed.draft.parentRef !== undefined && parsed.draft.parentRef !== null && typeof parsed.draft.parentRef !== "string") throw new Error(); parsed.draft.parentRef ??= null; if (parsed.job?.receipt) registryReceipt(parsed.job.request, parsed.job.receipt); this.cache = parsed; } } catch { this.unreadable = "导航输入恢复记录无法解析，请保留原记录；新操作与提交暂停。"; }
    this.render(); await this.refresh();
    if (epoch === this.epoch && this.cache.job) await this.run(() => this.lookup());
  }
  private async authority(identity: HostedLinksIdentity | null): Promise<boolean> {
    if (!identity || !sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me");
    if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) { this.owner.authorizationChanged("授权策略已变化，旧目录与标签展示已清除，请重新连接。", current); return false; } return true;
  }
  private persist(): void { if (!this.key || this.unreadable) throw new Error("导航恢复记录不可写，未发送新操作。"); try { sessionStorage.setItem(this.key, JSON.stringify(this.cache)); } catch { throw new Error("浏览器不能可靠保留原导航请求，未发送；输入和原编号仍在本页。"); } }
  private keep(): void { try { this.persist(); } catch (e) { this.message.textContent = (e as Error).message; } }
  private async run(work: () => Promise<void>): Promise<void> { if (this.busy) return; this.busy = true; const epoch = this.epoch; this.render(); try { await work(); } catch (e) { if (epoch === this.epoch) { this.verified = false; this.resolved.replaceChildren(); this.message.textContent = (e as Error).message; } } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async refresh(): Promise<void> {
    const epoch = this.epoch, serial = ++this.catalogSerial, identity = this.owner.identity();
    try { const result = await this.request<RegistryCatalog>("navigation/registry"); if (!await this.authority(identity) || epoch !== this.epoch || serial !== this.catalogSerial) return;
      if (result.directoryScope !== "configuredAuthorizedDirectories" || !["complete", "partial"].includes(result.directoryCoverage) || !Array.isArray(result.directories) || !Array.isArray(result.admissionSlots) || !result.tags || !["complete", "partial", "unavailable"].includes(result.tags.coverage) || !Array.isArray(result.tags.items)) throw new Error("目录与标签范围格式无法确认，未采用结果。");
      this.catalogTree = result.tags.read && result.tags.coverage === "complete" ? new TagTree(result.tags.items, true) : undefined; this.catalog = result; this.render();
    } catch (e) { if (epoch === this.epoch && serial === this.catalogSerial) { this.catalog = undefined; this.catalogTree = undefined; this.message.textContent = (e as Error).message; this.render(); } }
  }
  private accept(incoming: RegistryReceipt): void { const job = this.cache.job; if (!job) return; job.receipt = registryReceipt(job.request, incoming, job.receipt); job.phase = undefined; this.verified = true; this.keep(); if (incoming.status === "committed" && job.request.intent.kind !== "admitDirectory") { this.resolveSerial++; this.resolved.replaceChildren(); this.owner.registryChanged?.(); } this.message.textContent = incoming.status === "committed" ? "原导航操作已提交；目录和标签是独立导航资格，不授予正文读取或 Query 范围。" : incoming.status === "prepared" ? "请核对原提案，再明确提交。" : "原操作事实已核对；未把结束或未登记冒作提交成功。"; }
  private async prepare(): Promise<void> {
    if (this.cache.job) return; const catalog = this.catalog, draft = this.cache.draft; if (!catalog) throw new Error("请先读取获权目录与标签目录。");
    const intent = registryIntent(draft, catalog);
    const request = { operationId: crypto.randomUUID(), intent }, epoch = this.epoch, identity = this.owner.identity(); this.cache.job = { request, phase: "prepare" }; this.persist();
    const result = await this.request<RegistryReceipt>("navigation/registry/prepare", structuredClone(request)); if (!await this.authority(identity) || epoch !== this.epoch) return; this.accept(result);
  }
  private async lookup(): Promise<void> { const job = this.cache.job; if (!job) return; const epoch = this.epoch, identity = this.owner.identity(); const result = await this.request<RegistryReceipt>("navigation/registry/operation", { operationId: job.request.operationId }); if (!await this.authority(identity) || epoch !== this.epoch) return; this.accept(result); if (result.status === "committed") await this.refresh(); }
  private async commit(): Promise<void> { const job = this.cache.job; if (!this.verified || !job?.receipt?.commitAllowed || !job.receipt.planId) return; const epoch = this.epoch, identity = this.owner.identity(); job.commit ??= { operationId: job.request.operationId, planId: job.receipt.planId }; job.phase = "commit"; this.persist(); const result = await this.request<RegistryReceipt>("navigation/registry/commit", job.commit); if (!await this.authority(identity) || epoch !== this.epoch) return; this.accept(result); if (result.status === "committed") await this.refresh(); }
  private async settle(): Promise<void> { const job = this.cache.job; if (!job) return; const epoch = this.epoch, identity = this.owner.identity(); job.phase = "settle"; this.persist(); const result = await this.request<RegistryReceipt>("navigation/registry/settle", { operationId: job.request.operationId }); if (!await this.authority(identity) || epoch !== this.epoch) return; this.accept(result); }
  private async resolve(body: unknown, expected: string): Promise<void> { const epoch = this.epoch, serial = ++this.resolveSerial, identity = this.owner.identity(); const result = await this.request<RegistryResult>("navigation/registry/resolve", body); if (!await this.authority(identity) || epoch !== this.epoch || serial !== this.resolveSerial) return; if (!result || result.reference !== expected || result.status !== "available" || result.readOnly !== true) throw new Error("导航地址资格无法确认，未采用另一目标。"); this.resolved.replaceChildren(ui("h3", "", result.kind === "tag" ? "获权标签" : "获权目录"), ui("p", "", result.title ?? ""), ui("code", "", result.reference), action("复制已核对的导航地址", () => { void this.run(async () => { if (await this.authority(identity) && epoch === this.epoch && serial === this.resolveSerial) await navigator.clipboard.writeText(result.reference); }); })); }
  private selectTag(reference: string | null): void { this.resolved.replaceChildren(); this.resolveSerial++; if (!reference) return; this.message.textContent = "正在核对明确选择的标签身份…"; const epoch = this.epoch, pending = this.resolve({ kind: "tag", reference }, reference), serial = this.resolveSerial; void pending.catch(error => { if (epoch === this.epoch && serial === this.resolveSerial) this.message.textContent = String(error); }); }
  private render(): void {
    const focused = this.editor.contains(document.activeElement) ? document.activeElement as HTMLElement : null, label = focused?.getAttribute("aria-label");
    const caret = focused instanceof HTMLInputElement && ["text", "search"].includes(focused.type) ? [focused.selectionStart, focused.selectionEnd] : null;
    const restore = () => { const control = label ? this.editor.querySelector<HTMLElement>(`[aria-label="${CSS.escape(label)}"]`) : null; control?.focus({ preventScroll: true }); if (control instanceof HTMLInputElement && caret?.[0] !== null && caret?.[0] !== undefined && caret[1] !== null) control.setSelectionRange(caret[0], caret[1]); };
    this.host.hidden = !this.key; if (this.host.hidden) return; this.host.replaceChildren(ui("h2", "", "目录与标签导航"), this.message, this.editor, this.resolved); this.editor.replaceChildren();
    this.editor.append(action("刷新目录与标签", () => { void this.run(() => this.refresh()); }), ui("p", "hint", "目录地址与标签不会自动扫描子笔记或改变 Query 范围；正文草稿与 Undo 独立保留。"));
    if (this.unreadable) { this.editor.append(ui("p", "hint", this.unreadable)); return; }
    const catalog = this.catalog;
    if (catalog) {
      if (catalog.directoryCoverage === "partial") this.editor.append(ui("p", "hint", "目录导航部分可用；不可用项保留，不猜测路径。"));
      for (const directory of catalog.directories) { const button = action(`核对目录：${directory.title}`, () => { void this.run(() => this.resolve({ kind: "directory", directory: directory.directory }, directory.reference)); }); button.disabled = this.busy || !directory.canResolve || directory.status !== "available"; this.editor.append(button); }
      if (catalog.tags.coverage !== "complete") this.editor.append(ui("p", "hint", catalog.tags.coverage === "partial" ? "标签目录部分可用。" : "标签读取当前不可用。"));
      if (this.catalogTree) this.editor.append(tagPicker(this.catalogTree, { label: "获权标签目录 · 选择后核对地址", changed: reference => this.selectTag(reference) }));
    }
    const draft = this.cache.draft, form = ui("fieldset"); form.append(ui("legend", "", "导航提案"));
    const kind = selection("导航操作类型", [["admitDirectory", "明确纳入目录"], ["createTag", "新建标签"], ["renameTag", "重命名标签"], ["moveTag", "标签换父／移至根"]], draft.kind); kind.onchange = () => { if (kind.isConnected === false) return; this.inputScopes.clear(); draft.kind = kind.value as RegistryDraft["kind"]; this.keep(); this.render(); }; form.append(kind);
    if (draft.kind === "admitDirectory") { const slots = selection("目录纳入槽位", [["", "明确选择槽位"], ...(catalog?.admissionSlots ?? []).map(s => [s.slot, s.title] as [string, string])], draft.slot); slots.onchange = () => { if (slots.isConnected === false) return; this.inputScopes.clear(); draft.slot = slots.value; this.keep(); this.render(); }; form.append(slots); }
    else {
      if (this.catalogTree) { const tree = this.catalogTree;
        if (draft.kind !== "createTag") form.append(tagPicker(tree, { label: "待操作标签", value: draft.reference, changed: reference => { this.inputScopes.clear(); draft.reference = reference ?? ""; this.keep(); this.render(); } }));
        if (draft.kind !== "renameTag") form.append(tagPicker(tree, { label: "新父标签", value: draft.parentRef ?? "", root: true, excludedRoot: draft.kind === "moveTag" ? tree.byRef.get(draft.reference)?.tagId : undefined, changed: reference => { this.inputScopes.clear(); draft.parentRef = reference; this.keep(); } }));
      }
      else form.append(ui("p", "hint", "完整标签结构未获权读取；暂停父级选择及结构操作，原输入保留。"));
      if (draft.kind !== "moveTag") { const name = ui("input", "data-input"), duplicates = ui("div", "hint"); name.setAttribute("aria-label", "标签显示名称"); name.value = draft.displayName; const epoch = this.epoch, target = JSON.stringify([draft.kind, draft.slot, draft.reference, draft.parentRef]); this.inputScopes.get(draft, { kind: "registry", target }, () => this.epoch === epoch && this.cache.draft === draft && JSON.stringify([draft.kind, draft.slot, draft.reference, draft.parentRef]) === target)?.bind(name, "display-name"); const showNames = () => { duplicates.replaceChildren(); if (draft.kind !== "createTag" || !this.catalogTree || !name.value) return; const same = [...this.catalogTree.byId.values()].filter(tag => tag.displayName === name.value); if (same.length) duplicates.append(ui("p", "", `已有 ${same.length} 个同名标签；仍创建会产生新的独立身份，不合并。`), ...same.slice(0, 100).map(tag => ui("p", "", this.catalogTree!.label(tag.reference)))); }; name.oninput = () => { draft.displayName = name.value; this.keep(); showNames(); }; showNames(); form.append(name, duplicates); }
    }
    const canPrepare = !!catalog && (draft.kind === "admitDirectory" ? catalog.admissionSlots.some(s => s.slot === draft.slot) : catalog.tags.read && catalog.tags.coverage === "complete" && !!catalog.tags.basis && (draft.kind === "createTag" ? catalog.tags.create : draft.kind === "moveTag" ? catalog.tags.move : catalog.tags.rename));
    const prepare = action("准备导航提案", () => { void this.run(() => this.prepare()); }); prepare.disabled = this.busy || !!this.cache.job || !canPrepare; form.append(prepare); this.editor.append(form);
    const job = this.cache.job; if (!job) { restore(); return; } const card = ui("section", "output-status"), receipt = this.verified ? job.receipt : undefined; card.append(ui("h3", "", "原导航操作"), ui("p", "", job.request.operationId), ui("p", "", receipt?.status ?? "原编号保留，尚待当前授权核对"));
    if (receipt) { for (const [key, label] of [["title", "目录"], ["beforeName", "原标签"], ["displayName", "拟议名称"]] as const) if (receipt.preview[key] !== undefined) card.append(ui("p", "", `${label}：${receipt.preview[key]}`)); }
    if (receipt) { for (const [key, label] of [["beforeParentRef", "原父级"], ["parentRef", "拟议父级"]] as const) if (receipt.preview[key] !== undefined) card.append(ui("p", "", `${label}：${receipt.preview[key] ?? "根级（无父标签）"}`)); }
    const commit = action("明确提交原导航提案", () => { void this.run(() => this.commit()); }); commit.disabled = this.busy || !!job.phase || !receipt?.commitAllowed;
    const status = action("核对原导航操作", () => { void this.run(() => this.lookup()); }); status.disabled = this.busy;
    const settle = action("结束原未安装导航操作", () => { void this.run(() => this.settle()); }); settle.disabled = this.busy || ended(receipt?.status); card.append(commit, status, settle);
    if (receipt?.result?.canResolve && receipt.result.reference) { const body = job.request.intent.kind === "admitDirectory" ? { kind: "admittedDirectory", slot: job.request.intent.slot, operationId: job.request.operationId, reference: receipt.result.reference } : { kind: "tag", reference: receipt.result.reference }; card.append(action("核对原操作的当前导航地址", () => { void this.run(() => this.resolve(body, receipt.result!.reference)); })); }
    const fresh = action("明确准备另一导航操作", () => { delete this.cache.job; this.verified = false; this.resolved.replaceChildren(); this.keep(); this.render(); }); fresh.disabled = this.busy || !ended(receipt?.status); card.append(fresh); this.editor.append(card); restore();
  }
}
