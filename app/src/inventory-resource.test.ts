import test from "node:test";
import assert from "node:assert/strict";
import { decodeInventoryResource } from "./inventory-resource-wire";

const id = "a".repeat(64);
const descriptor = () => ({ resourceId: id, resource: `weftext-inventory-resource://${id}`, revision: `observed-inventory-resource:v1:${id}`,
    name: "image.png", bytes: 16, contentType: "image/png", metadata: { kind: "image", width: 2, height: 2 }, annotationReady: false, currentness: "observed" });

test("directory resource is an observed read-only exact descriptor", () => {
    const value = decodeInventoryResource(descriptor());
    assert.equal(value.annotationReady, false); assert.equal(value.metadata.kind, "image");
});
test("directory resources cannot acquire writer identities or substitute version/uri", () => {
    for (const patch of [{ nodeId: "node" }, { readBasis: "writer" }, { annotationReady: true }, { currentness: "committed" },
        { resource: "file:///private.png" }, { revision: "observed-inventory-resource:v1:" + "b".repeat(64) },
        { bytes: 64 * 1024 * 1024 + 1 }, { metadata: { kind: "image", width: 0, height: 2 } }])
        assert.throws(() => decodeInventoryResource({ ...descriptor(), ...patch }));
});
