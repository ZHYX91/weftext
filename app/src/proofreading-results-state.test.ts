import test from "node:test";
import assert from "node:assert/strict";
import { ProofreadingResultsState } from "./proofreading-results-state";
import { defaultProcessingSettings } from "./processing-settings";
import type { ProofreadFinding, ProofreadResult } from "./proofreading-wire";

function report(count: number): ProofreadResult {
    const findings: ProofreadFinding[] = Array.from({ length: count }, (_, index) => ({ id: `issue-${index}`, category: index % 2 ? "character" : "typo", ruleId: index % 2 ? "rule-a" : "rule-b", ruleVersion: "1", excerpt: `作者原文 ${index}`, messageOrigin: "author", authorMessage: "请核对", replacement: null, executable: false, sourceRange: { from: index * 2, to: index * 2 + 1, utf16From: index * 2, utf16To: index * 2 + 1, raw: "x" } }));
    return { coverage: "complete", findings, warnings: [], checkedCharacters: count * 2, existing: [], processing: defaultProcessingSettings(), processingOrigin: null, fragments: [] };
}

test("a 450-issue report loads in bounded batches without mutating or discarding its original findings", () => {
    const original = report(450), snapshot = JSON.stringify(original), state = new ProofreadingResultsState();
    state.replace(original);
    assert.equal(state.visible().length, 200);
    assert.equal(state.remaining, 250);
    assert.equal(state.selectedIds().length, 200);
    const first = [...state.visible()];
    state.loadMore();
    assert.equal(state.visible().length, 400);
    assert.deepEqual(state.visible().slice(0, 200), first);
    assert.equal(state.selectedIds().length, 200);
    assert.equal(state.select(state.visible()[201], true), false);
    state.loadMore();
    assert.equal(state.visible().length, 450);
    assert.equal(state.remaining, 0);
    state.loadMore();
    assert.equal(state.visible().length, 450);
    assert.equal(JSON.stringify(original), snapshot);
});

test("explicit visible selection never silently splits more than 200 author findings", () => {
    const original = report(450), state = new ProofreadingResultsState();
    state.replace(original); state.loadMore(); state.loadMore(); state.clearSelection();
    for (const finding of state.visible().slice(200, 400)) assert.equal(state.select(finding, true), true);
    assert.deepEqual(state.selectedIds(), original.findings.slice(200, 400).map(finding => finding.id));
    assert.equal(state.select(original.findings[400], true), false);
    assert.equal(state.selectedIds().length, 200);
    state.selectVisible();
    assert.deepEqual(state.selectedIds(), original.findings.slice(0, 200).map(finding => finding.id));
    assert.equal(state.select(original.findings[0], false), true);
    assert.equal(state.select(original.findings[400], true), true);
    assert.equal(state.selectedIds().length, 200);
});

test("filtering and ordering cannot leave hidden selections in an author submission", () => {
    const original = report(450), state = new ProofreadingResultsState(); state.replace(original);
    state.setCategory("character");
    assert.equal(state.matching().length, 225);
    assert.equal(state.visible().length, 200);
    assert.equal(state.selectedIds().length, 100);
    assert.ok(state.selectedIds().every(id => Number(id.slice(6)) % 2 === 1));
    state.selectVisible();
    assert.equal(state.selectedIds().length, 200);
    state.setCategory("all");
    assert.equal(state.visible().length, 200);
    assert.equal(state.selectedIds().length, 100);
    state.setSort("rule");
    assert.ok(state.visible().every(finding => finding.ruleId === "rule-a"));
    assert.equal(state.selectedIds().length, 100);
    assert.equal(state.select(original.findings[0], true), false);
    assert.deepEqual(original.findings.map(finding => finding.id), Array.from({ length: 450 }, (_, index) => `issue-${index}`));
});

test("recording a batch updates the same report without selecting or submitting the next batch", () => {
    const original = report(450), state = new ProofreadingResultsState(); state.replace(original); state.loadMore();
    const submitted = state.selectedIds();
    original.existing.push(...submitted);
    state.reconcileSelection();
    assert.deepEqual(state.selectedIds(), []);
    assert.equal(state.eligible(original.findings[0]), false);
    assert.equal(state.recorded(original.findings[0]), true);
    state.selectVisible();
    assert.deepEqual(state.selectedIds(), original.findings.slice(200, 400).map(finding => finding.id));
});

test("unlocated and already recorded findings stay visible without becoming executable selections", () => {
    const original = report(4); original.findings[0].sourceRange = null; original.existing.push("issue-2");
    const state = new ProofreadingResultsState(); state.replace(original);
    assert.deepEqual(state.visible().map(finding => finding.id), ["issue-1", "issue-2", "issue-3", "issue-0"]);
    assert.deepEqual(state.selectedIds(), ["issue-1", "issue-3"]);
    assert.equal(state.select(original.findings[0], true), false);
    assert.equal(state.select(original.findings[2], true), false);
    state.replace(null);
    assert.equal(state.total, 0); assert.equal(state.visible().length, 0); assert.deepEqual(state.selectedIds(), []);
    state.replace(report(1));
    assert.deepEqual(state.selectedIds(), ["issue-0"]);
});
