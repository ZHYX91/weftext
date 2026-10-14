import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { nodeSourceHash } from "./hosted-navigation-state";
import { sameHostedLinksContext, sameHostedLinksIdentity, type HostedLinksContext, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { citationPatches, type CitationDraft, type CitationProposal, type CitationWork } from "./hosted-citations-state";
import type { EmbeddedQueryBasis } from "./embedded-query";
import type { SourcePatch } from "./editor";

type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedCitationHost {
  context(): HostedLinksContext | null;
  identity(): HostedLinksIdentity | null;
  basis(): Promise<EmbeddedQueryBasis>;
  apply(patches: SourcePatch[], basis: EmbeddedQueryBasis): Promise<void>;
  authorizationChanged(message: string, current: { actor: string; deployment: string; generation: number }): void;
}
/** Publication choices are independent capabilities, never inferred from body read. */
export class HostedCitations {
  private epoch = 0;
  private inputVersion = 0;
  private prefix = "";
  private drafts: Record<string, CitationDraft> = {};
  private works: CitationWork[] = [];
  private coverage = "complete";
  private unreadable?: string;
  private busy = false;
  private proposeButton?: HTMLButtonElement;
  private message = ui("p", "hint");
  private editor = ui("section");
  constructor(private host: HTMLElement, private request: Request, private owner: HostedCitationHost) { this.clear(false); }
  clear(removeCache = true): void {
    this.epoch++; this.inputVersion++; this.prefix = ""; this.drafts = {}; this.works = []; this.busy = false; this.unreadable = undefined; this.editor.replaceChildren(); this.message.textContent = ""; this.host.replaceChildren(); this.host.hidden = true;
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-citation:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number): Promise<void> {
    this.clear(false); this.prefix = hostedCachePrefix("citation", deployment, actor, generation); const epoch = this.epoch, identity = this.owner.identity();
    if (!identity || identity.deployment !== deployment || identity.actor !== actor || identity.generation !== generation) { this.clear(false); return; }
    try { const raw = sessionStorage.getItem(this.prefix + "inputs"); if (raw) { const value = JSON.parse(raw) as Record<string, CitationDraft>; if (!value || typeof value !== "object" || Object.values(value).some(v => !v || typeof v.key !== "string" || typeof v.work !== "string")) throw new Error(); this.drafts = value; } } catch { this.unreadable = "引用输入缓存无法解析；请先保留本标签页原缓存，不发送提案。"; }
    this.documentChanged();
    try {
      const result = await this.request<{ works: CitationWork[]; coverage: "complete" | "partial" }>("citations");
      if (!identity || !await this.authority(identity) || epoch !== this.epoch) return;
      if (!Array.isArray(result.works) || !["complete", "partial"].includes(result.coverage) || result.works.some(w => !w || typeof w.resource !== "string" || typeof w.title !== "string" || !["ready", "unavailable"].includes(w.status))) throw new Error("文献目录格式无法确认，未采用结果。");
      this.works = result.works; this.coverage = result.coverage; this.paint();
    } catch (error) { if (epoch === this.epoch) { this.message.textContent = error instanceof Error ? error.message : "文献目录读取未完成。"; } }
  }
  documentChanged(): void { this.inputVersion++; this.busy = false; this.message.textContent = "选择获权 Work 与原生引用键，提案只进入当前草稿；普通保存才提交正文。"; this.paint(); }
  private async authority(identity: HostedLinksIdentity): Promise<boolean> {
    if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me");
    if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) { this.owner.authorizationChanged("授权策略已变化，旧引用目录与输入展示已清除，请重新连接。", current); return false; }
    return true;
  }
  private persist(): void { if (!this.prefix || this.unreadable) throw new Error("引用输入恢复记录不可写，未发送提案。"); try { sessionStorage.setItem(this.prefix + "inputs", JSON.stringify(this.drafts)); } catch { throw new Error("浏览器无法可靠保留引用输入，未发送提案；原输入仍在本页。"); } }
  private async propose(): Promise<void> {
    if (this.busy) return;
    const before = this.owner.context(); if (!before || !before.writable || before.pending || before.compare) throw new Error("请先核对当前可写草稿与原保存结果。");
    const draft = this.drafts[before.resource]; if (!draft || !/^[A-Za-z][A-Za-z0-9_-]{0,255}$/.test(draft.key)) throw new Error("引用键须为 ASCII 字母开头的原生标识；无效输入保留，请明确修改。");
    if (!this.works.some(w => w.resource === draft.work && w.status === "ready")) throw new Error("原 Work 选择当前不可用，请明确重新选择。");
    const epoch = this.epoch, version = this.inputVersion, frozen = { ...draft }; this.busy = true; this.paint();
    try {
      const basis = await this.owner.basis(), context = this.owner.context();
      if (!context || epoch !== this.epoch || version !== this.inputVersion || basis.sessionId !== context.sessionId || basis.serial !== context.serial || basis.source !== context.source || !sameHostedLinksContext(before, context)) return;
      const hash = await nodeSourceHash(context.source); if (!sameHostedLinksContext(context, this.owner.context())) return;
      this.persist();
      const result = await this.request<CitationProposal>("citations/propose", { sessionId: context.sessionId, source: context.source, serial: context.serial, sourceHash: hash, key: frozen.key, work: frozen.work });
      if (!await this.authority(context) || epoch !== this.epoch || version !== this.inputVersion || !sameHostedLinksContext(context, this.owner.context())) return;
      await this.owner.apply(citationPatches(result, context, hash), basis);
      if (epoch === this.epoch) this.message.textContent = "引用与准确 Work 绑定已加入同一草稿及 Undo；尚未保存正文。";
    } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "引用提案未完成，原稿和输入保留。"; }
    finally { if (epoch === this.epoch) { this.busy = false; const current = this.owner.context(); if (this.proposeButton) this.proposeButton.disabled = !current?.writable || !!current.pending || !!current.compare; } }
  }
  private paint(): void {
    this.editor.replaceChildren(); this.host.hidden = !this.owner.context(); if (this.host.hidden) { this.host.replaceChildren(); return; }
    this.host.replaceChildren(ui("h2", "", "正式文献引用"), this.message, this.editor);
    if (this.unreadable) { this.editor.append(ui("p", "hint", this.unreadable)); return; }
    const context = this.owner.context()!, draft = this.drafts[context.resource] ??= { key: "", work: "" };
    if (this.coverage === "partial") this.editor.append(ui("p", "hint", "文献目录部分可用；不可用 Work 仍保留原选择，不按空列表冒完整成功。"));
    const work = ui("select", "data-input"); work.setAttribute("aria-label", "引用 Work");
    const choices = [["", "明确选择获权文献"], ...this.works.map(w => [w.resource, `${w.title}${w.status === "unavailable" ? "（当前不可用）" : ""}`])];
    if (draft.work && !choices.some(([id]) => id === draft.work)) choices.push([draft.work, "原 Work（当前不在获权目录）"]);
    for (const [value, label] of choices) { const option = ui("option", "", label); option.value = value; option.disabled = !!value && this.works.some(w => w.resource === value && w.status !== "ready"); work.append(option); } work.value = draft.work;
    const key = ui("input", "data-input"); key.setAttribute("aria-label", "原生引用键"); key.value = draft.key;
    const keep = () => { this.inputVersion++; try { this.persist(); } catch (error) { this.message.textContent = (error as Error).message; } };
    work.onchange = () => { draft.work = work.value; keep(); }; key.oninput = () => { draft.key = key.value; keep(); };
    const propose = action("将文献引用提案加入当前草稿", () => { void this.propose().catch(e => { this.message.textContent = (e as Error).message; }); });
    this.proposeButton = propose; propose.disabled = this.busy || !context.writable || context.pending || context.compare; this.editor.append(work, key, propose);
  }
}
