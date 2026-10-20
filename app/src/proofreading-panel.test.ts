import test from "node:test";
import assert from "node:assert/strict";
import { decodeProofreadingOptions } from "./proofreading-panel";
import { decodeProcessingSettings, defaultProcessingSettings, parseProcessingLanguages } from "./processing-settings";

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
