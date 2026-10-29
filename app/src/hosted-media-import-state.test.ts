import assert from "node:assert/strict";
import test from "node:test";
import { canCommitResourceImport, checkResourceImportSizes, decodeResourceImportDraft, encodeResourceImportDraft, freezeResourceImport, hostedImportBatchBytes, hostedImportFileBytes, resourceCollection, resourceImportDigest, resourceImportInput, resourceImportLocator, resourceImportReceipt, type HostedResourceCollection, type ResourceImportDraft, type ResourceImportLocator, type ResourceImportReceipt } from "./hosted-media-import-state";
import { message } from "./ui-locale";
const collection = (): HostedResourceCollection => ({ collection: "attachments", owner: "Shared", title: "授权资料", lifecycle: "live", read: false, download: false, import: true, status: "ready" });
const draft = async (): Promise<ResourceImportDraft> => ({ version: 2, collection: "attachments", files: [{ name: "z-原图😀.png", fileName: "原件.png", bytesBase64: "AQID", bytes: 3, sha256: await resourceImportDigest(new Uint8Array([1, 2, 3])) }, { name: "a-empty.txt", fileName: "empty.txt", bytesBase64: "", bytes: 0, sha256: await resourceImportDigest(new Uint8Array()) }], revision: 2 });
const receipt = (request: ResourceImportLocator): ResourceImportReceipt => ({ collection: "attachments", owner: "Shared", operationId: request.operationId, status: "prepared", planId: "plan", preview: { files: request.files.map(file => ({ name: file.name, bytes: file.bytes, contentType: "image/png", metadata: { kind: "image", width: 12, height: 9 } })), totalBytes: request.files.reduce((sum, file) => sum + file.bytes, 0), summary: message("error.api.hostedResourceCollections.importPlanSummary") }, resourceRefs: [], assets: [], results: [], commitAllowed: true });
test("resource batch import is independent of read/download and never acquires a Trash writer", async () => {
    assert.equal(resourceCollection(collection()).read, false);
    assert.equal(freezeResourceImport(await draft(), collection(), "original").files.length, 2);
    assert.throws(() => resourceCollection({ ...collection(), download: true }));
    assert.throws(() => resourceCollection({ ...collection(), lifecycle: "trash" }));
    for (const denied of [{ ...collection(), import: false }, { ...collection(), status: "unavailable" as const }, { ...collection(), lifecycle: "trash" as const }]) assert.throws(() => freezeResourceImport({ version: 2, collection: "attachments", files: [], revision: 1 }, denied, "original"));
});
test("batch request freezes every file in original order; locator preserves hashes without upload bytes", async () => {
    const input = await draft(), request = freezeResourceImport(input, collection(), "original"), locator = await resourceImportLocator(request);
    input.files[0].name = "later"; input.files[0].bytesBase64 = "BAUG"; input.files.reverse();
    assert.deepEqual(request.files.map(file => file.name), ["z-原图😀.png", "a-empty.txt"]);
    assert.deepEqual(request.files.map(file => file.bytesBase64), ["AQID", ""]);
    assert.deepEqual(Object.keys(request).sort(), ["collection", "files", "operationId"]);
    assert.deepEqual(locator.files.map(file => file.bytes), [3, 0]);
    assert.equal(locator.files[0].sha256, await resourceImportDigest(new Uint8Array([1, 2, 3])));
    assert.match(locator.wireDigest, /^[a-f0-9]{64}$/);
    assert.doesNotMatch(JSON.stringify(locator), /bytesBase64|AQID/);
});
test("invalid per-file names/base64 and every actual hosted batch budget are rejected as a whole", async () => {
    const original = await draft();
    for (const name of ["../private.png", "bad:name", ".", "bad.", "bad ", ""]) assert.throws(() => freezeResourceImport({ ...original, files: [{ ...original.files[0], name }] }, collection(), "x"));
    for (const file of [{ ...original.files[0], bytes: 4 }, { ...original.files[0], bytesBase64: "YR==" }, { ...original.files[0], bytesBase64: undefined }]) assert.throws(() => freezeResourceImport({ ...original, files: [file] }, collection(), "x"));
    checkResourceImportSizes([{ bytes: hostedImportFileBytes }, { bytes: hostedImportFileBytes }]);
    assert.throws(() => checkResourceImportSizes([{ bytes: hostedImportFileBytes + 1 }]));
    assert.throws(() => checkResourceImportSizes([{ bytes: hostedImportFileBytes }, { bytes: hostedImportFileBytes }, { bytes: 1 }]));
    assert.throws(() => checkResourceImportSizes(Array.from({ length: 65 }, () => ({ bytes: 0 }))));
    assert.throws(() => checkResourceImportSizes([])); assert.equal(hostedImportBatchBytes, 32 * 1024 * 1024);
});
test("later file/collection input and same-value ABA disable commit without replacing the original batch", async () => {
    const input = await draft(), request = await resourceImportLocator(freezeResourceImport(input, collection(), "original"));
    input.pending = { request, owner: "Shared", input: resourceImportInput(input), revision: input.revision, receipt: receipt(request) };
    assert.equal(canCommitResourceImport(input, collection(), true), true);
    assert.equal(canCommitResourceImport(input, collection(), false), false);
    input.files[0].name = "B"; input.revision++; assert.equal(canCommitResourceImport(input, collection(), true), false);
    input.files[0].name = request.files[0].name; input.revision++; assert.equal(canCommitResourceImport(input, collection(), true), false);
    assert.equal(canCommitResourceImport({ ...input, revision: 2 }, { ...collection(), owner: "Remapped" }, true), false);
    assert.equal(input.pending.request.operationId, "original"); assert.throws(() => freezeResourceImport(input, collection(), "new-id"));
});
test("all preview files match original order/size and ordered results do not zip the sorted ref set", async () => {
    const input = await draft(), request = await resourceImportLocator(freezeResourceImport(input, collection(), "original")), prepared = receipt(request);
    assert.throws(() => resourceImportReceipt(request, { ...prepared, preview: { ...prepared.preview, files: [...prepared.preview.files!].reverse() } }));
    assert.throws(() => resourceImportReceipt(request, { ...prepared, preview: { ...prepared.preview, totalBytes: 9 } }));
    const refs = ["weftext-resource://workspace/a", "weftext-resource://workspace/z"];
    const committed = { ...prepared, status: "committed", planId: null, preview: {}, commitAllowed: false, resourceRefs: refs, assets: ["attachments/a", "attachments/z"], results: request.files.map((file, index) => ({ name: file.name, resourceRef: refs[1 - index], exactRevision: "opaque-revision-" + index })) };
    assert.deepEqual(resourceImportReceipt(request, committed).results.map(file => file.resourceRef), [...refs].reverse());
    assert.throws(() => resourceImportReceipt(request, { ...committed, results: committed.results.slice(0, 1) }));
    assert.throws(() => resourceImportReceipt(request, { ...committed, results: committed.results.map(file => ({ ...file, resourceRef: refs[0] })) }));
    assert.throws(() => resourceImportReceipt(request, { ...prepared, results: committed.results }));
});
test("small locator recovery drops uploads and keeps one immutable unresolved original; old single records stay unreadable", async () => {
    const input = await draft(), request = await resourceImportLocator(freezeResourceImport(input, collection(), "original"));
    input.pending = { request, owner: "Shared", input: resourceImportInput(input), revision: 2, receipt: receipt(request) };
    const encoded = encodeResourceImportDraft(input), recovered = decodeResourceImportDraft(encoded);
    assert.doesNotMatch(encoded, /bytesBase64|AQID/); assert.equal(recovered.pending!.request.operationId, "original");
    assert.equal(recovered.files[0].bytesBase64, undefined);
    assert.equal(canCommitResourceImport(recovered, collection(), false), false);
    assert.equal(canCommitResourceImport(recovered, collection(), true), true, "a freshly qualified prepared server plan owns its already-frozen bytes");
    assert.throws(() => freezeResourceImport(recovered, collection(), "new-id"));
    assert.throws(() => decodeResourceImportDraft(JSON.stringify({ collection: "attachments", name: "old-single", revision: 1, bytesBase64: "AQID" })));
    assert.throws(() => decodeResourceImportDraft(JSON.stringify({ ...recovered, files: [{ ...recovered.files[0], bytesBase64: "AQID" }] })));
    assert.throws(() => decodeResourceImportDraft(JSON.stringify({ ...recovered, pending: { ...recovered.pending, request: { ...request, wireDigest: "wrong" } } })));
});
test("filtered current-decision facts replace previews/results without changing original terminal outcome", async () => {
    const request = await resourceImportLocator(freezeResourceImport(await draft(), collection(), "original")), prepared = receipt(request);
    const filtered = { ...prepared, owner: "Remapped", status: "committed", planId: null, preview: {}, resourceRefs: [], assets: [], results: [], commitAllowed: false };
    assert.deepEqual(resourceImportReceipt(request, filtered, prepared), filtered);
    assert.throws(() => resourceImportReceipt(request, prepared, filtered));
    assert.throws(() => resourceImportReceipt(request, { ...filtered, operationId: "another" }));
    assert.throws(() => resourceImportReceipt(request, { ...prepared, planId: "another" }, prepared));
});
