import test from "node:test";
import assert from "node:assert/strict";
import { decodeProofreadingOptions } from "./proofreading-panel";
import { decodeProcessingSettings, defaultProcessingSettings, parseProcessingLanguages } from "./processing-settings";
import { decodeProofreadResult, proofreadFindingUiText, processingMatchUiText } from "./proofreading-wire";
import { locale, message, uiTextValue } from "./ui-locale";

test("proofread result has one declared product or author message and preserves original author UTF16", () => {
  const author = "作者规则\ud800\u0000 العربية", finding = { id: "one", category: "typo", ruleId: "rule", ruleVersion: "1", excerpt: "原文", replacement: null, sourceRange: { from: 0, to: 6, utf16From: 0, utf16To: 2, raw: "原文" }, executable: false };
  const raw = { coverage: "partial", findings: [{ ...finding, messageOrigin: "product", explanation: message("common.save") }, { ...finding, id: "two", messageOrigin: "author", authorMessage: author }], warnings: [message("common.unavailable")], checkedCharacters: 2, existing: [], processing: defaultProcessingSettings(), processingOrigin: null, fragments: [{ language: null, nativeLanguage: null, source: "unknown", checkedRules: [], skippedRules: [["rule", "missingDependency"]] }] }, frozen = JSON.stringify(raw);
  const parsed = decodeProofreadResult(JSON.parse(frozen), "local");
  try { locale.set("en-US"); assert.equal(uiTextValue(proofreadFindingUiText(parsed.findings[0])), "Save"); assert.equal(uiTextValue(proofreadFindingUiText(parsed.findings[1])), author); assert.match(uiTextValue(processingMatchUiText(parsed.fragments[0].skippedRules[0][1])), /dependency/); locale.set("zh-CN"); assert.equal(uiTextValue(proofreadFindingUiText(parsed.findings[0])), "保存"); assert.equal(uiTextValue(proofreadFindingUiText(parsed.findings[1])), author); } finally { locale.set("zh-CN"); }
  assert.equal(JSON.stringify(raw), frozen);
  for (const changed of [{ ...raw.findings[0], authorMessage: "second source" }, { ...finding, message: "old rendered message" }, { ...raw.findings[1], authorMessage: message("common.save") }]) assert.throws(() => decodeProofreadResult({ ...raw, findings: [changed] }, "local"));
  assert.throws(() => decodeProofreadResult({ ...raw, warnings: ["old rendered warning"] }, "local"));
  const hosted = { ...raw, scanId: "original-scan", sourceHash: "original-source", findings: raw.findings.map(value => { const copy = structuredClone(value); delete (copy.sourceRange as { raw?: string }).raw; return copy; }) };
  assert.equal(decodeProofreadResult(hosted, "hosted").scanId, "original-scan");
  assert.throws(() => decodeProofreadResult(raw, "hosted"));
  assert.throws(() => decodeProofreadResult(hosted, "local"));
});

test("portable rules reject duplicate identities and executable sensitive terms without altering prior settings", () => {
  const options = { characters: true, typos: true, pairs: true, sensitive: true, selection: [0, 3], rules: [{ id: "user.typo", version: "1", category: "typo", text: "错字", message: "核对", replacement: "正字" }], allowlist: ["许可词"] };
  const decoded = decodeProofreadingOptions(options);
  assert.equal(decoded.selection, null);
  decoded.rules[0].message = "changed";
  assert.equal(options.rules[0].message, "核对");
  assert.throws(() => decodeProofreadingOptions({ ...options, rules: [...options.rules, ...options.rules] }));
  assert.throws(() => decodeProofreadingOptions({ ...options, rules: [{ ...options.rules[0], category: "sensitive" }] }));
  assert.throws(() => decodeProofreadingOptions({ ...options, allowlist: [null] }));
});

test("legacy dictionaries remain undeclared until explicit confirmation and choices keep unknown region", () => {
  const options = { characters: true, typos: true, pairs: true, sensitive: false, rules: [{ id: "old", version: "1", category: "typo", text: "x", message: "old", replacement: "y" }], allowlist: ["x"] };
  const decoded = decodeProofreadingOptions(options);
  assert.equal(decoded.rules[0].scope, undefined);
  assert.deepEqual(decoded.allowlist, ["x"]);
  assert.deepEqual(decoded.processing?.languages, []);
  assert.deepEqual(parseProcessingLanguages("zh-Hant-HK, en"), [{ language: "zh", script: "Hant", region: "HK" }, { language: "en", script: null, region: null }]);
  assert.throws(() => parseProcessingLanguages("zh-CN-Hant"));
  assert.throws(() => decodeProcessingSettings({ ...defaultProcessingSettings(), scheme: "invoice-cn" }));
  assert.throws(() => decodeProofreadingOptions({ ...options, rules: [{ ...options.rules[0], scope: { universal: true, regions: ["CN"] } }] }));
});
