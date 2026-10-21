import test from "node:test";
import assert from "node:assert/strict";
import { enUS, zhCN } from "./ui-messages";
import { LocaleScope, locale, message, normalizeUiLocale, parseUiMessage, renderUiMessage, t, type UiLocale } from "./ui-locale";

test("interface templates preserve every semantic parameter in both delivered languages", () => {
  for (const key of Object.keys(zhCN) as (keyof typeof zhCN)[]) {
    const parameters = (value: string) => [...value.matchAll(/\{([A-Za-z][A-Za-z0-9_]*)\}/g)].map(match => match[1]).sort();
    assert.deepEqual(parameters(enUS[key]), parameters(zhCN[key]), key);
  }
});

test("unknown locale and missing translation fall back without changing parameters", () => {
  const original = { operationId: "原号<{count}>😀" };
  const spec = message("common.operation", original);
  original.operationId = "new value";
  assert.equal(renderUiMessage("unknown", spec), "原操作：原号<{count}>😀");
  assert.equal(renderUiMessage("en-US", spec, {}), "原操作：原号<{count}>😀");
  assert.equal(renderUiMessage("en-US", spec), "Original operation: 原号<{count}>😀");
  assert.equal(normalizeUiLocale("en"), "zh-CN");
});

test("live labels and attributes change without replacing nodes or altering control values", () => {
  locale.set("zh-CN");
  const scope = new LocaleScope();
  const text = { textContent: "" } as Text;
  const values = new Map<string, string>();
  const input = { value: "作者文字\r\n😀", setAttribute(name: string, value: string) { values.set(name, value); } };
  scope.bindText(text, message("common.save"));
  scope.bindAttr(input as unknown as Element, "aria-label", message("capture.body"));
  const sameInput = input, sameText = text;
  locale.set("en-US");
  assert.equal(text.textContent, "Save");
  assert.equal(values.get("aria-label"), "Body");
  assert.equal(input.value, "作者文字\r\n😀");
  assert.equal(input, sameInput); assert.equal(text, sameText);
  scope.dispose();
  locale.set("zh-CN");
  assert.equal(text.textContent, "Save");
  assert.equal(values.get("aria-label"), "Body");
});

test("an explicit dynamic message keeps current state while a disposed binding cannot revive it", () => {
  locale.set("zh-CN");
  const scope = new LocaleScope();
  const target = { textContent: "" } as Text;
  let count = 1;
  const unbind = scope.bindText(target, () => message("reminder.missed", { count }));
  count = 2;
  locale.set("en-US");
  assert.equal(target.textContent, "Valid missed reminders: 2.");
  unbind(); count = 3; scope.refresh();
  assert.equal(target.textContent, "Valid missed reminders: 2.");
  scope.dispose();
  assert.throws(() => scope.bindText(target, message("common.close")), /disposed/);
  assert.throws(() => locale.set("ar" as UiLocale), /Unsupported/);
  locale.set("zh-CN");
  assert.equal(t(message("common.close")), "关闭");
});

test("a faulty retired listener cannot stop other live labels from updating", () => {
  locale.set("zh-CN");
  const scope = new LocaleScope(), target = { textContent: "" } as Text;
  const unsubscribe = locale.subscribe(() => { throw new Error("retired component"); });
  scope.bindText(target, message("common.save"));
  assert.throws(() => locale.set("en-US"), AggregateError);
  assert.equal(target.textContent, "Save");
  unsubscribe(); scope.dispose(); locale.set("zh-CN");
});

test("message wire rejects raw diagnostics, unexpected keys and inexact numeric parameters", () => {
  assert.deepEqual(parseUiMessage({ key: "common.operation", params: { operationId: "original" } }), message("common.operation", { operationId: "original" }));
  for (const value of [
    { key: "common.save", params: { source: "protected" } },
    { key: "common.save", params: { "": "unexpected" } },
    { key: "common.operation", params: {} },
    { key: "common.save", params: {}, extra: true },
    { key: "unknown.message", params: {} },
    { key: "reminder.missed", params: { count: 0.5 } },
    { key: "reminder.missed", params: { count: Number.MAX_SAFE_INTEGER + 1 } },
    { key: "reminder.missed", params: { count: 1n } },
    { key: "common.operation", params: { operationId: "\ud800" } },
    "raw diagnostic",
  ]) assert.equal(parseUiMessage(value), undefined);
  assert.throws(() => message("common.operation", { operationId: "\ud800" }), /Invalid product/);
  assert.throws(() => message("reminder.missed", { count: Infinity }), /Invalid product/);
  assert.throws(() => message("reminder.missed", { count: 1.5 }), /Invalid product/);
  const safe = message("reminder.missed", { count: Number.MAX_SAFE_INTEGER });
  assert.deepEqual(parseUiMessage(JSON.parse(JSON.stringify(safe))), safe);
});

test("a failed initial binding leaves no retained callback or locale subscription", () => {
  locale.set("zh-CN");
  const scope = new LocaleScope(), target = { textContent: "" } as Text;
  let attempts = 0;
  assert.throws(() => scope.bindText(target, () => { attempts++; throw new Error("initial failure"); }), /initial failure/);
  assert.doesNotThrow(() => locale.set("en-US"));
  assert.equal(attempts, 1);
  scope.bindText(target, message("common.save"));
  assert.equal(target.textContent, "Save");
  scope.dispose(); locale.set("zh-CN");
});
