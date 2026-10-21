import test from "node:test";
import assert from "node:assert/strict";
import { thumbnailSourceBasis, thumbnailStatus } from "./list-thumbnail";

test("thumbnail exact source basis never guesses latest or converts missing to absent", () => {
  assert.equal(thumbnailSourceBasis({ state: "ready", sourceBasisKind: "observed", sourceBaseId: "original-read" }), "observed-source:original-read");
  assert.equal(thumbnailSourceBasis({ state: "ready", sourceBasisKind: "committed", sourceRevision: "original-revision" }), "original-revision");
  assert.throws(() => thumbnailSourceBasis({ state: "ready", sourceRevision: "wrong-kind" }));
  assert.throws(() => thumbnailSourceBasis({ state: "ready", sourceBasisKind: "observed", sourceRevision: "wrong-basis" }));
  assert.notEqual(thumbnailStatus({ state: "absent" }), thumbnailStatus({ state: "unavailable" }));
  assert.notEqual(thumbnailStatus({ state: "building" }), thumbnailStatus({ state: "unsupported" }));
  assert.match(thumbnailStatus({ state: "failed", message: "无法解码" }), /生成失败：无法解码/);
  assert.throws(() => thumbnailStatus({ state: "unknown" as never }));
});
