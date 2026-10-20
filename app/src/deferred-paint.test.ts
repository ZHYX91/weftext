import test from "node:test";
import assert from "node:assert/strict";
import { DeferredPaint } from "./deferred-paint";

test("focus and command lease release alone never rebuild an unchanged surface", async () => {
  const paint = new DeferredPaint(), owner = {}, projection = {}; let paints = 0;
  for (let index = 0; index < 3; index++) paint.flush(() => [owner, projection], () => false, () => paints++);
  await Promise.resolve(); assert.equal(paints, 0);
});

test("a genuinely deferred native projection installs once after its live input settles", async () => {
  const paint = new DeferredPaint(), owner = {}, projection = {}; let retained = true, paints = 0;
  paint.defer([owner, "session", "source", 3, projection, "write", false]);
  const flush = () => paint.flush(() => [owner, "session", "source", 3, projection, "write", false], () => retained, () => paints++);
  flush(); await Promise.resolve(); assert.equal(paints, 0);
  retained = false; flush(); flush(); await Promise.resolve(); assert.equal(paints, 1);
  flush(); await Promise.resolve(); assert.equal(paints, 1);
});

test("new session, source, serial, projection, mode, host or permission retires an old deferred paint", async () => {
  const original = [{}, "session", "source", 3, {}, "write", {}, false];
  for (let index = 0; index < original.length; index++) {
    const paint = new DeferredPaint(), current = [...original]; let paints = 0;
    paint.defer(original); paint.flush(() => current, () => false, () => paints++);
    current[index] = typeof original[index] === "object" ? {} : "changed";
    await Promise.resolve(); assert.equal(paints, 0, "Changed presentation index " + index);
    paint.flush(() => original, () => false, () => paints++); await Promise.resolve(); assert.equal(paints, 0, "Retired request cannot return after navigation");
  }
});

test("the latest truly deferred projection replaces an earlier queued one without losing its paint", async () => {
  const paint = new DeferredPaint(), owner = {}, old = {}, latest = {}; let current = old, paints = 0;
  paint.defer([owner, old]); paint.flush(() => [owner, current], () => false, () => paints++);
  current = latest; paint.defer([owner, latest]); paint.flush(() => [owner, current], () => false, () => paints++);
  await Promise.resolve(); assert.equal(paints, 1);
});

test("explicit teardown cancels an already queued paint without cancelling a new owner's request", async () => {
  const paint = new DeferredPaint(), old = {}, next = {}; const paints: object[] = [];
  paint.defer([old]); paint.flush(() => [old], () => false, () => paints.push(old));
  paint.clear(); paint.defer([next]); paint.flush(() => [next], () => false, () => paints.push(next));
  await Promise.resolve(); assert.deepEqual(paints, [next]);
});
