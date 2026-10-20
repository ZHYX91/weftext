import test from "node:test";
import assert from "node:assert/strict";
import { legacyBackendChoices, selectedCaptionOptions, type CaptionChoice } from "./office-import-options";
import type { DocxImportAnalysis, LegacyCapabilities } from "./export-state";

const analysis: DocxImportAnalysis = { contentHash: "a".repeat(64), captions: [{ blockIndex: 1, sequence: "图", text: "图 1 示例", suggestedText: "示例" }, { blockIndex: 4, sequence: "图", text: "图 2 后图", suggestedText: "后图" }], objects: [{ blockIndex: 0, kind: "figure" }, { blockIndex: 3, kind: "table" }] };
const choice = (overrides: Partial<CaptionChoice> = {}): CaptionChoice => ({ enabled: true, captionBlock: 1, objectBlock: "0", kind: "figure", captionText: "已核对示例", position: "bottom", ...overrides });

test("legacy software choices are isolated by format and conversion direction", () => {
  const capabilities: LegacyCapabilities = { ordinaryDocxRequiresOffice: false, backends: [{ id: "word", version: "16", directions: [{ direction: "docxToDoc", available: true, contentRoundtripVerified: false, reason: "" }, { direction: "docToDocx", available: false, contentRoundtripVerified: false, reason: "missing" }] }, { id: "wps", directions: [{ direction: "wpsToDocx", available: true, contentRoundtripVerified: false, reason: "" }] }] };
  assert.deepEqual(legacyBackendChoices(capabilities, "doc", false), [["word", "Microsoft Word · 16"]]);
  assert.deepEqual(legacyBackendChoices(capabilities, "doc", true), []);
  assert.deepEqual(legacyBackendChoices(capabilities, "wps", true), [["wps", "WPS Office"]]);
  assert.deepEqual(legacyBackendChoices(capabilities, "wps", false), []);
  assert.deepEqual(legacyBackendChoices(null, "doc", true), []);
  assert.deepEqual(legacyBackendChoices(capabilities, "docx", true), []);
});

test("unselected caption proposals keep ordinary text and never infer nearby objects", () => {
  assert.equal(selectedCaptionOptions(analysis, [choice({ enabled: false })]), undefined);
  assert.equal(selectedCaptionOptions(null, []), undefined);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ objectBlock: "" })]), /明确选择/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ kind: "" })]), /明确选择/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ position: "" })]), /明确选择/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ captionText: " " })]), /明确选择/);
});

test("explicit caption choices freeze the original hash, exact text, object, kind and position", () => {
  const selected = choice(); const result = selectedCaptionOptions(analysis, [selected])!;
  assert.deepEqual(result, { contentHash: analysis.contentHash, captionPairings: [{ captionBlock: 1, objectBlock: 0, kind: "figure", captionText: "已核对示例", position: "bottom" }] });
  selected.captionText = "后来输入"; assert.equal(result.captionPairings[0].captionText, "已核对示例");
  assert.throws(() => selectedCaptionOptions(null, [choice()]), /已失效/);
  assert.throws(() => selectedCaptionOptions({ ...analysis, contentHash: "old" }, [choice()]), /已失效/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ kind: "table" })]), /同类类别/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice({ objectBlock: "9" })]), /明确选择/);
  assert.throws(() => selectedCaptionOptions(analysis, [choice(), choice({ captionBlock: 4 })]), /只能配对一次/);
});
