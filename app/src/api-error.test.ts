import test from "node:test";
import assert from "node:assert/strict";
import { ApiError, decodeApiError } from "./api-error";
import { errorUiText, locale, message, uiTextValue } from "./ui-locale";

test("typed errors preserve code, raw diagnostics, conflict content and details while changing only product explanation", () => {
  const source = '作者原文\r\n😀', text = 'native original <script> & 中文\ud800', details = { kind: "currentSource", currentSource: source };
  locale.set("zh-CN");
  const error = decodeApiError({ code: "sameMachineCode", explanation: { key: "common.operation", params: { operationId: "original-operation" } }, diagnostics: [{ origin: "os", text }], causes: [{ code: "underlyingCode", explanation: { key: "common.unavailable", params: {} }, diagnostics: [{ origin: "dependency", text: "original dependency" }] }], details })!;
  assert.ok(error instanceof ApiError); assert.equal(error.currentSource, source); assert.deepEqual(error.details, details);
  const before = uiTextValue(errorUiText(error)); locale.set("en-US"); const after = uiTextValue(errorUiText(error));
  assert.match(before, /原操作：original-operation/); assert.match(after, /Original operation: original-operation/);
  assert.ok(before.includes(text) && after.includes(text)); assert.equal(error.diagnostics[0].text, text); assert.equal(error.code, "sameMachineCode"); assert.deepEqual(error.details, details); assert.equal(error.currentSource, source);
  locale.set("zh-CN");
});

test("error wire has no message-string fallback, unknown origin or extra field protocol", () => {
  const valid = { code: "failure", explanation: { key: "common.unavailable", params: {} }, diagnostics: [], causes: [] };
  for (const value of [{ code: "failure", message: "old product string" }, { ...valid, extra: true }, { ...valid, currentSource: "wrong location" }, { ...valid, diagnostics: [{ origin: "browserGuess", text: "raw" }] }, { ...valid, diagnostics: [{ origin: "os", text: "raw", explanation: "guessed" }] }, { ...valid, explanation: { key: "common.save", params: { source: "private" } } }, { ...valid, details: { currentSource: null } }]) assert.equal(decodeApiError(value), undefined);
});

test("causes are required, bounded, flat and retain their exact typed explanation", () => {
  const cause = { code: "inner", explanation: { key: "common.unavailable", params: {} }, diagnostics: [] }, valid = { code: "outer", explanation: { key: "error.invalidResponse", params: {} }, diagnostics: [], causes: [cause] };
  assert.equal(decodeApiError(valid)?.causes[0].code, "inner");
  for (const value of [{ ...valid, causes: undefined }, { ...valid, causes: Array(33).fill(cause) }, { ...valid, causes: [{ ...cause, causes: [] }] }, { ...valid, causes: [{ ...cause, currentSource: "wrong owner" }] }]) assert.equal(decodeApiError(value), undefined);
});

test("the same machine code can carry distinct actual product explanations without text matching", () => {
  const first = new ApiError("sharedEvidenceCode", message("common.unavailable")), second = new ApiError("sharedEvidenceCode", message("error.invalidResponse"));
  assert.equal(first.code, second.code); assert.notEqual(first.message, second.message); assert.notEqual(first.explanation.key, second.explanation.key);
});
