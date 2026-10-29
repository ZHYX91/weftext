import test from "node:test";
import assert from "node:assert/strict";
import { history, isolateHistory, undo, undoDepth } from "@codemirror/commands";
import { createSourceState } from "./editor";
import { DocumentSession } from "./session";
import type { DocumentBasis } from "./native-value-form";
import { message, UiError } from "./ui-locale";
import { writingRange, type WritingMap } from "./writing";
import {
    checkLocalResourceSizes, decodeLocalResourceInsertion, decodeLocalResourceLocator, decodeLocalResourcePlan,
    localResourceBatchBytes, localResourceKey, readLocalResourceLocator, removeLocalResourceLocator,
    resourceBasisKey, resourceDigest, resourceOwner, resourceSourceBasis, sameResourceOwner, writeLocalResourceLocator,
    type LocalResourceLocator, type LocalResourceMode, type LocalResourcePlan, type LocalResourceStorage,
} from "./resource-import-batch-state";

const basis = (): DocumentBasis => ({ inputId: "input", sessionId: "session", source: "正文😀\r\n#endnotes()", serial: 3, baseId: "base", path: "D:/Library/Note/Note.typ", root: "D:/Library", workspaceId: "workspace-a", nodeId: "node-a", inputRevision: 7 });
const request = (): LocalResourceLocator => ({
    version: 1, owner: resourceOwner(basis()), operationId: "11111111-1111-4111-8111-111111111111", requestKind: "operation", wireDigest: "a".repeat(64),
    files: [{ name: "第一图😀.png", fileName: "原始图片.png", bytes: 3, sha256: "b".repeat(64) }, { name: "原件.txt", fileName: "原件.txt", bytes: 0, sha256: "c".repeat(64) }],
    mode: "autoAppend", insertion: "pending", phase: "original",
});
function wirePlan(locator = request(), status: LocalResourcePlan["status"] = "prepared") {
    const results = locator.files.map((file, i) => ({ name: file.name, resourceRef: `weftext-resource://${locator.owner.workspaceId}/22222222-2222-4222-8222-${String(i + 1).padStart(12, "0")}`, exactRevision: `exact-revision-${i}` }));
    return { operation: { operationId: locator.operationId, planId: "original-plan", kind: "importResources", status,
        changes: [...locator.files.map(file => ({ path: `D:/Library/Note/${file.name}`, action: "import", bytes: file.bytes })), { path: "D:/Library/Note/weftext.resources.json", action: "import", bytes: 300 }],
        nodeMap: { "node-a": "node-a" }, newNodeId: null, revisions: { "D:/Library/Note/原件.txt": "exact-revision-1" }, resourceMap: { "D:/Library/Note/原件.txt": results[1].resourceRef }, losses: [] }, imports: results };
}
const clone = <T>(value: T): T => structuredClone(value);
function storage() {
    const values = new Map<string, string>();
    const port: LocalResourceStorage = { getItem: key => values.get(key) ?? null, setItem: (key, value) => { values.set(key, value); }, removeItem: key => { values.delete(key); } };
    return { port, values };
}
function nativeMap(source: string, hash: string, at = source.length): WritingMap {
    const role = () => ({ status: "absent", text: null, range: null, construct: null, encoding: null });
    return { sourceHash: hash, syntaxValid: true, literalRegions: [{ range: writingRange(source, 0, 2), text: source.slice(0, 2), role: "body", paragraphBreaks: [] }], structures: [], title: role(), subtitle: role(), bodyInsertion: writingRange(source, at, at), unavailableReason: null,
        notes: { sourceHash: hash, syntaxValid: true, entries: [] }, headingNumbering: { sourceHash: hash, settings: null, editable: true, frozenEndnotesRequired: false, reason: null }, bodyBlocks: [] };
}
async function insertionFixture() {
    const locator = request(), document = basis(), sourceHash = await resourceDigest(document.source), at = 4;
    const imports = wirePlan(locator, "committed"), plan = decodeLocalResourcePlan(imports, locator), originalMap = nativeMap(document.source, sourceHash, at);
    const insert = `\n\n#link("${plan.imports[0].resourceRef}")[第一图😀.png]\n\n#link("${plan.imports[1].resourceRef}")[原件.txt]\n\n`;
    const next = document.source.slice(0, at) + insert + document.source.slice(at), nextHash = await resourceDigest(next);
    const value = { operationId: locator.operationId, imports: clone(plan.imports), edit: { source: next, sourceHash: nextHash, serial: document.serial,
        patches: [{ from: originalMap.bodyInsertion!.from, to: originalMap.bodyInsertion!.to, utf16From: at, utf16To: at, expected: "", insert }], selection: { from: at + insert.length, to: at + insert.length }, map: nativeMap(next, nextHash) } };
    const decode = (incoming: unknown = value, current = locator, currentPlan = plan, currentBasis = document, hash = sourceHash, map = originalMap) => decodeLocalResourceInsertion(incoming, current, currentPlan, currentBasis, hash, map);
    return { locator, document, sourceHash, at, insert, next, value, plan, originalMap, decode };
}

test("resource source cuts retain the author identity separately from Draft ACKs", () => {
    const original = basis(), owner = resourceOwner(original);
    assert.deepEqual(owner, { workspaceId: original.workspaceId, nodeId: original.nodeId, root: original.root, path: original.path });
    assert.equal(sameResourceOwner(owner, { ...owner }), true);
    for (const key of ["workspaceId", "nodeId", "root", "path"] as const) {
        const changed = { ...owner, [key]: `${owner[key]}-other` };
        assert.equal(sameResourceOwner(owner, changed), false);
        if (key === "workspaceId" || key === "nodeId") assert.notEqual(localResourceKey(owner), localResourceKey(changed));
        else assert.equal(localResourceKey(owner), localResourceKey(changed), "a moved owner retains the original locator slot");
    }
    assert.equal(sameResourceOwner(owner, null), false);
    for (const patch of [{ workspaceId: undefined }, { nodeId: undefined }, { workspaceId: "" }, { path: "" }]) assert.throws(() => resourceOwner({ ...original, ...patch }), UiError);
    for (const key of ["inputId", "sessionId", "source", "serial", "baseId", "path", "root", "workspaceId", "nodeId"] as const) {
        const current = original[key], changed = typeof current === "number" ? current + 1 : `${current} changed`;
        assert.notEqual(resourceBasisKey(original), resourceBasisKey({ ...original, [key]: changed }));
    }
    for (const inputRevision of [undefined, original.inputRevision! + 1]) {
        const acknowledged = { ...original, inputRevision };
        assert.equal(resourceBasisKey(original), resourceBasisKey(acknowledged));
        assert.deepEqual(resourceSourceBasis(original), resourceSourceBasis(acknowledged));
    }
    assert.equal(Object.hasOwn(resourceSourceBasis(original), "inputRevision"), false);
    assert.equal(original.inputRevision, 7, "the acknowledged basis is not mutated or treated as a source-edit grant");
});

test("real session repeated Draft acknowledgements leave the captured resource author cut intact", () => {
    const original = basis();
    const session = new DocumentSession({ sessionId: original.sessionId, path: original.path, root: original.root,
        source: original.source, baseId: original.baseId, kind: "managed", nodeId: original.nodeId, workspaceId: original.workspaceId,
        viewInputId: original.inputId });
    const current = (): DocumentBasis => ({ ...original, inputId: session.inputId, source: session.source, serial: session.serial,
        baseId: session.baseId, path: session.path, inputRevision: session.acknowledgedInputRevision ?? undefined });
    const captured = resourceSourceBasis(current()), key = resourceBasisKey(captured);
    for (let count = 0; count < 3; count++) {
        const revision = session.issueDraftRevision();
        session.acknowledgeDraft(session.source, session.serial, revision, session.inputId);
        assert.equal(session.acknowledgedInputRevision, revision);
        assert.equal(resourceBasisKey(current()), key);
        assert.deepEqual(resourceSourceBasis(current()), captured);
        assert.equal(Object.hasOwn(captured, "inputRevision"), false);
    }
    assert.equal(session.acknowledgedInputRevision, 3, "source proposals do not erase or acquire Save ACK authority");
});

test("real same-text replacement and edit-Undo ABA cannot revive an earlier resource author cut", () => {
    const original = basis();
    const session = new DocumentSession({ sessionId: original.sessionId, path: original.path, root: original.root,
        source: original.source, baseId: original.baseId, kind: "managed", nodeId: original.nodeId, workspaceId: original.workspaceId,
        viewInputId: original.inputId });
    const current = (): DocumentBasis => ({ ...original, inputId: session.inputId, source: session.source, serial: session.serial,
        baseId: session.baseId, path: session.path, inputRevision: session.acknowledgedInputRevision ?? undefined });
    const captured = resourceSourceBasis(current()), key = resourceBasisKey(captured);
    session.edit(session.source, "another-same-text-writer");
    assert.equal(session.source, captured.source);
    assert.equal(session.serial, captured.serial);
    assert.notEqual(resourceBasisKey(current()), key, "equal bytes cannot substitute for the original writer");
    session.edit(`${original.source}\nLater author input`, "later-input");
    session.edit(original.source, original.inputId);
    session.acknowledgeDraft(session.source, session.serial, session.issueDraftRevision(), session.inputId);
    assert.equal(session.source, captured.source);
    assert.equal(session.inputId, captured.inputId);
    assert.notEqual(session.serial, captured.serial);
    assert.notEqual(resourceBasisKey(current()), key, "even the old input ID plus equal bytes cannot erase a changed author serial");
});

test("local import sizes use the existing 64-file and cumulative 64 MiB boundaries", () => {
    checkLocalResourceSizes([{ bytes: localResourceBatchBytes }]);
    checkLocalResourceSizes(Array.from({ length: 64 }, () => ({ bytes: localResourceBatchBytes / 64 })));
    checkLocalResourceSizes([{ bytes: 0 }]);
    for (const files of [[], Array.from({ length: 65 }, () => ({ bytes: 0 })), [{ bytes: localResourceBatchBytes }, { bytes: 1 }], [{ bytes: -1 }], [{ bytes: 0.5 }], [{ bytes: NaN }], [{ bytes: Infinity }], [{ bytes: Number.MAX_SAFE_INTEGER + 1 }]]) assert.throws(() => checkLocalResourceSizes(files), UiError);
});

test("local digests preserve exact Unicode, line endings and original byte views", async () => {
    assert.equal(await resourceDigest("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    const bytes = new Uint8Array([99, 97, 98, 99, 100]);
    assert.equal(await resourceDigest(bytes.subarray(1, 4)), await resourceDigest("abc"));
    assert.notEqual(await resourceDigest("正文😀\r\n"), await resourceDigest("正文😀\n"));
    await assert.rejects(resourceDigest("\ud800"), UiError);
});

test("locators round-trip only original identifiers, own file metadata and insertion state", () => {
    for (const mode of ["importOnly", "autoAppend", "linksAppend"] satisfies LocalResourceMode[]) {
        for (const insertion of ["pending", "applying", "inserted", "uncertain"] as const) {
            const original = { ...request(), mode, insertion };
            assert.deepEqual(decodeLocalResourceLocator(JSON.stringify(original), original.owner), original);
        }
    }
    const original = request();
    assert.throws(() => decodeLocalResourceLocator(JSON.stringify(original), { ...original.owner, nodeId: "different" }), UiError);
    const invalid = [null, [], { ...original, source: "private source" }, { ...original, bytesBase64: "AQID" }, { ...original, committed: true }, { ...original, version: 2 }, { ...original, phase: "new" }, { ...original, requestKind: "importResources" }, { ...original, operationId: "new" }, { ...original, wireDigest: "0" }, { ...original, insertion: "done" }, { ...original, mode: "append" }, { ...original, owner: { ...original.owner, source: "private" } }];
    for (const value of invalid) assert.throws(() => decodeLocalResourceLocator(JSON.stringify(value)), UiError);
    assert.throws(() => decodeLocalResourceLocator("{"), UiError);
    assert.throws(() => decodeLocalResourceLocator(" ".repeat(128 * 1024 + 1)), UiError);
});

test("locator file records preserve Core-rejected names but reject hidden bytes and malformed metadata", () => {
    const original = request();
    for (const name of ["", "../other", ".hidden", "CON.txt", "bad#name.png", "trailing. ", "中".repeat(41)]) {
        const value = { ...original, files: [{ ...original.files[0], name }] };
        assert.deepEqual(decodeLocalResourceLocator(JSON.stringify(value)).files, value.files, "only Core owns final target-name qualification");
    }
    for (const patch of [{ name: "中".repeat(342) }, { name: "\ud800" }, { fileName: "folder/original.png" }, { sha256: "A".repeat(64) }, { bytes: -1 }, { bytesBase64: "AQID" }]) {
        assert.throws(() => decodeLocalResourceLocator(JSON.stringify({ ...original, files: [{ ...original.files[0], ...patch }] })), UiError);
    }
    assert.equal(decodeLocalResourceLocator(JSON.stringify({ ...original, files: [original.files[0], original.files[0]] })).files.length, 2);
    assert.throws(() => decodeLocalResourceLocator(JSON.stringify({ ...original, files: [] })), UiError);
});

test("storage retains exact original raw tokens and rejects stale retirement or immutable replacement", () => {
    const f = storage(), original = request(), key = localResourceKey(original.owner), raw = JSON.stringify(original, null, 2);
    f.values.set(key, raw);
    const first = readLocalResourceLocator(f.port, original.owner)!;
    assert.equal(first.raw, raw);
    const second = writeLocalResourceLocator(f.port, { ...first.locator, insertion: "applying" }, first);
    assert.equal(second.locator.operationId, original.operationId);
    assert.equal(readLocalResourceLocator(f.port, original.owner)!.raw, second.raw);
    assert.throws(() => removeLocalResourceLocator(f.port, first), UiError);
    assert.throws(() => writeLocalResourceLocator(f.port, original, first), UiError);
    assert.throws(() => writeLocalResourceLocator(f.port, { ...second.locator, operationId: "33333333-3333-4333-8333-333333333333" }, second), UiError);
    assert.throws(() => writeLocalResourceLocator(f.port, { ...second.locator, mode: "linksAppend" }, second), UiError);
    assert.equal(f.values.get(key), second.raw);
    removeLocalResourceLocator(f.port, second);
    assert.equal(readLocalResourceLocator(f.port, original.owner), null);
    assert.equal(writeLocalResourceLocator(f.port, original, null).locator.operationId, original.operationId);
    assert.throws(() => writeLocalResourceLocator(f.port, original, null), UiError);
});

test("malformed, denied and mismatching persistence never trigger cleanup of original or newer records", () => {
    const f = storage(), original = request(), key = localResourceKey(original.owner);
    f.values.set(key, "malformed original");
    assert.throws(() => readLocalResourceLocator(f.port, original.owner), UiError);
    assert.equal(f.values.get(key), "malformed original");
    f.values.delete(key);
    const saved = writeLocalResourceLocator(f.port, original, null);
    for (const method of ["getItem", "setItem", "removeItem"] as const) {
        const denied = { ...f.port, [method]: () => { throw Error("storage denied"); } };
        if (method === "getItem") assert.throws(() => readLocalResourceLocator(denied, original.owner), UiError);
        if (method === "setItem") assert.throws(() => writeLocalResourceLocator(denied, { ...original, insertion: "applying" }, saved), UiError);
        if (method === "removeItem") assert.throws(() => removeLocalResourceLocator(denied, saved), UiError);
        assert.equal(f.values.get(key), saved.raw);
    }
    const newer = JSON.stringify({ ...original, insertion: "uncertain" });
    const raced = { ...f.port, setItem: (key: string) => { f.values.set(key, newer); } };
    assert.throws(() => writeLocalResourceLocator(raced, { ...original, insertion: "applying" }, saved), UiError);
    assert.equal(f.values.get(key), newer);
    assert.throws(() => removeLocalResourceLocator(f.port, saved), UiError);
    assert.equal(f.values.get(key), newer);
});

test("same-node path changes discover and preserve the old locator instead of allocating a new slot", () => {
    const f = storage(), original = request(), first = writeLocalResourceLocator(f.port, original, null);
    const moved = { ...original.owner, path: "D:/Library/Moved/Note.typ" }, remounted = { ...original.owner, root: "E:/Library" };
    for (const owner of [moved, remounted]) {
        assert.equal(localResourceKey(owner), localResourceKey(original.owner));
        assert.throws(() => readLocalResourceLocator(f.port, owner), UiError);
        assert.throws(() => writeLocalResourceLocator(f.port, { ...original, owner }, null), UiError);
        assert.equal(f.values.get(localResourceKey(owner)), first.raw);
    }
    assert.deepEqual(readLocalResourceLocator(f.port, resourceOwner({ ...basis(), sessionId: "reopened-session" }))!.locator, original);
});

test("plan decoding accepts real preview fields and retains ordered results across original statuses", () => {
    const original = request();
    for (const status of ["prepared", "committed", "recoveryPending", "cancelled", "resolved"] as const) {
        const value = wirePlan(original, status), plan = decodeLocalResourcePlan(value, original);
        assert.equal(plan.status, status);
        assert.deepEqual(plan.imports, value.imports);
        assert.deepEqual(plan.changes, value.operation.changes);
        assert.equal(plan.planId, "original-plan");
        assert.equal(Object.hasOwn(plan, "resourceMap"), false, "a normalized preview does not create a persisted fact store");
    }
});

test("plan decoding rejects mismatched operations, unknown facts and incomplete or reordered results", () => {
    const original = request(), valid = wirePlan();
    for (const patch of [{ operationId: "different" }, { kind: "importResource" }, { status: "unknown" }, { planId: "" }, { changes: [] }, { changes: [{ path: "p", action: "import", bytes: -1 }] }, { source: "private" }, { losses: "bad" }]) assert.throws(() => decodeLocalResourcePlan({ ...valid, operation: { ...valid.operation, ...patch } }, original), UiError);
    for (const result of [[], [...valid.imports].reverse(), [valid.imports[0]], [valid.imports[0], valid.imports[0]], valid.imports.map(item => ({ ...item, exactRevision: "" })), valid.imports.map(item => ({ ...item, resourceRef: item.resourceRef.replace("workspace-a", "workspace-b") })), valid.imports.map(item => ({ ...item, resourceRef: item.resourceRef + "/extra" })), valid.imports.map(item => ({ ...item, bytesBase64: "AQID" }))]) assert.throws(() => decodeLocalResourcePlan({ ...valid, imports: result }, original), UiError);
    assert.throws(() => decodeLocalResourcePlan({ ...valid, status: "committed" }, original), UiError);
});

test("plan owner evidence requires exactly one original node-map value", () => {
    const original = request(), valid = wirePlan(), { nodeMap: _nodeMap, ...withoutNodeMap } = valid.operation;
    assert.throws(() => decodeLocalResourcePlan({ ...valid, operation: withoutNodeMap }, original), UiError);
    for (const nodeMap of [null, [], {}, { other: "different-node" }, { first: original.owner.nodeId, second: original.owner.nodeId }, { first: original.owner.nodeId, second: "different-node" }]) {
        assert.throws(() => decodeLocalResourcePlan({ ...valid, operation: { ...valid.operation, nodeMap } }, original), UiError);
    }
    assert.equal(decodeLocalResourcePlan(valid, original).operationId, original.operationId);
});

test("one certified middle-body insertion converts bytes to UTF-16 and is one complete Undo unit", async () => {
    const f = await insertionFixture(), patch = f.decode();
    assert.equal(f.value.edit.patches[0].from, 10);
    assert.deepEqual(patch, { from: 4, to: 4, text: f.insert });
    assert.notEqual(f.sourceHash, f.value.edit.sourceHash, "response sourceHash certifies resulting source, not original input");
    assert.equal(f.next.endsWith("\r\n#endnotes()"), true);
    let state = createSourceState(f.document.source, [history()]);
    state = state.update({ selection: { anchor: 0, head: 2 } }).state;
    state = state.update({ changes: { from: patch.from, to: patch.to, insert: patch.text }, annotations: isolateHistory.of("full"), userEvent: "input.native" }).state;
    assert.equal(state.doc.toString(), f.next);
    assert.equal(undoDepth(state), 1);
    assert.equal(undo({ state, dispatch: transaction => { state = transaction.state; } }), true);
    assert.equal(state.doc.toString(), f.document.source);
    assert.deepEqual({ from: state.selection.main.from, to: state.selection.main.to }, { from: 0, to: 2 });
});

test("insertion requires original committed identities, pending insertion intent and the stable owner", async () => {
    const f = await insertionFixture();
    for (const insertion of ["applying", "inserted", "uncertain"] as const) assert.throws(() => f.decode(f.value, { ...f.locator, insertion }), UiError);
    assert.throws(() => f.decode(f.value, { ...f.locator, mode: "importOnly" }), UiError);
    for (const status of ["prepared", "recoveryPending", "cancelled", "resolved"] as const) assert.throws(() => f.decode(f.value, f.locator, { ...f.plan, status }), UiError);
    assert.throws(() => f.decode(f.value, f.locator, { ...f.plan, operationId: "other" }), UiError);
    assert.throws(() => f.decode(f.value, f.locator, { ...f.plan, planId: "" }), UiError);
    assert.throws(() => f.decode(f.value, f.locator, f.plan, { ...f.document, nodeId: "other" }), UiError);
    assert.throws(() => f.decode({ ...f.value, operationId: "other" }), UiError);
    assert.throws(() => f.decode({ ...f.value, imports: [...f.value.imports].reverse() }), UiError);
    assert.throws(() => f.decode({ ...f.value, imports: f.value.imports.map(item => ({ ...item, exactRevision: "new" })) }), UiError);
});

test("a Core-qualified same-node runtime route may differ from frozen owner path metadata", async () => {
    const f = await insertionFixture(), originalLocator = clone(f.locator);
    // Controller must first obtain status-at proof for the current session.
    // This helper validates that route's exact source/map; it grants no Core
    // permission to move an operation across libraries or owners by itself.
    for (const patch of [{ path: "D:/Library/Renamed/Note.typ" }, { root: "E:/Library" }, { path: "D:/Library/Renamed/Note.typ", sessionId: "reopened-session" }]) {
        const current = { ...f.document, ...patch };
        assert.equal(sameResourceOwner(f.locator.owner, resourceOwner(current)), false);
        assert.deepEqual(f.decode(f.value, f.locator, f.plan, current), { from: f.at, to: f.at, text: f.insert });
    }
    assert.deepEqual(f.locator, originalLocator, "the original locator's route metadata is never rewritten");
});

test("runtime path agreement cannot qualify a different workspace or node for insertion", async () => {
    const f = await insertionFixture();
    for (const patch of [{ workspaceId: "workspace-b" }, { nodeId: "node-b" }, { workspaceId: "workspace-b", nodeId: "node-b", path: "D:/Library/Renamed/Note.typ", root: "E:/Library" }]) {
        assert.throws(() => f.decode(f.value, f.locator, f.plan, { ...f.document, ...patch }), UiError);
    }
});

test("insertion rejects stale maps, wrong byte/UTF-16 points and replacement or multiple patches", async () => {
    const f = await insertionFixture();
    for (const patch of [{ sourceHash: "f".repeat(64) }, { syntaxValid: false }, { bodyInsertion: null }, { bodyInsertion: { ...f.originalMap.bodyInsertion!, writable: false } }, { bodyInsertion: writingRange(f.document.source, f.document.source.length, f.document.source.length) }]) assert.throws(() => f.decode(f.value, f.locator, f.plan, f.document, f.sourceHash, { ...f.originalMap, ...patch }), UiError);
    for (const patch of [{ from: 4, to: 4 }, { utf16From: 10, utf16To: 10 }, { from: 9, to: 9, utf16From: 3, utf16To: 3 }, { to: 11 }, { utf16To: 5 }, { expected: "original" }, { insert: "" }, { insert: "\ud800" }, { byteFrom: 10 }]) {
        const incoming = clone(f.value); Object.assign(incoming.edit.patches[0], patch);
        assert.throws(() => f.decode(incoming), UiError);
    }
    for (const patches of [[], [...f.value.edit.patches, ...f.value.edit.patches]]) assert.throws(() => f.decode({ ...f.value, edit: { ...f.value.edit, patches } }), UiError);
});

test("native writing validation rejects corrupt maps, serials, complete source and selection atomically", async () => {
    const f = await insertionFixture();
    for (const patch of [{ serial: f.document.serial + 1 }, { serial: "3" }, { source: `${f.next}extra` }, { source: "\ud800" }, { sourceHash: "other" }, { selection: { from: f.next.length + 1, to: f.next.length + 1 } }, { map: { ...f.value.edit.map, sourceHash: "e".repeat(64) } }, { map: { ...f.value.edit.map, syntaxValid: false } }, { map: { ...f.value.edit.map, unavailableReason: { explanation: message("error.invalidResponse"), diagnostics: [], causes: [] } } }, { map: { ...f.value.edit.map, unknown: true } }, { map: { ...f.value.edit.map, literalRegions: [{ ...f.value.edit.map.literalRegions[0], range: { ...f.value.edit.map.literalRegions[0].range, from: 1 } }] } }]) assert.throws(() => f.decode({ ...f.value, edit: { ...f.value.edit, ...patch } }), UiError);
    assert.throws(() => f.decode({ ...f.value, source: "unexpected" }), UiError);
    assert.throws(() => f.decode({ ...f.value, edit: { ...f.value.edit, save: true } }), UiError);
});
