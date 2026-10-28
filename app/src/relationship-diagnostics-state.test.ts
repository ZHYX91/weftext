import { test } from "node:test";
import assert from "node:assert/strict";
import { connectivityClassification, diagnosticGroups, diagnosticPage, DIAGNOSTIC_PAGE_SIZE, issueRuleKey, projectDiagnosticIssues, projectDiagnosticNodes, sourceDirectory, type PersonalRule, type RelationConnectivity, type RelationDiagnostics, type RelationIssue, type RelationNode } from "./relationship-diagnostics-state";

const node = (key: string, title = key, path?: string): RelationNode => ({ key, title, path, reference: "weftext-node://workspace/" + key });
const connectivity = (key: string, state = "isolated", incoming = 0, outgoing = 0, invalidOutgoing = 0): RelationConnectivity => ({ node: node(key), state, incoming, outgoing, invalidOutgoing });
const snapshot = (nodes: RelationConnectivity[] = [], issues: RelationIssue[] = [], coverage: RelationDiagnostics["coverage"] = "complete"): RelationDiagnostics => ({ coverage, scannedNodes: nodes.length, nodes, issues });
const label = (value: RelationNode): string => String(value.title);
const nodeOptions = { query: "", filter: "all" as const, sort: "scan" as const, showExpected: false };
const issueOptions = { query: "", status: "all", grouping: "target" as const, sort: "scan" as const, showIgnored: false };
const issueText = { node: label, status: (value: string): string => value };

test("expected isolation hides only currently established isolation and keeps bad-link caution separate", () => {
    const data = snapshot([connectivity("expected"), connectivity("broken", "invalidOutgoing", 0, 0, 2), connectivity("review"), connectivity("outgoing", "connected", 0, 1)]);
    const rules: PersonalRule[] = [{ kind: "expected", value: "expected" }, { kind: "expected", value: "broken" }, { kind: "expected", value: "outgoing" }];
    const projection = projectDiagnosticNodes(data, rules, nodeOptions, label);
    assert.deepEqual(projection.items.map(item => item.value.node.key), ["review"]);
    assert.deepEqual([projection.expectedCount, projection.reviewCount, projection.incompleteCount], [2, 1, 0]);
    const expected = projectDiagnosticNodes(data, rules, { ...nodeOptions, filter: "expected" }, label);
    assert.deepEqual(expected.items.map(item => item.value), data.nodes.slice(0, 2));
    assert.equal(expected.items[1].value.invalidOutgoing, 2);
    const noIncoming = projectDiagnosticNodes(data, rules, { ...nodeOptions, filter: "noIncoming" }, label);
    assert.deepEqual(noIncoming.items.map(item => item.value.node.key), ["review", "outgoing"]);
    assert.equal(noIncoming.items[1].markedExpected, true);
    assert.equal(noIncoming.items[1].expected, false);
    assert.equal(noIncoming.items[1].classification, "connected");
    assert.deepEqual(projectDiagnosticNodes(data, rules, { ...nodeOptions, showExpected: true }, label).items.map(item => item.value.node.key), ["expected", "broken", "review"]);
});

test("partial scans and unknown states never turn a retained personal mark into a proven orphan", () => {
    const data = snapshot([connectivity("marked"), connectivity("incomplete", "incomplete"), connectivity("unknown", "future-state")], [], "partial");
    const rules: PersonalRule[] = [{ kind: "expected", value: "marked" }];
    const projection = projectDiagnosticNodes(data, rules, nodeOptions, label);
    assert.equal(projection.items.length, 3);
    assert.deepEqual(projection.items.map(item => item.classification), ["incomplete", "incomplete", "incomplete"]);
    assert.equal(projection.items[0].markedExpected, true);
    assert.equal(projection.items[0].expected, false);
    assert.deepEqual([projection.expectedCount, projection.reviewCount, projection.incompleteCount], [0, 0, 3]);
    assert.deepEqual(projectDiagnosticNodes(data, rules, { ...nodeOptions, filter: "expected" }, label).items, []);
    assert.equal(connectivityClassification(data.nodes[2], "complete"), "incomplete");
    assert.equal(connectivityClassification(connectivity("contradictory", "isolated", 1), "complete"), "connected");
    assert.equal(connectivityClassification(connectivity("connected", "connected"), "complete"), "connected");
});

test("node search and problem ordering use the disclosed snapshot without changing counts or source objects", () => {
    const data = snapshot([connectivity("Alpha", "invalidOutgoing", 0, 0, 1), connectivity("beta", "invalidOutgoing", 0, 0, 4), connectivity("gamma", "incomplete")]);
    const before = JSON.stringify(data);
    const result = projectDiagnosticNodes(data, [], { ...nodeOptions, query: " A ", sort: "problems" }, label);
    assert.deepEqual(result.items.map(item => item.value.node.key), ["beta", "Alpha", "gamma"]);
    assert.equal(result.items[0].value, data.nodes[1]);
    assert.deepEqual([result.reviewCount, result.incompleteCount], [2, 1]);
    assert.equal(JSON.stringify(data), before);
});

test("source-directory grouping preserves exact occurrences and distinguishes a real address from an object identity", () => {
    const first: RelationIssue = { source: node("one", "First", "C:\\library\\A\\one.typ"), address: "missing", status: "missing", from: 7, to: 9 };
    const second: RelationIssue = { source: node("two", "Second", "C:/library/A/two.typ"), address: "missing", status: "missing", from: 11, to: 13 };
    const third: RelationIssue = { source: node("three"), target: node("missing"), address: "another-address", status: "fragmentMissing", from: 19, to: 21 };
    const fourth: RelationIssue = { source: node("four", "Fourth", "C:/library/B/four.typ"), address: "missing", status: "missing", from: 23, to: 25 };
    const data = snapshot([], [first, second, third, fourth]);
    assert.equal(diagnosticGroups(data.issues, "target").length, 2);
    const projection = projectDiagnosticIssues(data, [], { ...issueOptions, grouping: "sourceFolder", sort: "problems" }, issueText);
    assert.deepEqual(projection.items.slice(0, 2), [first, second]);
    assert.equal(projection.items[0], first);
    assert.equal(new Set(projection.items).size, 4);
    assert.deepEqual(diagnosticGroups(projection.items, "sourceFolder").map(([, group]) => group.length), [2, 1, 1]);
    assert.equal(sourceDirectory("C:\\one.typ"), "C:/");
    assert.equal(sourceDirectory("/one.typ"), "/");
    assert.equal(sourceDirectory("one.typ"), undefined);
});

test("issue search and exact ignore rules preserve known totals and every source location", () => {
    const source = node("a", "Alpha", "/library/Author/a.typ");
    const issues: RelationIssue[] = [{ source, address: "missing", status: "missing", from: 1, to: 3 }, { source, address: "missing", status: "missing", from: 5, to: 7 }, { source, address: "other", status: "fragmentMissing", field: "Contact", from: 9, to: 11 }];
    const data = snapshot([], issues), before = JSON.stringify(data);
    const rules: PersonalRule[] = [{ kind: "issue", value: issueRuleKey(issues[0]) }];
    const hidden = projectDiagnosticIssues(data, rules, issueOptions, issueText);
    assert.equal(hidden.unhiddenCount, 1);
    assert.deepEqual(hidden.items, [issues[2]]);
    const searched = projectDiagnosticIssues(data, rules, { ...issueOptions, showIgnored: true, query: " AUTHOR ", status: "missing", grouping: "occurrence", sort: "name" }, issueText);
    assert.equal(searched.unhiddenCount, 3);
    assert.deepEqual(searched.items, issues.slice(0, 2));
    assert.equal(projectDiagnosticIssues(data, rules, { ...issueOptions, query: "contact" }, issueText).items[0], issues[2]);
    assert.equal(JSON.stringify(data), before);
});

test("100-row pages preserve all independently locatable issues across split groups and clamp after filtering", () => {
    const source = node("source"), issues = Array.from({ length: 205 }, (_, index): RelationIssue => ({ source, address: "same", status: "missing", from: index * 10, to: index * 10 + 4 }));
    const projected = projectDiagnosticIssues(snapshot([], issues), [], issueOptions, issueText).items;
    assert.equal(DIAGNOSTIC_PAGE_SIZE, 100);
    const pages = [0, 1, 2].map(index => diagnosticPage(projected, index));
    assert.deepEqual(pages.map(page => [page.from, page.to, page.total, page.pages]), [[1, 100, 205, 3], [101, 200, 205, 3], [201, 205, 205, 3]]);
    assert.deepEqual(pages.map(page => page.items.length), [100, 100, 5]);
    assert.deepEqual(pages.flatMap(page => page.items), issues);
    assert.equal(pages[1].items[0], issues[100]);
    assert.deepEqual(diagnosticPage(projected.slice(0, 3), 2), { items: issues.slice(0, 3), page: 0, pages: 1, total: 3, from: 1, to: 3 });
    assert.equal(diagnosticPage(projected, -1).page, 0);
    assert.equal(diagnosticPage(projected, Number.NaN).page, 0);
    assert.deepEqual(diagnosticPage([], 8), { items: [], page: 0, pages: 1, total: 0, from: 0, to: 0 });
});
