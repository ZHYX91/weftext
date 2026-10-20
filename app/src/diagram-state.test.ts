import test from "node:test";
import assert from "node:assert/strict";
import { DiagramController, diagramBlock, diagramOccurrences, diagramPatch, validateDiagramSource, type DiagramImage } from "./diagram-state";
const png = Uint8Array.from(Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=", "base64"));
const source = 'flowchart LR\n  A[中文😀] --> B[完成 "引号"]';
const image: DiagramImage = { source, png, width: 1, height: 1 };
const id = "12345678-1234-1234-1234-123456789abc";
const basis = { sessionId: "s", baseId: "b", serial: 1, root: "r", path: "p", source: "原稿\n" };
test("one native block retains exact Chinese source and PNG; editing source marks stale", () => {
  const block = diagramBlock(id, image); const document = `前文\n${block}\n后文`;
  const [item] = diagramOccurrences(document); assert.equal(item.source, source); assert.deepEqual(item.png, png); assert.equal(document.slice(item.from, item.to), block); assert.equal(item.stale, false);
  const changed = document.replace('let source = "flowchart', 'let source = "graph');
  assert.equal(diagramOccurrences(changed)[0].stale, true);
  const invalid = document.replace('let source = "flowchart', 'let source = "https://invalid.test/flowchart');
  assert.equal(diagramOccurrences(invalid)[0].stale, true); assert.match(diagramOccurrences(invalid)[0].source,/https:/);
  assert.match(block, /assert\(source == confirmed/); assert.match(block, /image\(bytes/);
});
test("unproven native alterations are preserved, never selected as writable diagram", () => {
  const block = diagramBlock(id, image); assert.equal(diagramOccurrences(block.replace('format: "png"', 'format: "svg"')).length, 0);
  const [item] = diagramOccurrences(block); assert.throws(() => diagramPatch({ ...basis, source: "unrelated" }, image, item), /来源已改变/);
  const patch = diagramPatch(basis, image); assert.equal(patch.from, basis.source.length); assert.equal(patch.to, basis.source.length); assert.match(patch.text, /中文😀/);
});
test("copied blocks with equal IDs still patch only the explicitly selected source occurrence", () => {
  const block=diagramBlock(id,image);const document=`${block}\nunchanged between\n${block}`;const items=diagramOccurrences(document);assert.equal(items.length,2);
  const patch=diagramPatch({...basis,source:document},{...image,source:"flowchart LR\nA-->C"},items[1]);
  assert.equal(patch.from,items[1].from);assert.equal(document.slice(0,patch.from),`${block}\nunchanged between\n`);
});
test("external resources, active config, HTML and document budget fail without rewriting", () => {
  for (const value of ["flowchart LR\nA@{img: 'https://invalid.test/x.png'}", "%%{init: {securityLevel:'loose'}}%%\ngraph LR\nA-->B", "flowchart LR\nA[<img src=x>]", "---\nconfig:\n---\nflowchart LR\nA-->B"]) assert.throws(() => validateDiagramSource(value));
  assert.throws(() => diagramPatch({ ...basis, source: "中".repeat(700000) }, image), /2 MiB/);
});
test("editing, cancellation and late success or failure cannot revive a stale PNG", async () => {
  let resolve!: (value: DiagramImage) => void; const controller = new DiagramController(() => new Promise(done => resolve = done), source);
  const first = controller.render(); controller.edit("graph LR\nA-->C"); resolve(image); assert.equal(await first, false); assert.equal(controller.image, null);
  const second = controller.render(); controller.cancel(); resolve({ ...image, source: controller.source }); assert.equal(await second, false); assert.equal(controller.image, null);
  let reject!: (reason: Error) => void; const cancelled = new DiagramController(() => new Promise((_, fail) => reject = fail), source); const third = cancelled.render(); cancelled.cancel(); reject(new Error("late")); assert.equal(await third, false);
});
