import test from "node:test";
import assert from "node:assert/strict";
import { mediaRectangle, playerTick, storedMediaSelection, resourceOpenError } from "./media-selection";

test("image drag retains original pixels, reversed drag, and exact boundaries", () => {
  assert.deepEqual(mediaRectangle({ x: 99.5, y: 49.4 }, { x: 10.2, y: 3.8 }, 100, 50, true), { x: 10, y: 3, rectWidth: 90, rectHeight: 47 });
  assert.deepEqual(mediaRectangle({ x: -8, y: -4 }, { x: 180, y: 90 }, 100, 50, true), { x: 0, y: 0, rectWidth: 100, rectHeight: 50 });
  assert.equal(mediaRectangle({ x: 1, y: 1 }, { x: 1, y: 1 }, 100, 50, true), null);
  assert.equal(mediaRectangle({ x: 1.5, y: 1.5 }, { x: 1.5, y: 1.5 }, 100, 50, true), null);
});
test("PDF point coordinates retain fractional accuracy", () => {
  assert.deepEqual(mediaRectangle({ x: 1.5, y: 2.25 }, { x: 10.5, y: 20.75 }, 300, 400, false), { x: 1.5, y: 2.25, rectWidth: 9, rectHeight: 18.5 });
  assert.equal(mediaRectangle({ x: Number.NaN, y: 1 }, { x: 5, y: 6 }, 300, 400, false), null);
});
test("player capture explicitly quantizes to server timebase with bounds", () => {
  assert.equal(playerTick(0.12345, 48000, 1, 48000), 5926);
  assert.equal(playerTick(2, 30000, 1001, 100), 60);
  assert.equal(playerTick(100, 48000, 1, 48000), 48000);
  assert.equal(playerTick(-1, 48000, 1, 48000), null);
  assert.equal(playerTick(1, Number.MAX_SAFE_INTEGER + 1, 1, 48000), null);
});
test("media back-jump preserves stored exact target units and rejects unknown forms", () => {
  assert.deepEqual(storedMediaSelection({ kind: "pdfRegion", resource: "r", revisionId: "v", pageWidth: 300, pageHeight: 400, pageIndex: 2, rotation: 90, rect: { x: 1.5, y: 2, width: 30, height: 40 } }), { kind: "pdf_region", resource: "r", revision: "v", width: 300, height: 400, pageIndex: 2, rotation: 90, x: 1.5, y: 2, rectWidth: 30, rectHeight: 40 });
  const video = storedMediaSelection({ kind: "mediaInterval", resource: "r", revisionId: "v", timebase: { numerator: 30000, denominator: 1001 }, durationTick: 1000, startTick: 10, endTick: 20, region: { pixelWidth: 320, pixelHeight: 180, rect: { x: 2, y: 3, width: 40, height: 50 } } });
  assert.equal(video?.mediaType, "video"); assert.equal(video?.timebaseDenominator, 1001); assert.equal(video?.startTick, 10);
  assert.equal(storedMediaSelection({ kind: "sourceRange", anchor: { raw: "private source" } }), null);
  assert.equal(storedMediaSelection({ kind: "imageRegion", resource: "r", revisionId: "v", pixelWidth: 100, pixelHeight: 100, rect: { x: 0.5, y: 1, width: 5, height: 5 } }), null);
  assert.equal(storedMediaSelection({ kind: "imageRegion", resource: "r", revisionId: "v", pixelWidth: 100, pixelHeight: 100, rect: { x: 1, y: 1, width: 5, height: 5 } })?.x, 1);
});
test("resource navigation requires the exact version and proved bounded PDF page", () => {
  const resource = { resource: "weftext-resource://w/r", revision: "actual-version", metadata: { kind: "pdf", pages: [{}, {}] } };
  const target = { resource: resource.resource, revision: resource.revision };
  assert.equal(resourceOpenError(target, resource), null);
  assert.equal(resourceOpenError({ ...target, pageIndex: 1 }, resource), null);
  assert.notEqual(resourceOpenError({ ...target, revision: "old-version" }, resource), null);
  assert.notEqual(resourceOpenError({ ...target, resource: "similar-file-name" }, resource), null);
  for (const pageIndex of [-1, 1.5, 2]) assert.notEqual(resourceOpenError({ ...target, pageIndex }, resource), null);
  assert.notEqual(resourceOpenError({ ...target, pageIndex: 0 }, { ...resource, metadata: { kind: "image" } }), null);
});
