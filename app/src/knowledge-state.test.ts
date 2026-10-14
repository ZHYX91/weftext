import test from "node:test";
import assert from "node:assert/strict";
import { canApplySuggestion, isGraphemeRange, KnowledgeReadGate, knowledgeLossMessage, layoutMentions, nodeIdentity, retainPendingKnowledgePlan, sameKnowledgeBasis, uncertainKnowledgeOperation, validateMediaRegion } from "./knowledge-state";
import type { KnowledgeBasis, KnowledgeMention, MediaRegionInput } from "./knowledge-state";

const mention = (start: number, end: number, targets: string[]): KnowledgeMention => ({ startScalar: start, endScalar: end, startUtf16: start, endUtf16: end, targets: targets.map(nodeId => ({ workspaceId: "w", nodeId })), terms: [`${start}:${end}`], writable: true, component: -1 });

test("same-span multiple targets, nested and crossing mentions remain reachable with only two visual lanes", () => {
  const hits = layoutMentions([mention(2, 3, ["C"]), mention(0, 3, ["B"]), mention(2, 5, ["D"]), mention(0, 3, ["A", "B"])]);
  assert.equal(hits.length, 3);
  assert.deepEqual(hits.map(hit => [hit.startScalar, hit.endScalar, hit.lane]), [[0, 3, 0], [2, 5, 1], [2, 3, undefined]]);
  assert.deepEqual(hits[0].targets.map(nodeIdentity), ["w/A", "w/B"]);
  assert.equal(hits.flatMap(hit => hit.targets).length, 4);
  assert.deepEqual(hits.map(hit => hit.component), [0, 0, 0]);
});

test("touching endpoints can reuse lanes; connected dense overlaps never discard a candidate", () => {
  const dense = Array.from({ length: 400 }, (_, index) => mention(index, 1000 - index, [String(index)]));
  const hits = layoutMentions([...dense, mention(1000, 1001, ["tail"])]);
  assert.equal(hits.length, 401);
  assert.equal(hits.filter(hit => hit.component === 0 && hit.lane !== undefined).length, 2);
  assert.equal(hits.filter(hit => hit.component === 0 && hit.lane === undefined).length, 398);
  assert.equal(hits.at(-1)?.lane, 0);
  assert.equal(hits.at(-1)?.component, 1);
});

test("inconsistent same-span source mappings retain discovery but cannot acquire shortcut write permission", () => {
  const a = mention(0, 3, ["A"]), b = { ...mention(0, 3, ["B"]), endUtf16: 4 };
  const hit = layoutMentions([a, b])[0];
  assert.equal(hit.writable, false);
  assert.equal(hit.targets.length, 2);
});

const basis: KnowledgeBasis = { sessionId: "s", path: "a.typ", source: "原文", serial: 0, baseId: "r1" };
test("local draft refusal never invents an unknown Core operation; actual network uncertainty retains its ID", () => {
  assert.equal(retainPendingKnowledgePlan(true, new Error("reading mode")), false);
  assert.equal(retainPendingKnowledgePlan(true, new Error("draft persistence failed after applying")), false);
  assert.equal(retainPendingKnowledgePlan(false, new Error("connection lost")), true);
  assert.equal(retainPendingKnowledgePlan(false, { code: "resultUnknown" }), true);
  assert.equal(retainPendingKnowledgePlan(false, { code: "conflict" }), false);
});
test("a late background response cannot replace a newer refresh or a switched draft", () => {
  const gate = new KnowledgeReadGate(); const first = gate.begin(), second = gate.begin();
  assert.equal(gate.accepts(first, basis, basis), false);
  assert.equal(gate.accepts(second, basis, { ...basis, path: "b.typ" }), false);
  assert.equal(gate.accepts(second, basis, { ...basis, serial: 2 }), false);
  assert.equal(gate.accepts(second, basis, { ...basis, baseId: "r2" }), false);
  assert.equal(gate.accepts(second, basis, basis), true);
  gate.invalidate(); assert.equal(gate.accepts(second, basis, basis), false);
});

test("equal source bytes after edits or save are insufficient to reuse a prepared draft basis", () => {
  assert.equal(sameKnowledgeBasis(basis, { ...basis, serial: 2 }), false);
  assert.equal(sameKnowledgeBasis(basis, { ...basis, baseId: "r2" }), false);
  assert.equal(sameKnowledgeBasis(null, basis), false);
});

test("selection anchors reject partial combining characters, emoji sequences and CRLF", () => {
  const text = "甲e\u0301👩‍🔬\r\n乙";
  assert.equal(isGraphemeRange(text, 1, 2), false);
  assert.equal(isGraphemeRange(text, 1, 3), true);
  assert.equal(isGraphemeRange(text, 3, 5), false);
  assert.equal(isGraphemeRange(text, 3, 8), true);
  assert.equal(isGraphemeRange(text, 8, 9), false);
  assert.equal(isGraphemeRange(text, 8, 10), true);
  assert.equal(isGraphemeRange(text, 3, 3), false);
  assert.equal(isGraphemeRange(text, 3, 3, true), true);
  assert.equal(isGraphemeRange(text, 2, 2, true), false);
});

test("stale, rejected and unverified suggestions preserve discussion but cannot expose accept action", () => {
  const suggestion = { purpose: "suggestion", suggestionState: "pending", targetResolution: "exact" as const };
  assert.equal(canApplySuggestion(suggestion), true);
  assert.equal(canApplySuggestion({ ...suggestion, targetResolution: "mapped" }), true);
  assert.equal(canApplySuggestion({ ...suggestion, targetResolution: "stale" }), false);
  assert.equal(canApplySuggestion({ ...suggestion, suggestionState: "accepted" }), false);
  assert.equal(canApplySuggestion({ ...suggestion, verified: false }), false);
});

const image: MediaRegionInput = { kind: "image_region", resource: "image.png", revision: "exact", width: 1200, height: 800, x: 1100, y: 100, rectWidth: 100, rectHeight: 200 };
test("media region validation preserves coordinate units and rejects rectangles outside the resource", () => {
  assert.equal(validateMediaRegion(image), null);
  assert.notEqual(validateMediaRegion({ ...image, rectWidth: 101 }), null);
  assert.notEqual(validateMediaRegion({ ...image, x: -1 }), null);
  assert.notEqual(validateMediaRegion({ ...image, revision: "" }), null);
  assert.equal(validateMediaRegion({ ...image, kind: "pdf_region", pageIndex: 0 }), null);
  assert.notEqual(validateMediaRegion({ ...image, kind: "pdf_region", pageIndex: -1 }), null);
});

test("audio and video ticks are exact integers and video retains both time and frame region", () => {
  const audio: MediaRegionInput = { kind: "media_interval", mediaType: "audio", resource: "sound.wav", revision: "r", startTick: 1000, endTick: 2000, timebase: 48000 };
  assert.equal(validateMediaRegion(audio), null);
  assert.notEqual(validateMediaRegion({ ...audio, endTick: 1000 }), null);
  assert.notEqual(validateMediaRegion({ ...audio, startTick: 1.5 }), null);
  assert.notEqual(validateMediaRegion({ ...audio, timebase: Number.MAX_SAFE_INTEGER + 1 }), null);
  assert.notEqual(validateMediaRegion({ ...audio, durationTick: 1999 }), null);
  assert.notEqual(validateMediaRegion({ ...audio, timebaseDenominator: 0 }), null);
  assert.equal(validateMediaRegion({ ...audio, durationTick: 2000, timebaseDenominator: 1 }), null);
  assert.notEqual(validateMediaRegion({ ...audio, mediaType: "video" }), null);
  assert.equal(validateMediaRegion({ ...image, ...audio, mediaType: "video" }), null);
});

test("recovery pending is an uncertain original operation, while a definite conflict is terminal", () => {
  assert.equal(uncertainKnowledgeOperation("recoveryPending"), true);
  assert.equal(uncertainKnowledgeOperation("pending_reconciliation"), true);
  assert.equal(uncertainKnowledgeOperation("resultUnknown"), true);
  assert.equal(uncertainKnowledgeOperation("conflict"), false);
  assert.equal(uncertainKnowledgeOperation("committed"), false);
});

test("typed Core losses expose their actual human message in the confirmation flow", () => {
  assert.equal(knowledgeLossMessage({ code: "unknownReference", message: "此引用未能迁移，已保存准确原件。" }), "此引用未能迁移，已保存准确原件。");
  assert.equal(knowledgeLossMessage("原文保留"), "原文保留");
});
