import "./knowledge-view.css";
import { action, ui } from "./data-panel";
import { nodeSourceHash } from "./hosted-navigation-state";
import { hostedLinksPatch, requireHostedLinksBasis, sameHostedLinksContext, sameHostedLinksIdentity, type HostedLinksBasis, type HostedLinksContext, type HostedLinksIdentity, type HostedLinksEdge, type HostedLinksHit, type HostedLinksProfile, type HostedLinksProposal, type HostedLinksResolution } from "./hosted-knowledge-links-state";
import type { SourcePatch } from "./editor";
import { PersonalRelationRules, RelationshipDiagnosticsView, visibleCandidate, type RelationNode, type RelationIssue } from "./relationship-diagnostics";
import type { HostedLinksNode } from "./hosted-knowledge-links-state";
import { RelationshipGraphView } from "./relationship-graph-view";
import { HeadingStructureView } from "./heading-structure-view";
import type { StructureHeading } from "./heading-structure-state";

type Request = <T>(path: string, body?: unknown, signal?: AbortSignal) => Promise<T>;
export interface HostedKnowledgeLinksHost {
  context(): HostedLinksContext | null;
  identity(): HostedLinksIdentity | null;
  flush(): Promise<void>;
  apply(patches: SourcePatch[], basis: { sessionId: string; source: string; serial: number }, event: string): Promise<void>;
  navigate(reference: string): Promise<void>;
  select(from: number, to: number): void;
  revealRendered?(heading: StructureHeading): boolean;
  authorizationChanged(message: string, identity: { actor: string; deployment: string; generation: number }): void;
}
const states: Record<string, string> = { resolved: "目标有效", unavailable: "目标当前不可用", missing: "在本次获权范围未找到目标", ambiguous: "目标或片段有歧义", fragmentMissing: "目标片段缺失", invalid: "原地址无效", external: "外部地址", inDocument: "文内引用" };

/** No independent editor, persistence protocol or protected result cache.
 * Every button is keyboard accessible, including all overlapping candidates. */
export class HostedKnowledgeLinks {
  private epoch = 0;
  private busy = false;
  private stale = false;
  private composing = false;
  private deferredPaint = false;
  private controller?: AbortController;
  private profile?: HostedLinksProfile;
  private captured?: HostedLinksContext;
  private hash?: string;
  private showMentions = false;
  private mentionDomain = "all";
  private exclusionScope = "note";
  private rules = new PersonalRelationRules();
  private diagnostics = this.diagnosticView();
  private graph = new RelationshipGraphView();
  private headings = new HeadingStructureView();
  private viewSession = "";
  private diagnosticView(): RelationshipDiagnosticsView {
    return new RelationshipDiagnosticsView(this.rules, async node => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(node.reference); }, async issue => {
      const { context, epoch } = this.current(); if (!await this.audience(context, epoch)) return;
      await this.owner.navigate(issue.source.reference);
      if (issue.field) { this.message.textContent = "已打开属性来源笔记；没有精确源码位置，请在获权字段入口查看。"; return; }
      const opened = this.owner.context();
      if (!opened || !issue.sourceHash || await nodeSourceHash(opened.source) !== issue.sourceHash) { this.message.textContent = "已打开来源笔记；当前正文已变化，没有采用旧精确位置。"; return; }
      if (issue.from !== undefined && issue.to !== undefined) this.owner.select(issue.from, issue.to);
    }, error => { this.message.textContent = error instanceof Error ? error.message : String(error); });
  }
  private relationNode(node: HostedLinksNode): RelationNode { return { key: `${node.nodeRef.workspaceId}/${node.nodeRef.nodeId}`, title: node.title, reference: node.reference }; }
  private readonly content = ui("div");
  private readonly message = ui("p", "hint");
  private readonly refresh = action("读取链接与潜在提及", () => this.run(() => this.read()));
  private readonly stop = action("停止等待链接扫描", () => { this.epoch++; this.controller?.abort(); this.controller = undefined; this.busy = false; this.profile = undefined; this.content.replaceChildren(); this.refresh.disabled = false; this.stop.disabled = true; this.message.textContent = "已停止等待只读扫描；服务端计算可能继续，正文与个人规则保留。"; });
  constructor(private readonly host: HTMLElement, private readonly request: Request, private readonly owner: HostedKnowledgeLinksHost) { this.clear(); host.addEventListener("compositionstart", () => { this.composing = true; }); host.addEventListener("compositionend", () => { this.composing = false; if (this.deferredPaint) { this.deferredPaint = false; this.paint(); } }); }
  clear(): void { this.graph.clear(); this.headings.clear(); this.rules.closePreviews(); this.epoch++; this.controller?.abort(); this.controller = undefined; this.profile = undefined; this.captured = undefined; this.hash = undefined; this.busy = false; this.showMentions = false; this.rules = new PersonalRelationRules(); this.diagnostics = this.diagnosticView(); this.content.replaceChildren(); this.host.replaceChildren(); this.host.hidden = true; }
  connect(): void { this.clear(); this.documentChanged(); }
  async showStructure(kind: "graph" | "headings"): Promise<void> {
    if (!this.profile) await this.run(() => this.read());
    const target = this.content.querySelector<HTMLDetailsElement>(`details[data-knowledge-view="${kind}"]`);
    if (target) { target.open = true; target.scrollIntoView({ block: "start", behavior: "instant" }); target.querySelector<HTMLElement>("summary")?.focus(); }
  }
  documentChanged(): void {
    if (this.viewSession !== (this.owner.context()?.sessionId ?? "")) { this.viewSession = this.owner.context()?.sessionId ?? ""; this.graph.clear(); this.headings.clear(); }
    this.rules.closePreviews(); this.epoch++; this.controller?.abort(); this.controller = undefined; this.profile = undefined; this.captured = undefined; this.hash = undefined; this.busy = false; this.content.replaceChildren();
    this.host.hidden = !this.owner.context();
    if (!this.host.hidden) { this.message.textContent = "读取当前草稿与明确获权对象；结果完整性仅指本次获权范围。"; this.host.replaceChildren(ui("h2", "", "链接与潜在提及"), this.refresh, this.stop, this.message, this.content); } this.stop.disabled = true;
    this.refresh.disabled = false;
  }
  environmentChanged(): void { this.documentChanged(); }
  private async run(work: () => Promise<void>): Promise<void> {
    if (this.busy) return;
    this.busy = true; const epoch = this.epoch; this.refresh.disabled = true; this.stop.disabled = false;
    try { await work(); } catch (error) { if (epoch === this.epoch) {
      const http = (error as { http?: number })?.http;
      let retained = false;
      if (http && (http >= 500 || http === 429) && this.captured && this.profile) { try { retained = await this.audience(this.captured, epoch); } catch { /* Cannot retain protected results without current audience. */ } }
      if (epoch !== this.epoch) return;
      if (retained) this.stale = true; else { this.profile = undefined; this.content.replaceChildren(); }
      this.message.textContent = `${retained ? "扫描失败；仍获权的最近结果已过期，只供查看，请重新读取。" : ""}${error instanceof Error ? error.message : "链接读取未完成；草稿与原链接保留。"}`;
    } }
    finally { if (epoch === this.epoch) { this.busy = false; this.refresh.disabled = false; this.stop.disabled = true; if (this.profile) this.paint(); } }
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
    this.profile = result; this.captured = context; this.hash = hash; this.stale = false;
    this.message.textContent = result.links.coverage === "complete" ? "已读取本次明确获权范围的链接。" : "链接扫描未完整，仅显示已确认的结果；空列表不代表全库没有链接。";
  }
  private current(): { context: HostedLinksContext; profile: HostedLinksProfile; hash: string; epoch: number } {
    const context = this.captured, profile = this.profile, hash = this.hash;
    if (this.stale || !context || !profile || !hash || !this.accepts(context, this.epoch)) throw new Error("当前草稿或展示依据已变化，请重新读取链接。");
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
    for (const candidate of hit.candidates ?? hit.targets.map(target => ({ target, term: hit.terms[0], provenance: "title", fragment: undefined }))) {
      const target = candidate.target;
      if ((this.mentionDomain === "fragments" && !candidate.fragment || this.mentionDomain === "notes" && candidate.fragment) || !visibleCandidate(this.rules.rules, this.owner.context()?.resource ?? "", candidate.term, this.relationNode(target).key)) continue;
      const choice = ui("div"), open = action("打开此候选", () => this.run(async () => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(`${target.reference}${candidate.fragment ? `#${encodeURIComponent(candidate.fragment)}` : ""}`); })); open.disabled = !target.canOpen; choice.append(ui("span", "", `${target.title}${candidate.fragment ? ` #${candidate.fragment}` : ""} · ${({ title: "笔记标题", nameField: "名称字段", heading: "标题文字", stableLabel: "稳定标签" })[candidate.provenance] ?? candidate.provenance}`), open);
      const convert = action(`将此范围转换为链接：${target.title}${candidate.fragment ? ` #${candidate.fragment}` : ""}`, () => this.run(() => this.convert({ kind: "mention", fragmentId: fragment, startScalar: hit.startScalar, endScalar: hit.endScalar, target: target.nodeRef, fragment: candidate.fragment })));
      const context = this.owner.context(); convert.disabled = this.busy || !hit.writable || !this.profile?.writable || !context?.writable || !!context.pending || !!context.compare;
      const source = this.exclusionScope === "note" ? this.owner.context()?.resource : undefined;
      choice.append(convert, action("排除此目标…", () => this.rules.preview({ kind: "target", value: this.relationNode(target).key, source }, `准确目标：${target.title} · ${this.relationNode(target).key}；范围：${source ? "此篇" : "本页授权范围"}，只存当前页面偏好。`, this.profile?.mentions.fragments.flatMap(f => f.hits).filter(h => h.targets.some(t => t.resource === target.resource)).length ?? 0, () => this.paint())), action("忽略此词…", () => this.rules.preview({ kind: "term", value: candidate.term, source }, `精确词：${candidate.term}；范围：${source ? "此篇" : "本页授权范围"}，退出连接后清除。`, this.profile?.mentions.fragments.flatMap(f => f.hits).filter(h => h.terms.includes(candidate.term)).length ?? 0, () => this.paint()))); row.append(choice);
    }
    if (!hit.writable) row.append(ui("p", "hint", "此提及没有当前唯一可写的文字范围，可阅读并选择候选。"));
    return row;
  }
  private paint(): void {
    const profile = this.profile; if (!profile) return;
    if (this.composing) { this.deferredPaint = true; return; }
    const active = document.activeElement;
    const focus = active instanceof HTMLInputElement && this.content.contains(active) ? { label: active.getAttribute("aria-label"), start: active.selectionStart, end: active.selectionEnd } : null;
    this.content.replaceChildren();
    const graphSection = ui("details"), graphTitle = ui("summary", "", "二维关系图谱"); graphSection.dataset.knowledgeView = "graph"; graphSection.append(graphTitle);
    const root = profile.graph?.root ?? "";
    this.graph.render(graphSection, profile.graph, root, {
      open: async node => { const { context, epoch } = this.current(); if (await this.audience(context, epoch)) await this.owner.navigate(node.reference); },
      source: async (edge, node) => { const { context, epoch } = this.current(); if (!await this.audience(context, epoch)) return; await this.owner.navigate(node.reference); const opened = this.owner.context(); if (!opened || await nodeSourceHash(opened.source) !== edge.sourceHash) { this.message.textContent = "已打开作者来源；正文已变化，没有采用旧精确位置。"; return; } if (edge.range) this.owner.select(edge.range.utf16From, edge.range.utf16To); else this.message.textContent = `已打开作者来源，请在独立获权属性入口编辑 ${edge.fieldId ?? ""}。`; }, error: error => { this.message.textContent = error instanceof Error ? error.message : String(error); },
    }); this.content.append(graphSection);
    const headingSection = ui("details"); headingSection.dataset.knowledgeView = "headings"; headingSection.append(ui("summary", "", "只读标题结构"));
    this.headings.render(headingSection, profile.headingStructure, heading => { void this.run(async () => { const { context, epoch } = this.current(); if (!await this.audience(context, epoch)) return; if (heading.range) this.owner.select(heading.range.utf16From, heading.range.utf16To); else if (!this.owner.revealRendered?.(heading)) this.message.textContent = "此生成标题没有唯一作者地址；当前原生显示位置未匹配，请更新阅读预览后刷新，不按文字猜测源码。"; }); }); this.content.append(headingSection);
    const d = profile.diagnostics;
    this.diagnostics.render(this.content, d && { ...d, issues: d.issues.map<RelationIssue>(i => ({ source: this.relationNode(i.source), target: i.resolution.target ? this.relationNode(i.resolution.target) : undefined, address: i.occurrence.target, status: i.resolution.status, from: i.occurrence.utf16From, to: i.occurrence.utf16To, sourceHash: i.sourceHash })).concat((d.propertyIssues ?? []).map(p => ({ source: this.relationNode(p.source), target: p.resolution.target ? this.relationNode(p.resolution.target) : undefined, address: p.address, status: p.resolution.status, from: undefined, to: undefined, sourceHash: undefined, field: [p.fieldId, ...p.itemPath].join(" / ") }))), nodes: d.nodes.map(n => ({ ...n, node: this.relationNode(n.node) })) });
    for (const [label, edges, outgoing] of [["出链", profile.links.outgoing, true], ["稳定 ID 反链", profile.links.incomingId, false], ["当前路径入链", profile.links.incomingPath, false]] as const) {
      const section = ui("section"); section.append(ui("h3", "", label)); if (!edges.length) section.append(ui("p", "hint", profile.links.coverage === "partial" ? "本次已确认部分没有此类结果，扫描未完整。" : "本次明确获权范围没有此类结果。")); for (const edge of edges) section.append(this.edge(edge, outgoing)); this.content.append(section);
    }
    const rendered = ui("section"); rendered.append(ui("h3", "", "当前 HTML 编译链接"));
    for (const link of profile.renderedLinks) { const row = ui("div"); row.append(ui("p", "", link.destination)); this.destination(row, link.resolution); row.append(ui("p", "hint", "编译出现项只读；没有唯一作者槽时不提供转换。")); rendered.append(row); } this.content.append(rendered);
    const properties = ui("section"); properties.append(ui("h3", "", "属性引用"), ui("p", "hint", "仅显示独立获权字段中的准确引用。")); if (profile.propertyReferences.coverage === "partial") properties.append(ui("p", "hint", "属性引用解析未完整，保留已确认项。"));
    for (const item of profile.propertyReferences.items) { const row = ui("div"); row.append(ui("p", "", `${item.source.title} · ${item.fieldId}`)); this.destination(row, item.resolution); properties.append(row); } this.content.append(properties);
    const mentions = ui("section"), show = ui("input"); show.type = "checkbox"; show.checked = this.showMentions; const label = ui("label"); label.append(show, document.createTextNode("显示潜在提及")); show.onchange = () => { this.showMentions = show.checked; this.paint(); }; mentions.append(ui("h3", "", "潜在提及"), label);
    mentions.append(ui("p", "hint", "当前可见普通文字的精确 Unicode 字面匹配；候选来自可读标题、独立获权名称字段与本次编译确认的唯一稳定片段。无稳定标签的标题不会猜测寻址。提示默认关闭，只改个人呈现。"));
    const domain = ui("select"); domain.setAttribute("aria-label", "候选范围"); for (const [v, t] of [["all", "标题、名称及稳定片段"], ["notes", "仅标题与名称"], ["fragments", "仅稳定片段"]]) { const o = ui("option", "", t); o.value = v; domain.append(o); } domain.value = this.mentionDomain; domain.onchange = () => { this.mentionDomain = domain.value; this.paint(); }; mentions.append(domain);
    const scope = ui("select"); scope.setAttribute("aria-label", "排除规则范围"); for (const [v, t] of [["note", "此篇"], ["page", "本页授权范围"]]) { const o = ui("option", "", t); o.value = v; scope.append(o); } scope.value = this.exclusionScope; scope.onchange = () => { this.exclusionScope = scope.value; }; mentions.append(scope); this.rules.controls(mentions, () => this.paint());
    if (profile.mentions.coverage === "partial") mentions.append(ui("p", "hint", "可见文字或候选词典未完整；没有命中不代表全库没有潜在链接。"));
    if (this.showMentions) for (const fragment of profile.mentions.fragments) { if (!fragment.hits.length) continue; const group = ui("section"); group.append(ui("p", "", fragment.text)); if (fragment.coverage === "partial") group.append(ui("p", "hint", "此逻辑文字片段仅扫描了部分范围。")); for (const hit of fragment.hits) group.append(this.mention(fragment.id, hit)); mentions.append(group); } this.content.append(mentions);
    if (focus?.label) { const input = this.content.querySelector<HTMLInputElement>(`input[aria-label="${CSS.escape(focus.label)}"]`); input?.focus(); if (input && focus.start !== null && focus.end !== null) input.setSelectionRange(focus.start, focus.end); }
  }
}
