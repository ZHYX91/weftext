import test from "node:test";
import assert from "node:assert/strict";
import { decodeReviewPreview, decodeReviewReceipt } from "./hosted-review-wire";
import { locale, message, uiTextValue } from "./ui-locale";

test("review preview is a typed safe projection with an independent frozen annotation language", () => {
    const body = "作者批注\ud800\u0000 العربية", raw = { summary: message("common.save"), kind: "annotationProofread", body, annotationLocale: "en-US", count: 2 }, original = JSON.stringify(raw);
    const preview = decodeReviewPreview(JSON.parse(original));
    try { locale.set("zh-CN"); assert.equal(uiTextValue(preview.summary!), "保存"); locale.set("en-US"); assert.equal(uiTextValue(preview.summary!), "Save"); assert.equal(preview.body, body); assert.equal(preview.annotationLocale, "en-US"); } finally { locale.set("zh-CN"); }
    assert.equal(JSON.stringify(raw), original);
    for (const invalid of [{ ...raw, summary: "old rendered label" }, { ...raw, _review: {} }, { ...raw, annotationLocale: { "en-US": null } }, { ...raw, count: -1 }]) assert.throws(() => decodeReviewPreview(invalid));
});
test("review receipts preserve exact original resource, operation and commit admission", () => {
    const receipt = { operationId: "original", resource: "owner", status: "prepared", planId: "plan", preview: { summary: message("common.save") }, commitAllowed: true };
    assert.equal(decodeReviewReceipt(structuredClone(receipt), "owner", "original").planId, "plan");
    assert.equal(decodeReviewReceipt({ ...receipt, status: "committed", planId: null, commitAllowed: false, preview: {} }, "owner", "original").status, "committed");
    for (const changed of [{ ...receipt, operationId: "another" }, { ...receipt, resource: "another" }, { ...receipt, planId: null }, { ...receipt, status: "success" }, { ...receipt, message: "legacy" }, { ...receipt, status: "committed" }]) assert.throws(() => decodeReviewReceipt(changed, "owner", "original"));
});
