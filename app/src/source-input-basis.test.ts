import test from "node:test";
import assert from "node:assert/strict";
import type { DocumentBasis } from "./native-value-form";
import { sameSourceInput, sourceInputBasis, sourceInputKey } from "./source-input-basis";

const original: DocumentBasis = { inputId: "author", sessionId: "session", source: "原文😀\r\n", serial: 3,
    baseId: "base", path: "library/A.typ", root: "library", workspaceId: "workspace", nodeId: "node", dirty: true, inputRevision: 7 };

test("a retained source read/proposal copies only its complete author identity, never document authority", () => {
    const document = { ...original, operationId: "original-save", evidence: "authorDraft" }, captured = sourceInputBasis(document);
    assert.deepEqual(Object.keys(captured).sort(), ["inputId", "sessionId", "source", "serial", "baseId", "path", "root", "workspaceId", "nodeId"].sort());
    assert.equal(sameSourceInput(captured, document), true);
    for (const inputRevision of [undefined, 8, 99]) {
        const acknowledged = { ...document, inputRevision, dirty: false };
        assert.equal(sameSourceInput(acknowledged, captured), true);
        assert.equal(sourceInputKey(acknowledged), sourceInputKey(captured));
        assert.deepEqual(sourceInputBasis(acknowledged), captured);
    }
    document.inputId = "replacement author with equal bytes";
    assert.equal(captured.inputId, original.inputId);
    assert.equal(sameSourceInput(document, captured), false);
    assert.equal(original.inputRevision, 7);
});

test("every source/route/owner member fences the original read or proposal including same-byte ABA", () => {
    const captured = sourceInputBasis(original), key = sourceInputKey(captured);
    for (const field of ["inputId", "sessionId", "source", "serial", "baseId", "path", "root", "workspaceId", "nodeId"] as const) {
        const changed = { ...captured, [field]: field === "serial" ? captured.serial + 2 : `${captured[field]}-changed` };
        assert.equal(sameSourceInput(changed, captured), false, field);
        assert.notEqual(sourceInputKey(changed), key, field);
    }
    assert.equal(sameSourceInput({ ...captured, serial: captured.serial + 2 }, captured), false, "Undo returning to the old input ID/bytes retains the later serial");
    assert.equal(sameSourceInput(null, captured), false, "unavailable/revoked sources cannot keep a lease");
});
