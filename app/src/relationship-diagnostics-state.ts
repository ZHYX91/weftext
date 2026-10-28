import type { UiText } from "./ui-locale";

/** Personal presentation only; none of these projections change scan authority or edges. */
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
export interface RelationConnectivity {
    node: RelationNode;
    incoming: number;
    outgoing: number;
    invalidOutgoing: number;
    state: string;
}
export interface RelationDiagnostics {
    generation?: string;
    coverage: "complete" | "partial";
    scannedNodes: number;
    issues: RelationIssue[];
    nodes: RelationConnectivity[];
}
export interface PersonalRule {
    kind: "issue" | "expected" | "term" | "target";
    value: string;
    source?: string;
}
export type DiagnosticGrouping = "target" | "source" | "sourceFolder" | "occurrence";
export type DiagnosticSort = "scan" | "name" | "problems";
export type IsolationFilter = "all" | "noIncoming" | "expected";
export const DIAGNOSTIC_PAGE_SIZE = 100;

export const issueRuleKey = (issue: RelationIssue): string => JSON.stringify([issue.source.key, issue.address]);
export function ruleMatches(rules: readonly PersonalRule[], kind: PersonalRule["kind"], value: string, source?: string): boolean {
    return rules.some(r => r.kind === kind && r.value === value && (!r.source || r.source === source));
}
export function visibleCandidate(rules: readonly PersonalRule[], source: string, term: string, target: string): boolean {
    return !ruleMatches(rules, "term", term, source) && !ruleMatches(rules, "target", target, source);
}
/** Display grouping of already disclosed paths; this never resolves or reads a path. */
export function sourceDirectory(path: string | undefined): string | undefined {
    if (path === undefined)
        return undefined;
    const display = path.replaceAll("\\", "/"), slash = display.lastIndexOf("/");
    if (slash < 0)
        return undefined;
    if (slash === 0 || slash === 2 && /^[a-z]:/i.test(display))
        return display.slice(0, slash + 1);
    return display.slice(0, slash);
}
export function diagnosticGroups(issues: readonly RelationIssue[], grouping: DiagnosticGrouping): [string, RelationIssue[]][] {
    const groups = new Map<string, RelationIssue[]>();
    for (const [index, issue] of issues.entries()) {
        // Tuple namespaces keep a real address from colliding with an object key.
        const key = grouping === "source" ? JSON.stringify(["source", issue.source.key])
            : grouping === "sourceFolder" ? JSON.stringify(["folder", sourceDirectory(issue.source.path) ?? null])
                : grouping === "occurrence" ? JSON.stringify(["occurrence", index])
                    : JSON.stringify(issue.target ? ["target", issue.target.key] : ["address", issue.address]);
        const group = groups.get(key) ?? [];
        group.push(issue);
        groups.set(key, group);
    }
    return [...groups];
}
const searchText = (value: string): string => value.toLocaleLowerCase();
export function projectDiagnosticIssues(data: RelationDiagnostics, rules: readonly PersonalRule[], options: {
    query: string;
    status: string;
    grouping: DiagnosticGrouping;
    sort: DiagnosticSort;
    showIgnored: boolean;
}, text: { node: (node: RelationNode) => string; status: (status: string) => string }): {
    items: RelationIssue[];
    unhiddenCount: number;
} {
    const unhidden = data.issues.filter(i => options.showIgnored || !ruleMatches(rules, "issue", issueRuleKey(i)));
    const query = searchText(options.query.trim());
    const matching = unhidden.filter(i => (!query || searchText(`${text.node(i.source)} ${i.source.path ?? ""} ${i.address} ${i.field ?? ""} ${text.status(i.status)}`).includes(query))
        && (options.status === "all" || i.status === options.status));
    const groups = diagnosticGroups(matching, options.grouping);
    const name = (issue: RelationIssue): string => options.grouping === "source" ? text.node(issue.source)
        : options.grouping === "sourceFolder" ? sourceDirectory(issue.source.path) ?? ""
            : options.grouping === "occurrence" ? `${text.node(issue.source)} ${issue.address}`
                : issue.target ? text.node(issue.target) : issue.address;
    if (options.sort !== "scan")
        groups.sort((a, b) => (options.sort === "problems" ? b[1].length - a[1].length : 0)
            || name(a[1][0]).localeCompare(name(b[1][0])));
    // Original objects and every independent source occurrence survive sorting.
    return { items: groups.flatMap(([, items]) => items), unhiddenCount: unhidden.length };
}

export type ConnectivityClassification = "connected" | "isolated" | "incomplete";
export function connectivityClassification(item: RelationConnectivity, coverage: RelationDiagnostics["coverage"]): ConnectivityClassification {
    if (item.incoming > 0 || item.outgoing > 0 || item.state === "connected")
        return "connected";
    if (coverage !== "complete" || item.state === "incomplete")
        return "incomplete";
    return item.state === "isolated" || item.state === "invalidOutgoing" ? "isolated" : "incomplete";
}
export interface DiagnosticNodeItem {
    value: RelationConnectivity;
    classification: ConnectivityClassification;
    markedExpected: boolean;
    expected: boolean;
}
export function projectDiagnosticNodes(data: RelationDiagnostics, rules: readonly PersonalRule[], options: {
    query: string;
    filter: IsolationFilter;
    sort: DiagnosticSort;
    showExpected: boolean;
}, label: (node: RelationNode) => string): {
    items: DiagnosticNodeItem[];
    expectedCount: number;
    reviewCount: number;
    incompleteCount: number;
} {
    const all = data.nodes.map(value => {
        const classification = connectivityClassification(value, data.coverage);
        const markedExpected = ruleMatches(rules, "expected", value.node.key);
        return { value, classification, markedExpected, expected: markedExpected && classification === "isolated" };
    });
    const query = searchText(options.query.trim());
    const items = all.filter(item => (!query || searchText(`${label(item.value.node)} ${item.value.node.path ?? ""}`).includes(query))
        && (options.filter === "expected" ? item.expected
            : (options.filter === "noIncoming" ? item.value.incoming === 0 : item.classification !== "connected")
                && (options.showExpected || !item.expected)));
    if (options.sort !== "scan")
        items.sort((a, b) => (options.sort === "problems" ? b.value.invalidOutgoing - a.value.invalidOutgoing : 0)
            || label(a.value.node).localeCompare(label(b.value.node)));
    return {
        items,
        expectedCount: all.filter(item => item.expected).length,
        reviewCount: all.filter(item => item.classification === "isolated" && !item.expected).length,
        incompleteCount: all.filter(item => item.classification === "incomplete").length,
    };
}

export function diagnosticPage<T>(items: readonly T[], requestedPage: number): {
    items: T[];
    page: number;
    pages: number;
    total: number;
    from: number;
    to: number;
} {
    const pages = Math.max(1, Math.ceil(items.length / DIAGNOSTIC_PAGE_SIZE));
    const page = Math.max(0, Math.min(pages - 1, Number.isFinite(requestedPage) ? Math.floor(requestedPage) : 0));
    const start = page * DIAGNOSTIC_PAGE_SIZE;
    return { items: items.slice(start, start + DIAGNOSTIC_PAGE_SIZE), page, pages, total: items.length, from: items.length ? start + 1 : 0, to: Math.min(items.length, start + DIAGNOSTIC_PAGE_SIZE) };
}
