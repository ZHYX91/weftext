import test from "node:test";
import assert from "node:assert/strict";
import { reviewInputOwner } from "./review-input-owner";

test("source annotation input does not inherit a retained media proposal identity", () => {
    const retained = { asset: "new-media", revision: "new-version" };
    for (const target of ["selection", "whole"]) assert.deepEqual(reviewInputOwner("document", target, retained), { kind: "review", resource: "document" });
    for (const target of ["media", "mediaWhole"]) assert.deepEqual(reviewInputOwner("document", target, retained), { kind: "review", resource: "document", ...retained });
});

test("reply ownership comes from its own verified thread, independently of the new annotation target", () => {
    const selected = { asset: "unrelated-media", revision: "unrelated-version" }, source = { id: "source-thread" }, media = { id: "media-thread", resourceLocation: { asset: "actual-media", revision: "actual-version" } };
    for (const target of ["selection", "whole", "media", "mediaWhole"]) {
        assert.deepEqual(reviewInputOwner("document", target, selected, source), { kind: "review", resource: "document", annotationId: "source-thread" });
        assert.deepEqual(reviewInputOwner("document", target, selected, media), { kind: "review", resource: "document", annotationId: "media-thread", asset: "actual-media", revision: "actual-version" });
    }
    assert.deepEqual(reviewInputOwner("document", "media", selected, { id: "unverified-media" }), { kind: "review", resource: "document", annotationId: "unverified-media" });
});
