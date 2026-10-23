import test from "node:test";
import assert from "node:assert/strict";
import { beginNativeRequest, freezeNativeApi, nativeApiIdle, thawNativeApi } from "./native-api-gate";

test("native close cannot freeze an outstanding transport or admit a late request after freezing", () => {
  thawNativeApi();
  const first = beginNativeRequest(), second = beginNativeRequest();
  assert.equal(nativeApiIdle(), false);
  assert.equal(freezeNativeApi(), false);
  first(); first();
  assert.equal(nativeApiIdle(), false);
  second();
  assert.equal(freezeNativeApi(), true);
  assert.throws(() => beginNativeRequest(), error => (error as { code?: string }).code === "nativeSurfaceFrozen");
  thawNativeApi();
  const resumed = beginNativeRequest();
  assert.equal(freezeNativeApi(), false);
  resumed();
  assert.equal(nativeApiIdle(), true);
});
