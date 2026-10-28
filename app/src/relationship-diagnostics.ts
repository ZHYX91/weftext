import { uiDisplayParameter, message as uiMessage, displayMessage, setUiText, setUiAttr, uiTextValue, type UiText } from "./ui-locale";
import { diagnosticGroups, diagnosticPage, issueRuleKey, projectDiagnosticIssues, projectDiagnosticNodes, ruleMatches, sourceDirectory, type DiagnosticGrouping, type DiagnosticSort, type IsolationFilter, type PersonalRule, type RelationDiagnostics, type RelationIssue, type RelationNode } from "./relationship-diagnostics-state";
export { diagnosticGroups, issueRuleKey, ruleMatches, visibleCandidate, type PersonalRule, type RelationDiagnostics, type RelationIssue, type RelationNode } from "./relationship-diagnostics-state";
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
    private view: "issues" | "nodes" = "issues";
    private grouping: DiagnosticGrouping = "target";
    private query = "";
    private issueStatus = "all";
    private nodeFilter: IsolationFilter = "all";
    private issueSort: DiagnosticSort = "scan";
    private nodeSort: DiagnosticSort = "scan";
    private issuePage = 0;
    private nodePage = 0;
    private layout: "list" | "tree" = "list";
    private showIgnored = false;
    private showExpected = false;
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
        if (data.generation)
            section.dataset.diagnosticGeneration = data.generation;
        section.append(el("p", displayMessage("relationshipDiagnostics.scanScopeSummary", { p0: uiDisplayParameter(data.coverage === "complete" ? uiMessage("relationshipDiagnostics.declaredScopeComplete") : uiMessage("relationshipDiagnostics.scanIncompleteIsolationUnproven")), p1: data.scannedNodes })));
        const tabs = el("div");
        tabs.className = "relationship-tabs";
        tabs.setAttribute("role", "tablist");
        setUiAttr(tabs, "aria-label", uiMessage("relationshipDiagnostics.relationshipResultTabs"));
        const results = el("div");
        results.setAttribute("aria-live", "polite");
        const appendPage = (page: ReturnType<typeof diagnosticPage>): void => {
            if (!page.total)
                return;
            const nav = el("nav");
            nav.className = "relationship-pagination";
            setUiAttr(nav, "aria-label", uiMessage("relationshipDiagnostics.resultPages"));
            const previous = button(uiMessage("relationshipDiagnostics.previousPage"), () => {
                if (this.view === "issues") this.issuePage--; else this.nodePage--;
                paint();
            });
            const next = button(uiMessage("relationshipDiagnostics.nextPage"), () => {
                if (this.view === "issues") this.issuePage++; else this.nodePage++;
                paint();
            });
            previous.dataset.diagnosticPage = "previous";
            next.dataset.diagnosticPage = "next";
            previous.disabled = page.page === 0;
            next.disabled = page.page + 1 === page.pages;
            nav.append(previous, el("span", displayMessage("relationshipDiagnostics.pageRange", { p0: page.from, p1: page.to, p2: page.total })), next);
            results.append(nav);
        };
        const paint = (): void => {
            const active = document.activeElement;
            const pageFocus = active instanceof HTMLElement && results.contains(active) ? active.dataset.diagnosticPage : undefined;
            results.replaceChildren();
            if (this.view === "issues") {
                const projection = projectDiagnosticIssues(data, this.rules.rules, { query: this.query, status: this.issueStatus, grouping: this.grouping, sort: this.issueSort, showIgnored: this.showIgnored }, { node: node => uiTextValue(node.title), status: status => uiTextValue(statusNames[status] ?? status) });
                const page = diagnosticPage(projection.items, this.issuePage);
                this.issuePage = page.page;
                results.append(el("p", displayMessage("relationshipDiagnostics.visibleKnownIssueCounts", { p0: projection.items.length, p1: data.issues.length })));
                if (!projection.items.length)
                    results.append(el("p", this.query ? uiMessage("relationshipDiagnostics.searchNoMatch") : this.issueStatus !== "all" || projection.unhiddenCount !== data.issues.length ? uiMessage("relationshipDiagnostics.filterNoResults") : data.scannedNodes === 0 ? uiMessage("relationshipDiagnostics.emptyAuthorizedScope") : data.coverage === "complete" ? uiMessage("relationshipDiagnostics.noLinkIssuesInScope") : uiMessage("relationshipDiagnostics.partialScanNoIssues")));
                for (const [, group] of diagnosticGroups(page.items, this.grouping)) {
                    const container = el("section");
                    const heading = this.grouping === "source" ? group[0].source.title
                        : this.grouping === "sourceFolder" ? sourceDirectory(group[0].source.path) ?? uiMessage("relationshipDiagnostics.authorizedNodeNoDirectory")
                            : group[0].target?.title ?? group[0].address;
                    container.append(el("h4", heading));
                    for (const issue of group) {
                        const row = el("article");
                        row.className = "knowledge-card";
                        row.append(el("p", () => [uiTextValue(issue.source.title), uiTextValue(statusNames[issue.status] ?? issue.status), uiTextValue(issue.field ? displayMessage("relationshipDiagnostics.propertyOwnerNoExactSource", { p0: uiDisplayParameter(issue.field) }) : "UTF-16 [" + issue.from + ", " + issue.to + ")")].join(" · ")), el("p", issue.address), button(uiMessage("relationshipDiagnostics.locateSource"), () => this.locate(issue).catch(this.report)));
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
                appendPage(page);
            }
            else {
                const projection = projectDiagnosticNodes(data, this.rules.rules, { query: this.query, filter: this.nodeFilter, sort: this.nodeSort, showExpected: this.showExpected }, node => uiTextValue(node.title));
                const page = diagnosticPage(projection.items, this.nodePage);
                this.nodePage = page.page;
                results.append(el("p", displayMessage("relationshipDiagnostics.visibleCheckedNodeCounts", { p0: projection.items.length, p1: data.nodes.length })), el("p", displayMessage("relationshipDiagnostics.isolationSummary", { p0: projection.expectedCount, p1: projection.reviewCount, p2: projection.incompleteCount })));
                if (!projection.items.length)
                    results.append(el("p", this.query ? uiMessage("relationshipDiagnostics.searchNoMatch") : uiMessage("relationshipDiagnostics.filterNoKnownResults")));
                const groups = new Map<string | undefined, typeof page.items>();
                for (const item of page.items) {
                    const key = this.layout === "tree" ? sourceDirectory(item.value.node.path) : undefined;
                    const group = groups.get(key) ?? [];
                    group.push(item);
                    groups.set(key, group);
                }
                for (const [path, nodes] of groups) {
                    const group = el(this.layout === "tree" ? "details" : "div");
                    if (group instanceof HTMLDetailsElement) {
                        group.open = true;
                        group.append(el("summary", path ?? uiMessage("relationshipDiagnostics.authorizedNodeNoDirectory")));
                    }
                    for (const projected of nodes) {
                        const item = projected.value, row = el("article");
                        row.className = "knowledge-card";
                        const state = projected.classification === "incomplete" ? uiMessage("relationshipDiagnostics.stateIncompleteIsolationUnproven")
                            : projected.classification === "connected" ? uiMessage("relationshipDiagnostics.stateConnected")
                                : item.state === "invalidOutgoing" ? uiMessage("relationshipDiagnostics.stateInvalidOutgoing")
                                    : uiMessage("relationshipDiagnostics.stateActuallyIsolated");
                        row.append(button(item.node.title, () => this.navigate(item.node).catch(this.report)), el("p", displayMessage("relationshipDiagnostics.nodeConnectionSummary", { p0: uiDisplayParameter(state), p1: item.incoming, p2: item.outgoing, p3: item.invalidOutgoing, p4: uiDisplayParameter(item.incoming === 0 ? uiMessage("relationshipDiagnostics.noKnownIncomingSuffix") : ""), p5: uiDisplayParameter(projected.expected ? uiMessage("relationshipDiagnostics.expectedIsolationSuffix") : "") })));
                        if (projected.markedExpected && !projected.expected)
                            row.append(el("p", uiMessage("relationshipDiagnostics.expectedMarkRetained")));
                        const mark = button(uiMessage("relationshipDiagnostics.markExpectedIsolation"), () => this.rules.preview({ kind: "expected", value: item.node.key }, displayMessage("relationshipDiagnostics.exactObjectDescription", { p0: uiDisplayParameter(item.node.title), p1: uiDisplayParameter(item.node.key), p2: uiDisplayParameter(item.node.path ? " · " + item.node.path : "") }), 1, repaint));
                        mark.disabled = projected.classification !== "isolated" || projected.markedExpected;
                        if (projected.classification !== "isolated")
                            setUiAttr(mark, "title", uiMessage("relationshipDiagnostics.expectedNeedsIsolated"));
                        row.append(mark);
                        group.append(row);
                    }
                    results.append(group);
                }
                appendPage(page);
            }
            if (pageFocus) {
                const controls = [...results.querySelectorAll<HTMLButtonElement>("[data-diagnostic-page]")];
                (controls.find(control => control.dataset.diagnosticPage === pageFocus && !control.disabled)
                    ?? controls.find(control => !control.disabled))?.focus();
            }
        };
        const controls = el("div"), repaint = (): void => { section.remove(); this.render(parent, data); };
        const refilter = (): void => { this.issuePage = 0; this.nodePage = 0; paint(); };
        controls.className = "relationship-controls";
        const focusTab = (): void => { parent.querySelector<HTMLElement>('.relationship-diagnostics [role=tab][aria-selected=true]')?.focus(); };
        for (const [id, label] of [["issues", uiMessage("relationshipDiagnostics.tabLinkIssues")], ["nodes", uiMessage("relationshipDiagnostics.tabIsolatedNodes")]] as const) {
            const b = button(label, () => { this.view = id; repaint(); focusTab(); });
            b.setAttribute("role", "tab");
            b.setAttribute("aria-selected", String(id === this.view));
            b.tabIndex = id === this.view ? 0 : -1;
            b.onkeydown = e => {
                if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key) && !e.isComposing) {
                    e.preventDefault();
                    this.view = e.key === "Home" ? "issues" : e.key === "End" ? "nodes" : this.view === "issues" ? "nodes" : "issues";
                    repaint();
                    focusTab();
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
                refilter();
        };
        search.addEventListener("compositionend", () => { this.query = search.value; refilter(); });
        controls.append(search, button(uiMessage("relationshipDiagnostics.clearSearch"), () => { this.query = ""; search.value = ""; refilter(); search.focus(); }));
        const sortOptions: [string, UiText][] = [["scan", uiMessage("relationshipDiagnostics.sortScanOrder")], ["name", uiMessage("relationshipDiagnostics.sortName")], ["problems", uiMessage("relationshipDiagnostics.sortProblemCount")]];
        if (this.view === "issues") {
            controls.append(choice(uiMessage("relationshipDiagnostics.issueGrouping"), [["target", uiMessage("relationshipDiagnostics.groupByTarget")], ["source", uiMessage("relationshipDiagnostics.groupBySource")], ["sourceFolder", uiMessage("relationshipDiagnostics.groupBySourceDirectory")], ["occurrence", uiMessage("relationshipDiagnostics.groupByOccurrence")]], this.grouping, v => { this.grouping = v as DiagnosticGrouping; refilter(); }), choice(uiMessage("relationshipDiagnostics.issueStatus"), [["all", uiMessage("relationshipDiagnostics.allIssues")], ...Object.entries(statusNames)], this.issueStatus, v => { this.issueStatus = v; refilter(); }), choice(uiMessage("relationshipDiagnostics.resultSort"), sortOptions, this.issueSort, v => { this.issueSort = v as DiagnosticSort; refilter(); }));
            const ignored = el("input");
            ignored.type = "checkbox";
            ignored.checked = this.showIgnored;
            const label = el("label", uiMessage("relationshipDiagnostics.includeIgnoredIssues"));
            label.prepend(ignored);
            ignored.onchange = () => { this.showIgnored = ignored.checked; refilter(); };
            controls.append(label);
        }
        else {
            const expected = el("input");
            expected.type = "checkbox";
            expected.checked = this.showExpected;
            expected.disabled = this.nodeFilter === "expected";
            controls.append(choice(uiMessage("relationshipDiagnostics.isolationFilter"), [["all", uiMessage("relationshipDiagnostics.isolatedInvalidIncomplete")], ["noIncoming", uiMessage("relationshipDiagnostics.noIncomingIndependentFilter")], ["expected", uiMessage("relationshipDiagnostics.ruleExpectedIsolation")]], this.nodeFilter, v => { this.nodeFilter = v as IsolationFilter; expected.disabled = this.nodeFilter === "expected"; refilter(); }), choice(uiMessage("relationshipDiagnostics.isolationLayout"), [["list", uiMessage("queryView.list")], ["tree", uiMessage("relationshipDiagnostics.groupByDirectory")]], this.layout, v => { this.layout = v as "list" | "tree"; refilter(); }), choice(uiMessage("relationshipDiagnostics.resultSort"), sortOptions, this.nodeSort, v => { this.nodeSort = v as DiagnosticSort; refilter(); }));
            const label = el("label", uiMessage("relationshipDiagnostics.includeExpected"));
            label.prepend(expected);
            expected.onchange = () => { this.showExpected = expected.checked; refilter(); };
            controls.append(label);
        }
        section.append(tabs, controls);
        this.rules.controls(section, repaint);
        section.append(results);
        paint();
    }
}
