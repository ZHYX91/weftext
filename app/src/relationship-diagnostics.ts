/** Personal presentation only. No rules alter the authorized scan or real edges. */
export interface RelationNode { key: string; title: string; path?: string; reference: string }
export interface RelationIssue { source: RelationNode; target?: RelationNode; address: string; status: string; from?: number; to?: number; field?: string; sourceHash?: string }
export interface RelationDiagnostics {
  coverage: "complete" | "partial"; scannedNodes: number;
  issues: RelationIssue[];
  nodes: { node: RelationNode; incoming: number; outgoing: number; invalidOutgoing: number; state: string }[];
}
export interface PersonalRule { kind: "issue" | "expected" | "term" | "target"; value: string; source?: string }
export const issueRuleKey = (issue: RelationIssue): string => JSON.stringify([issue.source.key, issue.address]);
export function ruleMatches(rules: PersonalRule[], kind: PersonalRule["kind"], value: string, source?: string): boolean {
  return rules.some(r => r.kind === kind && r.value === value && (!r.source || r.source === source));
}
export function visibleCandidate(rules: PersonalRule[], source: string, term: string, target: string): boolean {
  return !ruleMatches(rules, "term", term, source) && !ruleMatches(rules, "target", target, source);
}
export function diagnosticGroups(issues: RelationIssue[], grouping: string): [string, RelationIssue[]][] {
  const groups = new Map<string, RelationIssue[]>();
  for (const [index, issue] of issues.entries()) {
    const key = grouping === "source" ? issue.source.key : grouping === "occurrence" ? String(index) : issue.target?.key ?? issue.address;
    const group = groups.get(key) ?? []; group.push(issue); groups.set(key, group);
  }
  return [...groups];
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string): HTMLElementTagNameMap[K] { const v = document.createElement(tag); if (text !== undefined) v.textContent = text; return v; }
function button(text: string, run: () => void | Promise<void>): HTMLButtonElement { const b = el("button", text); b.type = "button"; b.className = "knowledge-button"; b.onclick = () => { void run(); }; return b; }
function choice(label: string, options: [string, string][], value: string, changed: (value: string) => void): HTMLLabelElement {
  const l = el("label", label), s = el("select"); s.setAttribute("aria-label", label);
  for (const [value, text] of options) { const o = el("option", text); o.value = value; s.append(o); }
  s.value = value; s.onchange = () => changed(s.value); l.append(s); return l;
}
export class PersonalRelationRules {
  rules: PersonalRule[] = [];
  private previews = new Set<() => void>();
  closePreviews(): void { for (const close of [...this.previews]) close(); }
  private history: PersonalRule[][] = [];
  constructor(private readonly persist?: (rules: PersonalRule[]) => void, initial?: string | null) {
    try { const value: unknown = JSON.parse(initial ?? "[]"); if (Array.isArray(value)) this.rules = value.filter((r): r is PersonalRule => !!r && ["issue", "expected", "term", "target"].includes(r.kind) && typeof r.value === "string" && (r.source === undefined || typeof r.source === "string")); } catch { /* Broken preferences never affect scan authority. */ }
  }
  save(next: PersonalRule[]): void { this.persist?.(next); this.history.push(this.rules); this.rules = next; }
  undo(): void { const next = this.history.at(-1); if (!next) return; this.persist?.(next); this.history.pop(); this.rules = next; }
  preview(rule: PersonalRule, label: string, matches: number, changed: () => void): void {
    const dialog = el("dialog"), error = el("p"); dialog.className = "knowledge-dialog"; dialog.setAttribute("aria-label", "个人关系规则预览");
    dialog.append(el("h3", "个人关系规则预览"), el("p", label), el("p", `当前获权结果匹配 ${matches} 项。只改变个人呈现；正文与真实连接保留。`), error);
    const trigger = document.activeElement as HTMLElement | null;
    const close = (): void => { this.previews.delete(close); dialog.close(); dialog.remove(); if (trigger?.isConnected) trigger.focus(); }; this.previews.add(close);
    dialog.append(button("取消", close), button("保存个人规则", () => { try { this.save([...this.rules.filter(r => JSON.stringify(r) !== JSON.stringify(rule)), rule]); close(); changed(); } catch { error.textContent = "个人偏好保存失败，未应用规则。预览与选择保留，可重试。"; } }));
    dialog.addEventListener("cancel", event => { event.preventDefault(); close(); }); document.body.append(dialog); dialog.showModal();
  }
  controls(parent: HTMLElement, changed: () => void): void {
    const details = el("details"); details.append(el("summary", `个人规则（${this.rules.length}）`));
    for (const rule of this.rules) { const row = el("p", `${({ issue: "忽略问题", expected: "预期孤立", term: "忽略词", target: "排除目标" })[rule.kind]} · ${rule.source ? "此篇" : "此授权范围"}`); row.append(el("code", rule.value), button("移除此规则", () => { try { this.save(this.rules.filter(r => r !== rule)); changed(); } catch { row.append(el("span", "偏好保存失败，请重试。")); } })); details.append(row); }
    const undo = button("撤销偏好修改", () => { try { this.undo(); changed(); } catch { details.append(el("p", "偏好保存失败，原规则保留。")); } }); undo.disabled = this.history.length === 0; details.append(undo); parent.append(details);
  }
}
const statusNames: Record<string, string> = { missing: "目标缺失", fragmentMissing: "内部片段缺失", ambiguous: "目标或片段歧义", unavailable: "目标或片段不可用", invalid: "地址无效" };
export class RelationshipDiagnosticsView {
  private page = "issues";
  private grouping = "target";
  private query = "";
  private filter = "all";
  private layout = "list";
  private showIgnored = false;
  constructor(private readonly rules: PersonalRelationRules, private readonly navigate: (node: RelationNode) => Promise<void>, private readonly locate: (issue: RelationIssue) => Promise<void>, private readonly report: (error: unknown) => void) {}
  render(parent: HTMLElement, data?: RelationDiagnostics): void {
    const section = el("section"); section.setAttribute("aria-label", "关系诊断"); section.className = "knowledge-section relationship-diagnostics"; parent.append(section);
    section.append(el("h3", "关系"));
    if (!data) { section.append(el("p", "尚未扫描：刷新后读取当前获权范围。")); return; }
    section.append(el("p", `${data.coverage === "complete" ? "本次声明范围检查完成" : "扫描不完整，以下为已知结果；不能判定全库孤立"} · 已检查 ${data.scannedNodes} 篇。范围：获权原生正文与 typed 属性引用；生成链接未获完整证明时保持部分覆盖。`));
    const tabs = el("div"); tabs.className = "relationship-tabs"; tabs.setAttribute("role", "tablist"); tabs.setAttribute("aria-label", "关系结果页");
    const results = el("div"); results.setAttribute("aria-live", "polite");
    const paint = (): void => {
      results.replaceChildren();
      if (this.page === "issues") {
        const all = data.issues.filter(i => this.showIgnored || !ruleMatches(this.rules.rules, "issue", issueRuleKey(i)));
        const issues = all.filter(i => (!this.query || `${i.source.title} ${i.source.path ?? ""} ${i.address} ${statusNames[i.status] ?? i.status}`.includes(this.query)) && (this.filter === "all" || i.status === this.filter));
        results.append(el("p", `当前可见 ${issues.length} 处 / 已知问题 ${data.issues.length} 处。`));
        if (!issues.length) results.append(el("p", this.query ? "搜索无匹配。" : this.filter !== "all" || all.length !== data.issues.length ? "筛选无结果。" : data.scannedNodes === 0 ? "空范围：没有获权笔记。" : data.coverage === "complete" ? "本次范围未发现链接问题。" : "本次已扫描部分未发现问题；检查尚未完整。"));
        for (const [, group] of diagnosticGroups(issues, this.grouping)) {
          const container = el("section"); container.append(el("h4", this.grouping === "source" ? group[0].source.title : group[0].target?.title ?? group[0].address));
          for (const issue of group) {
            const row = el("article"); row.className = "knowledge-card";
            row.append(el("p", `${issue.source.title} · ${statusNames[issue.status] ?? issue.status} · ${issue.field ? `属性 ${issue.field}，仅能打开笔记，无精确源码位置` : `UTF-16 [${issue.from}, ${issue.to})`}`), el("p", issue.address), button("定位来源", () => this.locate(issue).catch(this.report)));
            if (issue.target) row.append(el("span", "目标节点存在；片段状态独立。"), button("仅打开目标笔记", () => this.navigate(issue.target!).catch(this.report)));
            const key = issueRuleKey(issue); row.append(button("忽略此问题…", () => this.rules.preview({ kind: "issue", value: key }, `范围：来源 ${issue.source.title}（${issue.source.key}）→准确地址 ${issue.address}；本授权范围内此来源的全部相同地址引用。`, data.issues.filter(i => issueRuleKey(i) === key).length, repaint)));
            if (ruleMatches(this.rules.rules, "issue", key)) row.append(el("span", "已忽略（仍保留真实连接）")); container.append(row);
          } results.append(container);
        }
      } else {
        const nodes = data.nodes.filter(n => (!this.query || `${n.node.title} ${n.node.path ?? ""}`.includes(this.query)) && (this.filter === "noIncoming" ? n.incoming === 0 : this.filter === "expected" ? ruleMatches(this.rules.rules, "expected", n.node.key) : n.state !== "connected"));
        results.append(el("p", `当前可见 ${nodes.length} / 已检查节点 ${data.nodes.length}。无入链与真正孤立分别显示；数字仅为已知下界。`));
        if (!nodes.length) results.append(el("p", this.query ? "搜索无匹配。" : "本筛选没有已知结果。"));
        const groups = new Map<string, typeof nodes>();
        for (const node of nodes) { const key = this.layout === "tree" ? node.node.path?.replaceAll("\\", "/").split("/").slice(0, -1).join("/") || "获权节点（无目录路径）" : "结果"; const group = groups.get(key) ?? []; group.push(node); groups.set(key, group); }
        for (const [path, nodes] of groups) {
          const group = el(this.layout === "tree" ? "details" : "div"); if (group instanceof HTMLDetailsElement) { group.open = true; group.append(el("summary", path)); }
          for (const item of nodes) {
            const expected = ruleMatches(this.rules.rules, "expected", item.node.key), row = el("article"); row.className = "knowledge-card";
            const state = item.state === "isolated" ? "真正孤立（此声明范围）" : item.state === "invalidOutgoing" ? "仍有无效出链" : item.state === "incomplete" ? "扫描不完整：尚不能判孤立" : "有节点连接";
            row.append(button(item.node.title, () => this.navigate(item.node).catch(this.report)), el("p", `${state} · 入 ${item.incoming} / 出 ${item.outgoing} / 无效出链 ${item.invalidOutgoing}${item.incoming === 0 ? " · 无已知入链" : ""}${expected ? " · 预期孤立" : ""}`), button("标记预期孤立…", () => this.rules.preview({ kind: "expected", value: item.node.key }, `准确对象：${item.node.title} · ${item.node.key}${item.node.path ? ` · ${item.node.path}` : ""}`, 1, repaint))); group.append(row);
          } results.append(group);
        }
      }
    };
    const controls = el("div"), repaint = (): void => { section.remove(); this.render(parent, data); }; controls.className = "relationship-controls";
    for (const [id, label] of [["issues", "链接问题"], ["nodes", "孤立节点"]]) {
      const b = button(label, () => { this.page = id; this.filter = "all"; repaint(); parent.querySelector<HTMLElement>(`[role=tab][aria-selected=true]`)?.focus(); }); b.setAttribute("role", "tab"); b.setAttribute("aria-selected", String(id === this.page)); b.tabIndex = id === this.page ? 0 : -1;
      b.onkeydown = e => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key) && !e.isComposing) { e.preventDefault(); this.page = e.key === "Home" ? "issues" : e.key === "End" ? "nodes" : this.page === "issues" ? "nodes" : "issues"; this.filter = "all"; repaint(); parent.querySelector<HTMLElement>('[role=tab][aria-selected=true]')?.focus(); } }; tabs.append(b);
    }
    const search = el("input"); search.type = "search"; search.value = this.query; search.setAttribute("aria-label", "搜索关系诊断"); search.oninput = event => { this.query = search.value; if (!(event as InputEvent).isComposing) paint(); }; search.addEventListener("compositionend", () => { this.query = search.value; paint(); });
    controls.append(search, button("清空搜索", () => { this.query = ""; search.value = ""; paint(); search.focus(); }));
    if (this.page === "issues") controls.append(choice("问题分组", [["target", "按目标"], ["source", "按来源"], ["occurrence", "逐处引用"]], this.grouping, v => { this.grouping = v; paint(); }), choice("问题状态", [["all", "全部问题"], ...Object.entries(statusNames)], this.filter, v => { this.filter = v; paint(); }));
    else controls.append(choice("孤立筛选", [["all", "孤立／无效／不完整"], ["noIncoming", "无入链（独立筛选）"], ["expected", "预期孤立"]], this.filter, v => { this.filter = v; paint(); }), choice("孤立布局", [["list", "列表"], ["tree", "按目录分组"]], this.layout, v => { this.layout = v; paint(); }));
    const ignored = el("input"); ignored.type = "checkbox"; ignored.checked = this.showIgnored; const l = el("label", "包含已忽略问题"); l.prepend(ignored); ignored.onchange = () => { this.showIgnored = ignored.checked; paint(); }; controls.append(l);
    section.append(tabs, controls); this.rules.controls(section, repaint); section.append(results); paint();
  }
}
