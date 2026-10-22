import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Personal presentation only. No rules alter the authorized scan or real edges. */
export interface RelationNode {
    key: string;
    title: UiText;
    path?: string;
    reference: string;
}
export interface RelationIssue {
    source: RelationNode;
    target?: RelationNode;
    address: string;
    status: string;
    from?: number;
    to?: number;
    field?: string;
    sourceHash?: string;
}
export interface RelationDiagnostics {
    coverage: "complete" | "partial";
    scannedNodes: number;
    issues: RelationIssue[];
    nodes: {
        node: RelationNode;
        incoming: number;
        outgoing: number;
        invalidOutgoing: number;
        state: string;
    }[];
}
export interface PersonalRule {
    kind: "issue" | "expected" | "term" | "target";
    value: string;
    source?: string;
}
export const issueRuleKey = (issue: RelationIssue): string => JSON.stringify([issue.source.key, issue.address]);
export function ruleMatches(rules: PersonalRule[], kind: PersonalRule["kind"], value: string, source?: string): boolean {
    return rules.some(r => r.kind === kind && r.value === value && (!r.source || r.source === source));
}
export function visibleCandidate(rules: PersonalRule[], source: string, term: string, target: string): boolean {
    return !ruleMatches(rules, "term", term, source) && !ruleMatches(rules, "target", target, source);
}
export function diagnosticGroups(issues: RelationIssue[], grouping: string): [
    string,
    RelationIssue[]
][] {
    const groups = new Map<string, RelationIssue[]>();
    for (const [index, issue] of issues.entries()) {
        const key = grouping === "source" ? issue.source.key : grouping === "occurrence" ? String(index) : issue.target?.key ?? issue.address;
        const group = groups.get(key) ?? [];
        group.push(issue);
        groups.set(key, group);
    }
    return [...groups];
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: UiText): HTMLElementTagNameMap[K] {
    const v = document.createElement(tag);
    if (text !== undefined)
        setUiText(v, text);
    return v;
}
function button(text: UiText, run: () => void | Promise<void>): HTMLButtonElement { const b = el("button", text); b.type = "button"; b.className = "knowledge-button"; b.onclick = () => { void run(); }; return b; }
function choice(label: UiText, options: [
    string,
    UiText
][], value: string, changed: (value: string) => void): HTMLLabelElement {
    const l = el("label", label), s = el("select");
    setUiAttr(s, "aria-label", label);
    for (const [value, text] of options) {
        const o = el("option", text);
        o.value = value;
        s.append(o);
    }
    s.value = value;
    s.onchange = () => changed(s.value);
    l.append(s);
    return l;
}
export class PersonalRelationRules {
    rules: PersonalRule[] = [];
    private previews = new Set<() => void>();
    closePreviews(): void {
        for (const close of [...this.previews])
            close();
    }
    private history: PersonalRule[][] = [];
    constructor(private readonly persist?: (rules: PersonalRule[]) => void, initial?: string | null) {
        try {
            const value: unknown = JSON.parse(initial ?? "[]");
            if (Array.isArray(value))
                this.rules = value.filter((r): r is PersonalRule => !!r && ["issue", "expected", "term", "target"].includes(r.kind) && typeof r.value === "string" && (r.source === undefined || typeof r.source === "string"));
        }
        catch { /* Broken preferences never affect scan authority. */ }
    }
    save(next: PersonalRule[]): void { this.persist?.(next); this.history.push(this.rules); this.rules = next; }
    undo(): void {
        const next = this.history.at(-1);
        if (!next)
            return;
        this.persist?.(next);
        this.history.pop();
        this.rules = next;
    }
    preview(rule: PersonalRule, label: UiText, matches: number, changed: () => void): void {
        const dialog = el("dialog"), error = el("p");
        dialog.className = "knowledge-dialog";
        setUiAttr(dialog, "aria-label", uiMessage("relationshipDiagnostics.personalRulePreview"));
        dialog.append(el("h3", uiMessage("relationshipDiagnostics.personalRulePreview")), el("p", label), el("p", displayMessage("relationshipDiagnostics.personalRulePreviewSummary", { p0: matches })), error);
        const trigger = document.activeElement as HTMLElement | null;
        const close = (): void => {
            this.previews.delete(close);
            dialog.close();
            dialog.remove();
            if (trigger?.isConnected)
                trigger.focus();
        };
        this.previews.add(close);
        dialog.append(button(uiMessage("common.cancel"), close), button(uiMessage("relationshipDiagnostics.savePersonalRule"), () => {
            try {
                this.save([...this.rules.filter(r => JSON.stringify(r) !== JSON.stringify(rule)), rule]);
                close();
                changed();
            }
            catch {
                setUiText(error, uiMessage("relationshipDiagnostics.ruleSaveFailedPreviewRetained"));
            }
        }));
        dialog.addEventListener("cancel", event => { event.preventDefault(); close(); });
        document.body.append(dialog);
        dialog.showModal();
    }
    controls(parent: HTMLElement, changed: () => void): void {
        const details = el("details");
        details.append(el("summary", displayMessage("relationshipDiagnostics.personalRuleCount", { p0: this.rules.length })));
        for (const rule of this.rules) {
            const row = el("p", () => `${uiTextValue(({ issue: uiMessage("relationshipDiagnostics.ruleIgnoreIssue"), expected: uiMessage("relationshipDiagnostics.ruleExpectedIsolation"), term: uiMessage("relationshipDiagnostics.ruleIgnoreTerm"), target: uiMessage("relationshipDiagnostics.ruleExcludeTarget") })[rule.kind])} · ${uiTextValue(rule.source ? uiMessage("knowledge.scopeThisNote") : uiMessage("relationshipDiagnostics.scopeAuthorizedRange"))}`);
            row.append(el("code", rule.value), button(uiMessage("relationshipDiagnostics.removeRule"), () => {
                try {
                    this.save(this.rules.filter(r => r !== rule));
                    changed();
                }
                catch {
                    row.append(el("span", uiMessage("relationshipDiagnostics.preferenceSaveFailedRetry")));
                }
            }));
            details.append(row);
        }
        const undo = button(uiMessage("relationshipDiagnostics.undoPreferenceChange"), () => {
            try {
                this.undo();
                changed();
            }
            catch {
                details.append(el("p", uiMessage("relationshipDiagnostics.preferenceSaveFailedOriginalRetained")));
            }
        });
        undo.disabled = this.history.length === 0;
        details.append(undo);
        parent.append(details);
    }
}
const statusNames: Record<string, UiText> = { missing: uiMessage("knowledge.targetMissing"), fragmentMissing: uiMessage("relationshipDiagnostics.issueMissingInternalFragment"), ambiguous: uiMessage("relationshipDiagnostics.issueAmbiguousTargetOrFragment"), unavailable: uiMessage("relationshipDiagnostics.issueUnavailableTargetOrFragment"), invalid: uiMessage("knowledge.addressInvalid") };
export class RelationshipDiagnosticsView {
    private page = "issues";
    private grouping = "target";
    private query = "";
    private filter = "all";
    private layout = "list";
    private showIgnored = false;
    constructor(private readonly rules: PersonalRelationRules, private readonly navigate: (node: RelationNode) => Promise<void>, private readonly locate: (issue: RelationIssue) => Promise<void>, private readonly report: (error: unknown) => void) { }
    render(parent: HTMLElement, data?: RelationDiagnostics): void {
        const section = el("section");
        setUiAttr(section, "aria-label", uiMessage("relationshipDiagnostics.relationshipDiagnostics"));
        section.className = "knowledge-section relationship-diagnostics";
        parent.append(section);
        section.append(el("h3", uiMessage("knowledge.relationships")));
        if (!data) {
            section.append(el("p", uiMessage("relationshipDiagnostics.notScannedRefresh")));
            return;
        }
        section.append(el("p", displayMessage("relationshipDiagnostics.scanScopeSummary", { p0: uiDisplayParameter(data.coverage === "complete" ? uiMessage("relationshipDiagnostics.declaredScopeComplete") : uiMessage("relationshipDiagnostics.scanIncompleteIsolationUnproven")), p1: data.scannedNodes })));
        const tabs = el("div");
        tabs.className = "relationship-tabs";
        tabs.setAttribute("role", "tablist");
        setUiAttr(tabs, "aria-label", uiMessage("relationshipDiagnostics.relationshipResultTabs"));
        const results = el("div");
        results.setAttribute("aria-live", "polite");
        const paint = (): void => {
            results.replaceChildren();
            if (this.page === "issues") {
                const all = data.issues.filter(i => this.showIgnored || !ruleMatches(this.rules.rules, "issue", issueRuleKey(i)));
                const issues = all.filter(i => (!this.query || `${uiTextValue(i.source.title)} ${uiTextValue(i.source.path ?? "")} ${uiTextValue(i.address)} ${uiTextValue(statusNames[i.status] ?? i.status)}`.includes(this.query)) && (this.filter === "all" || i.status === this.filter));
                results.append(el("p", displayMessage("relationshipDiagnostics.visibleKnownIssueCounts", { p0: issues.length, p1: data.issues.length })));
                if (!issues.length)
                    results.append(el("p", this.query ? uiMessage("relationshipDiagnostics.searchNoMatch") : this.filter !== "all" || all.length !== data.issues.length ? uiMessage("relationshipDiagnostics.filterNoResults") : data.scannedNodes === 0 ? uiMessage("relationshipDiagnostics.emptyAuthorizedScope") : data.coverage === "complete" ? uiMessage("relationshipDiagnostics.noLinkIssuesInScope") : uiMessage("relationshipDiagnostics.partialScanNoIssues")));
                for (const [, group] of diagnosticGroups(issues, this.grouping)) {
                    const container = el("section");
                    container.append(el("h4", this.grouping === "source" ? group[0].source.title : group[0].target?.title ?? group[0].address));
                    for (const issue of group) {
                        const row = el("article");
                        row.className = "knowledge-card";
                        row.append(el("p", () => `${uiTextValue(issue.source.title)} · ${uiTextValue(statusNames[issue.status] ?? issue.status)} · ${uiTextValue(issue.field ? displayMessage("relationshipDiagnostics.propertyOwnerNoExactSource", { p0: uiDisplayParameter(issue.field) }) : `UTF-16 [${issue.from}, ${issue.to})`)}`), el("p", issue.address), button(uiMessage("relationshipDiagnostics.locateSource"), () => this.locate(issue).catch(this.report)));
                        if (issue.target)
                            row.append(el("span", uiMessage("relationshipDiagnostics.targetExistsFragmentIndependent")), button(uiMessage("relationshipDiagnostics.openTargetNoteOnly"), () => this.navigate(issue.target!).catch(this.report)));
                        const key = issueRuleKey(issue);
                        row.append(button(uiMessage("relationshipDiagnostics.ignoreThisIssue"), () => this.rules.preview({ kind: "issue", value: key }, displayMessage("relationshipDiagnostics.ignoreIssueScopeDescription", { p0: uiDisplayParameter(issue.source.title), p1: uiDisplayParameter(issue.source.key), p2: uiDisplayParameter(issue.address) }), data.issues.filter(i => issueRuleKey(i) === key).length, repaint)));
                        if (ruleMatches(this.rules.rules, "issue", key))
                            row.append(el("span", uiMessage("relationshipDiagnostics.ignoredActualConnectionRetained")));
                        container.append(row);
                    }
                    results.append(container);
                }
            }
            else {
                const nodes = data.nodes.filter(n => (!this.query || `${uiTextValue(n.node.title)} ${uiTextValue(n.node.path ?? "")}`.includes(this.query)) && (this.filter === "noIncoming" ? n.incoming === 0 : this.filter === "expected" ? ruleMatches(this.rules.rules, "expected", n.node.key) : n.state !== "connected"));
                results.append(el("p", displayMessage("relationshipDiagnostics.visibleCheckedNodeCounts", { p0: nodes.length, p1: data.nodes.length })));
                if (!nodes.length)
                    results.append(el("p", this.query ? uiMessage("relationshipDiagnostics.searchNoMatch") : uiMessage("relationshipDiagnostics.filterNoKnownResults")));
                const groups = new Map<string, typeof nodes>();
                for (const node of nodes) {
                    const key = this.layout === "tree" ? node.node.path?.replaceAll("\\", "/").split("/").slice(0, -1).join("/") || "@no-directory" : "@results";
                    const group = groups.get(key) ?? [];
                    group.push(node);
                    groups.set(key, group);
                }
                for (const [path, nodes] of groups) {
                    const group = el(this.layout === "tree" ? "details" : "div");
                    if (group instanceof HTMLDetailsElement) {
                        group.open = true;
                        group.append(el("summary", path === "@no-directory" ? uiMessage("relationshipDiagnostics.authorizedNodeNoDirectory") : path === "@results" ? uiMessage("relationshipDiagnostics.resultsGroup") : path));
                    }
                    for (const item of nodes) {
                        const expected = ruleMatches(this.rules.rules, "expected", item.node.key), row = el("article");
                        row.className = "knowledge-card";
                        const state = item.state === "isolated" ? uiMessage("relationshipDiagnostics.stateActuallyIsolated") : item.state === "invalidOutgoing" ? uiMessage("relationshipDiagnostics.stateInvalidOutgoing") : item.state === "incomplete" ? uiMessage("relationshipDiagnostics.stateIncompleteIsolationUnproven") : uiMessage("relationshipDiagnostics.stateConnected");
                        row.append(button(item.node.title, () => this.navigate(item.node).catch(this.report)), el("p", displayMessage("relationshipDiagnostics.nodeConnectionSummary", { p0: uiDisplayParameter(state), p1: item.incoming, p2: item.outgoing, p3: item.invalidOutgoing, p4: uiDisplayParameter(item.incoming === 0 ? uiMessage("relationshipDiagnostics.noKnownIncomingSuffix") : ""), p5: uiDisplayParameter(expected ? uiMessage("relationshipDiagnostics.expectedIsolationSuffix") : "") })), button(uiMessage("relationshipDiagnostics.markExpectedIsolation"), () => this.rules.preview({ kind: "expected", value: item.node.key }, displayMessage("relationshipDiagnostics.exactObjectDescription", { p0: uiDisplayParameter(item.node.title), p1: uiDisplayParameter(item.node.key), p2: uiDisplayParameter(item.node.path ? ` · ${item.node.path}` : "") }), 1, repaint)));
                        group.append(row);
                    }
                    results.append(group);
                }
            }
        };
        const controls = el("div"), repaint = (): void => { section.remove(); this.render(parent, data); };
        controls.className = "relationship-controls";
        for (const [id, label] of [["issues", uiMessage("relationshipDiagnostics.tabLinkIssues")], ["nodes", uiMessage("relationshipDiagnostics.tabIsolatedNodes")]] as const) {
            const b = button(label, () => { this.page = id; this.filter = "all"; repaint(); parent.querySelector<HTMLElement>(`[role=tab][aria-selected=true]`)?.focus(); });
            b.setAttribute("role", "tab");
            b.setAttribute("aria-selected", String(id === this.page));
            b.tabIndex = id === this.page ? 0 : -1;
            b.onkeydown = e => {
                if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key) && !e.isComposing) {
                    e.preventDefault();
                    this.page = e.key === "Home" ? "issues" : e.key === "End" ? "nodes" : this.page === "issues" ? "nodes" : "issues";
                    this.filter = "all";
                    repaint();
                    parent.querySelector<HTMLElement>('[role=tab][aria-selected=true]')?.focus();
                }
            };
            tabs.append(b);
        }
        const search = el("input");
        search.type = "search";
        search.value = this.query;
        setUiAttr(search, "aria-label", uiMessage("relationshipDiagnostics.searchDiagnostics"));
        search.oninput = event => {
            this.query = search.value;
            if (!(event as InputEvent).isComposing)
                paint();
        };
        search.addEventListener("compositionend", () => { this.query = search.value; paint(); });
        controls.append(search, button(uiMessage("relationshipDiagnostics.clearSearch"), () => { this.query = ""; search.value = ""; paint(); search.focus(); }));
        if (this.page === "issues")
            controls.append(choice(uiMessage("relationshipDiagnostics.issueGrouping"), [["target", uiMessage("relationshipDiagnostics.groupByTarget")], ["source", uiMessage("relationshipDiagnostics.groupBySource")], ["occurrence", uiMessage("relationshipDiagnostics.groupByOccurrence")]] as const, this.grouping, v => { this.grouping = v; paint(); }), choice(uiMessage("relationshipDiagnostics.issueStatus"), [["all", uiMessage("relationshipDiagnostics.allIssues")], ...Object.entries(statusNames)], this.filter, v => { this.filter = v; paint(); }));
        else
            controls.append(choice(uiMessage("relationshipDiagnostics.isolationFilter"), [["all", uiMessage("relationshipDiagnostics.isolatedInvalidIncomplete")], ["noIncoming", uiMessage("relationshipDiagnostics.noIncomingIndependentFilter")], ["expected", uiMessage("relationshipDiagnostics.ruleExpectedIsolation")]] as const, this.filter, v => { this.filter = v; paint(); }), choice(uiMessage("relationshipDiagnostics.isolationLayout"), [["list", uiMessage("queryView.list")], ["tree", uiMessage("relationshipDiagnostics.groupByDirectory")]] as const, this.layout, v => { this.layout = v; paint(); }));
        const ignored = el("input");
        ignored.type = "checkbox";
        ignored.checked = this.showIgnored;
        const l = el("label", uiMessage("relationshipDiagnostics.includeIgnoredIssues"));
        l.prepend(ignored);
        ignored.onchange = () => { this.showIgnored = ignored.checked; paint(); };
        controls.append(l);
        section.append(tabs, controls);
        this.rules.controls(section, repaint);
        section.append(results);
        paint();
    }
}
