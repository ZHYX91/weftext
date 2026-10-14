import { action, ui } from "./data-panel";
import { nodeSourceHash } from "./hosted-navigation-state";
import { hostedLinksPatch, requireHostedLinksBasis, sameHostedLinksContext, sameHostedLinksIdentity, type HostedLinksBasis, type HostedLinksContext, type HostedLinksIdentity, type HostedLinksEdge, type HostedLinksHit, type HostedLinksProfile, type HostedLinksProposal, type HostedLinksResolution } from "./hosted-knowledge-links-state";
import type { SourcePatch } from "./editor";

type Request = <T>(path: string, body?: unknown, signal?: AbortSignal) => Promise<T>;
export interface HostedKnowledgeLinksHost {
  context(): HostedLinksContext | null;
  identity(): HostedLinksIdentity | null;
  flush(): Promise<void>;
  apply(patches: SourcePatch[], basis: { sessionId: string; source: string; serial: number }, event: string): Promise<void>;
  navigate(reference: string): Promise<void>;
  select(from: number, to: number): void;
  authorizationChanged(message: string, identity: { actor: string; deployment: string; generation: number }): void;
}
const states: Record<string, string> = { resolved: "目标有效", unavailable: "目标当前不可用", missing: "在本次获权范围未找到目标", ambiguous: "目标或片段有歧义", fragmentMissing: "目标片段缺失", invalid: "原地址无效", external: "外部地址", inDocument: "文内引用" };

/** No independent editor, persistence protocol or protected result cache.
 * Every button is keyboard accessible, including all overlapping candidates. */
export class HostedKnowledgeLinks {
  private epoch = 0;
  private busy = false;
  private controller?: AbortController;
  private profile?: HostedLinksProfile;
  private captured?: HostedLinksContext;
  private hash?: string;
  private showMentions = true;
  private readonly content = ui("div");
  private readonly message = ui("p", "hint");
  private readonly refresh = action("读取链接与潜在提及", () => this.run(() => this.read()));
  constructor(private readonly host: HTMLElement, private readonly request: Request, private readonly owner: HostedKnowledgeLinksHost) { this.clear(); }
  clear(): void { this.epoch++; this.controller?.abort(); this.controller = undefined; this.profile = undefined; this.captured = undefined; this.hash = undefined; this.busy = false; this.showMentions = true; this.content.replaceChildren(); this.host.replaceChildren(); this.host.hidden = true; }
  connect(): void { this.clear(); this.documentChanged(); }
  documentChanged(): void {
    this.epoch++; this.controller?.abort(); this.controller = undefined; this.profile = undefined; this.captured = undefined; this.hash = undefined; this.busy = false; this.content.replaceChildren();
    this.host.hidden = !this.owner.context();
    if (!this.host.hidden) { this.message.textContent = "读取当前草稿与明确获权对象；结果完整性仅指本次获权范围。"; this.host.replaceChildren(ui("h2", "", "链接与潜在提及"), this.refresh, this.message, this.content); }
    this.refresh.disabled = false;
  }
  environmentChanged(): void { this.documentChanged(); }
  private async run(work: () => Promise<void>): Promise<void> {
    if (this.busy) return;
    this.busy = true; const epoch = this.epoch; this.refresh.disabled = true;
    try { await work(); } catch (error) { if (epoch === this.epoch) { this.profile = undefined; this.content.replaceChildren(); this.message.textContent = error instanceof Error ? error.message : "链接读取未完成；草稿与原链接保留。"; } }
    finally { if (epoch === this.epoch) { this.busy = false; this.refresh.disabled = false; if (this.profile) this.paint(); } }
  }
  private accepts(context: HostedLinksContext, epoch: number): boolean { return epoch === this.epoch && sameHostedLinksContext(context, this.owner.context()); }
  private async audience(context: HostedLinksContext, epoch: number): Promise<boolean> {
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me");
    if (!sameHostedLinksIdentity(context, this.owner.identity())) return false;
    if (current.actor !== context.actor || current.deployment !== context.deployment || current.generation !== context.generation) { this.clear(); this.owner.authorizationChanged("授权策略已变化，旧链接与提及结果已清除，请重新连接并核对草稿。", current); return false; }
    return this.accepts(context, epoch);
  }
  private async read(): Promise<void> {
    await this.owner.flush();
    const context = this.owner.context(); if (!context) throw new Error("请先打开获权笔记。");
    const epoch = this.epoch, hash = await nodeSourceHash(context.source);
    if (!this.accepts(context, epoch)) return;
    const controller = new AbortController(); this.controller = controller;
    const result = await this.request<HostedLinksProfile>("knowledge-links/read", { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, controller.signal);
    if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch)) return;
    requireHostedLinksBasis(result.basis, context, hash);
    if (result.propertyReferences?.scope !== "explicitAuthorizedFields" || result.mentions?.scope !== "nativeTitlesAndAuthorizedNameFields") throw new Error("字段引用或名称词典的授权范围无法确认，未采用结果。");
    if (result.scope !== "configuredAuthorizedManagedSources" || !["complete", "partial"].includes(result.links?.coverage) || !["complete", "partial"].includes(result.mentions?.coverage) || !Array.isArray(result.links.outgoing) || !Array.isArray(result.links.incomingId) || !Array.isArray(result.links.incomingPath) || !Array.isArray(result.mentions.fragments) || !Array.isArray(result.renderedLinks) || !Array.isArray(result.propertyReferences?.items) || result.fileConversion !== false) throw new Error("链接范围或结果格式无法确认，旧结果未安装。");
    this.profile = result; this.captured = context; this.hash = hash;
    this.message.textContent = result.links.coverage === "complete" ? "已读取本次明确获权范围的链接。" : "链接扫描未完整，仅显示已确认的结果；空列表不代表全库没有链接。";
  }
  private current(): { context: HostedLinksContext; profile: HostedLinksProfile; hash: string; epoch: number } {
    const context = this.captured, profile = this.profile, hash = this.hash;
    if (!context || !profile || !hash || !this.accepts(context, this.epoch)) throw new Error("当前草稿或展示依据已变化，请重新读取链接。");
    return { context, profile, hash, epoch: this.epoch };
  }
  private async convert(intent: unknown): Promise<void> {
    const original = this.current(); await this.owner.flush();
    if (!this.accepts(original.context, original.epoch)) throw new Error("转换前输入已变化，原稿保留，请重新读取。");
    const { context, profile, hash, epoch } = original;
    if (!context.writable || context.pending || context.compare || !profile.writable) throw new Error("请在当前可写草稿中转换；先核对未决保存或双方稿。");
    const result = await this.request<HostedLinksProposal>("knowledge-links/convert", { basis: { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, snapshot: profile.basis.snapshot, intent });
    if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch)) return;
    const patch = hostedLinksPatch(result, context, hash, profile.basis.snapshot);
    // The host's existing shared editor independently checks composition and
    // exact source/serial before one native Undo transaction.
    await this.owner.apply([patch], { sessionId: context.sessionId, source: context.source, serial: context.serial }, "input.knowledge.link.proposal");
    this.documentChanged(); this.message.textContent = "链接转换已加入当前草稿与同一撤销历史；尚未保存正文。";
  }
  private async copy(edge: HostedLinksEdge, form: string, full: boolean): Promise<void> {
    const original = this.current(); await this.owner.flush();
    if (!this.accepts(original.context, original.epoch)) throw new Error("复制前草稿已变化，请重新读取当前出现项。");
    const { context, profile, hash, epoch } = original;
    const result = await this.request<{ basis: HostedLinksBasis; address: string; fullLink: string; literal: boolean }>("knowledge-links/copy", { basis: { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, snapshot: profile.basis.snapshot, occurrence: { from: edge.occurrence.from, to: edge.occurrence.to, target: edge.occurrence.target }, form });
    if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch)) return;
    requireHostedLinksBasis(result.basis, context, hash, profile.basis.snapshot);
    if (typeof result.address !== "string" || typeof result.fullLink !== "string" || form === "original" && (!result.literal || result.address !== edge.occurrence.target || result.fullLink !== edge.raw)) throw new Error("复制回执与准确原链接不符，未交付另一地址。");
    await navigator.clipboard.writeText(full ? result.fullLink : result.address);
    if (this.accepts(context, epoch)) this.message.textContent = full ? "已复制完整原生链接。" : "已复制目标地址。";
  }
  private destination(parent: HTMLElement, resolution: HostedLinksResolution): void {
    parent.append(ui("span", "hint", states[resolution.status] ?? "目标当前不可用"));
    if (resolution.canOpen && resolution.reference) parent.append(action("打开获权目标", () => this.run(async () => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(resolution.reference!); })));
  }
  private edge(edge: HostedLinksEdge, outgoing: boolean): HTMLElement {
    const row = ui("div", "query-result-row"); row.append(ui("p", "", outgoing ? edge.occurrence.target : `${edge.source.title} · ${edge.occurrence.target}`)); this.destination(row, edge.resolution);
    if (!outgoing) { const open = action("打开来源笔记", () => this.run(async () => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(edge.source.reference); })); open.disabled = !edge.source.canOpen; row.append(open); return row; }
    row.append(action("选中源码出现项", () => this.run(async () => { this.current(); this.owner.select(edge.occurrence.utf16From, edge.occurrence.utf16To); })));
    const forms = ui("select"); forms.setAttribute("aria-label", `链接地址形式：${edge.occurrence.target}`);
    for (const [value, label] of [["original", "保留原文"], ["id", "稳定 ID"], ["relative", "源相对路径（最短有效）"], ["workspace", "库内路径"]]) { const option = ui("option", "", label); option.value = value; forms.append(option); }
    const convert = action("转换此链接", () => this.run(() => this.convert({ kind: "link", occurrence: { from: edge.occurrence.from, to: edge.occurrence.to, target: edge.occurrence.target }, form: forms.value })));
    const update = (): void => { convert.disabled = this.busy || !this.profile?.writable || !this.owner.context()?.writable || !!this.owner.context()?.pending || !!this.owner.context()?.compare || !edge.editable || forms.value === "original" || edge.resolution.status !== "resolved"; };
    forms.onchange = update; update(); row.append(forms, convert, action("复制目标地址", () => this.run(() => this.copy(edge, forms.value, false))), action("复制完整链接", () => this.run(() => this.copy(edge, forms.value, true))));
    return row;
  }
  private mention(fragment: string, hit: HostedLinksHit): HTMLElement {
    const row = ui("div", "query-result-row"); row.append(ui("p", "", `${hit.terms.join(" / ")} · 文字位置 ${hit.startScalar}–${hit.endScalar}`));
    for (const target of hit.targets) {
      const choice = ui("div"), open = action("打开此候选", () => this.run(async () => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(target.reference); })); open.disabled = !target.canOpen; choice.append(ui("span", "", target.title), open);
      const convert = action(`将此范围转换为链接：${target.title}`, () => this.run(() => this.convert({ kind: "mention", fragmentId: fragment, startScalar: hit.startScalar, endScalar: hit.endScalar, target: target.nodeRef })));
      const context = this.owner.context(); convert.disabled = this.busy || !hit.writable || !this.profile?.writable || !context?.writable || !!context.pending || !!context.compare;
      choice.append(convert); row.append(choice);
    }
    if (!hit.writable) row.append(ui("p", "hint", "此提及没有当前唯一可写的文字范围，可阅读并选择候选。"));
    return row;
  }
  private paint(): void {
    const profile = this.profile; if (!profile) return;
    this.content.replaceChildren();
    for (const [label, edges, outgoing] of [["出链", profile.links.outgoing, true], ["稳定 ID 反链", profile.links.incomingId, false], ["当前路径入链", profile.links.incomingPath, false]] as const) {
      const section = ui("section"); section.append(ui("h3", "", label)); if (!edges.length) section.append(ui("p", "hint", profile.links.coverage === "partial" ? "本次已确认部分没有此类结果，扫描未完整。" : "本次明确获权范围没有此类结果。")); for (const edge of edges) section.append(this.edge(edge, outgoing)); this.content.append(section);
    }
    const rendered = ui("section"); rendered.append(ui("h3", "", "当前 HTML 编译链接"));
    for (const link of profile.renderedLinks) { const row = ui("div"); row.append(ui("p", "", link.destination)); this.destination(row, link.resolution); row.append(ui("p", "hint", "编译出现项只读；没有唯一作者槽时不提供转换。")); rendered.append(row); } this.content.append(rendered);
    const properties = ui("section"); properties.append(ui("h3", "", "属性引用"), ui("p", "hint", "仅显示独立获权字段中的准确引用。")); if (profile.propertyReferences.coverage === "partial") properties.append(ui("p", "hint", "属性引用解析未完整，保留已确认项。"));
    for (const item of profile.propertyReferences.items) { const row = ui("div"); row.append(ui("p", "", `${item.source.title} · ${item.fieldId}`)); this.destination(row, item.resolution); properties.append(row); } this.content.append(properties);
    const mentions = ui("section"), show = ui("input"); show.type = "checkbox"; show.checked = this.showMentions; const label = ui("label"); label.append(show, document.createTextNode("显示潜在提及")); show.onchange = () => { this.showMentions = show.checked; this.paint(); }; mentions.append(ui("h3", "", "潜在提及"), label);
    mentions.append(ui("p", "hint", "候选词来自可读原生标题和独立获权名称字段。"));
    if (profile.mentions.coverage === "partial") mentions.append(ui("p", "hint", "可见文字或候选词典未完整；没有命中不代表全库没有潜在链接。"));
    if (this.showMentions) for (const fragment of profile.mentions.fragments) { if (!fragment.hits.length) continue; const group = ui("section"); group.append(ui("p", "", fragment.text)); if (fragment.coverage === "partial") group.append(ui("p", "hint", "此逻辑文字片段仅扫描了部分范围。")); for (const hit of fragment.hits) group.append(this.mention(fragment.id, hit)); mentions.append(group); } this.content.append(mentions);
  }
}
