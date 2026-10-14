import assert from "node:assert/strict";
import test from "node:test";
import { decodeMediaDescriptor, mediaChunk, mediaIntentTarget, mediaResolved, mediaTicket, mediaCellReferences, mediaAnnotationSelection, type HostedMediaDescriptor, type HostedMediaResolved, type HostedMediaTicket, type HostedMediaChunk } from "./hosted-media-state";
const ref = "weftext-resource://workspace/image";
const descriptor = (): HostedMediaDescriptor => ({ asset: "image", owner: "Shared", resource: ref, revision: "exact", name: "批准名称", title: "批准名称", bytes: 3, contentType: "image/png", metadata: { kind: "image", width: 10, height: 8 }, annotationReady: true, wholeAnnotationReady: true, currentness: "accepted", lifecycle: "live", status: "ready", download: false });
const ticket = (): HostedMediaTicket => ({ ...descriptor(), ticket: "original", purpose: "view", sha256: "a".repeat(64), expiresInMs: 900000 });
const chunk = (): HostedMediaChunk => ({ ticket: "original", purpose: "view", sha256: "a".repeat(64), bytesBase64: "AQID", length: 3, offset: 0, total: 3, done: true });
const resolved = (): HostedMediaResolved => ({ ...descriptor(), viewerTarget: { resource: ref, revision: "exact", resourceFragment: { status: "none" } } });
test("hosted media tickets bind exact asset, approved name, metadata, version and purpose", () => {
  assert.equal(mediaTicket(descriptor(), "view", ticket()).ticket, "original");
  for (const changes of [{ purpose: "download" }, { revision: "latest" }, { resource: "weftext-resource://workspace/other" }, { owner: "Other" }, { name: "private-path-name" }, { bytes: 4 }, { metadata: { kind: "image", width: 12, height: 8 } }]) assert.throws(() => mediaTicket(descriptor(), "view", { ...ticket(), ...changes } as HostedMediaTicket));
  assert.throws(() => decodeMediaDescriptor({ ...descriptor(), revision: "" }));
  assert.throws(() => decodeMediaDescriptor({ ...descriptor(), metadata: { kind: "video", width: 10, height: 8, timebase: { numerator: Number.MAX_SAFE_INTEGER + 1, denominator: 1 }, durationTick: 8000 } }));
  assert.deepEqual(decodeMediaDescriptor({ ...descriptor(), status: "unavailable" } as never), { asset: "image", owner: "Shared", title: "批准名称", name: "批准名称", lifecycle: "live", status: "unavailable" });
});
test("hosted media validates every original chunk and permits only the empty final zero block", () => {
  assert.deepEqual([...mediaChunk(ticket(), 0, chunk())], [1, 2, 3]);
  for (const changes of [{ ticket: "other" }, { purpose: "download" }, { offset: 1 }, { total: 4 }, { sha256: "b".repeat(64) }, { done: false }, { length: 0 }, { length: 65537 }, { bytesBase64: "AQI=" }]) assert.throws(() => mediaChunk(ticket(), 0, { ...chunk(), ...changes } as HostedMediaChunk));
  assert.equal(mediaChunk({ ...ticket(), bytes: 0 }, 0, { ...chunk(), bytesBase64: "", length: 0, total: 0 }).length, 0);
  assert.throws(() => mediaChunk({ ...ticket(), bytes: 0 }, 0, { ...chunk(), bytesBase64: "", length: 0, total: 0, done: false }));
  assert.throws(() => mediaChunk(ticket(), -1, chunk()));
});
test("hosted resource resolution cannot substitute a ref, exact version or missing page", () => {
  assert.equal(mediaResolved(resolved(), ref).resource, ref);
  assert.throws(() => mediaResolved(resolved(), "weftext-resource://workspace/other"));
  assert.throws(() => mediaResolved(resolved(), ref, "old"));
  const pdf: HostedMediaResolved = { ...resolved(), metadata: { kind: "pdf", pages: [{ width: 100, height: 120, rotation: 0 }, { width: 100, height: 120, rotation: 0 }] }, viewerTarget: { resource: ref, revision: "exact", resourceFragment: { status: "found", pageIndex: 1 } } };
  assert.equal(mediaResolved(pdf, ref + "#page=2").viewerTarget.pageIndex, 1);
  for (const status of ["missing", "unavailable"] as const) assert.throws(() => mediaResolved({ ...pdf, viewerTarget: { ...pdf.viewerTarget, resourceFragment: { status } } }, ref + "#page=2"), /未退回/);
  assert.throws(() => mediaResolved(resolved(), ref + "#page=99"));
  assert.throws(() => mediaResolved({ ...pdf, viewerTarget: { ...pdf.viewerTarget, resourceFragment: { status: "found", pageIndex: 9 } } }, ref + "#page=10"));
});
test("hosted media region intents carry asset and verified geometry without client ResourceRef authority", () => {
  const selection = { descriptor: descriptor(), target: { kind: "image_region" as const, resource: ref, revision: "exact", width: 10, height: 8, x: 1, y: 1, rectWidth: 3, rectHeight: 2 } };
  const input = mediaIntentTarget(selection); assert.equal(input.asset, "image"); assert.equal(input.revision, "exact"); assert.equal("resource" in input, false); assert.equal(input.rectWidth, 3);
  assert.throws(() => mediaIntentTarget({ ...selection, target: { ...selection.target, width: 11 } }));
  assert.throws(() => mediaIntentTarget({ ...selection, descriptor: { ...descriptor(), lifecycle: "trash" } }, true));
  const unsupported: HostedMediaDescriptor = { ...descriptor(), annotationReady: false, metadata: { kind: "unsupported", message: "没有区域坐标" } };
  assert.deepEqual(mediaIntentTarget({ descriptor: unsupported, target: null }, true), { kind: "whole_resource", asset: "image", revision: "exact" });
  assert.throws(() => mediaIntentTarget({ descriptor: unsupported, target: null }));
  const target = mediaAnnotationSelection({ kind: "imageRegion", resource: ref, revisionId: "exact", pixelWidth: 10, pixelHeight: 8, rect: { x: 1, y: 1, width: 3, height: 2 } }); assert.deepEqual(target, selection.target);
});
test("typed Query resource cells gain independent viewer actions without guessing paths or authority", () => {
  assert.deepEqual(mediaCellReferences({ state: "value", value: { kind: "record", value: [{ key: "asset", value: { kind: "array", value: [{ kind: "text", value: ref }, { kind: "text", value: ref }, { kind: "text", value: "C:/private/file.png" }] } }] } }), [ref]);
  assert.deepEqual(mediaCellReferences({ state: "unavailable", value: { kind: "text", value: ref } }), []);
  assert.deepEqual(mediaCellReferences({ state: "value", value: { kind: "text", value: "weftext-node://workspace/note" } }), []);
});
