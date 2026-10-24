import test from "node:test";
import assert from "node:assert/strict";
import { NativeSurface, type CloseParticipant, type CloseProof } from "./native-host";
import { nativeDraftRequest } from "./native-draft";

const proof: CloseProof = { kind: "document", ownerId: "actual-owner", ownerGeneration: "owner-generation", sessionId: "opened-session", baseId: "opened-base", sourceDigest: "a".repeat(64), recordDigest: "b".repeat(64) };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(accept => { resolve = accept; }); return { promise, resolve }; }
async function fixture(run: (f: { surface: NativeSurface; fire: (kind: string) => void; deliver: (value: unknown) => void; messages: Record<string, unknown>[]; root: { inert: boolean }; tick: () => Promise<void> }) => Promise<void>) {
  const originals = [Object.getOwnPropertyDescriptor(globalThis, "document"), Object.getOwnPropertyDescriptor(globalThis, "window")];
  const events = new Map<string, (() => void)[]>(), messages: Record<string, unknown>[] = [], root = { inert: false };
  const received: ((event: MessageEvent<unknown>) => void)[] = [];
  Object.defineProperty(globalThis, "document", { configurable: true, value: { documentElement: root, addEventListener: (kind: string, listener: () => void) => events.set(kind, [...events.get(kind) ?? [], listener]) } });
  Object.defineProperty(globalThis, "window", { configurable: true, value: { chrome: { webview: { postMessage: (value: Record<string, unknown>) => messages.push(value), addEventListener: (_: string, listener: (event: MessageEvent<unknown>) => void) => { received.push(listener); } } } } });
  const surface = new NativeSurface("library");
  try { await run({ surface, fire: kind => events.get(kind)?.forEach(listener => listener()), deliver: value => received.forEach(listener => listener({ data: value } as MessageEvent<unknown>)), messages, root, tick: async () => { await new Promise(resolve => setImmediate(resolve)); } }); }
  finally { for (const [index, name] of ["document", "window"].entries()) { const prior = originals[index]; if (prior) Object.defineProperty(globalThis, name, prior); else Reflect.deleteProperty(globalThis, name); } }
}
const bound = { type: "surface-bound", surfaceId: "surface", surfaceEpoch: "epoch" };
const close = (type: string, attempt: string) => ({ ...bound, type, closeAttemptId: attempt });
const participant = (prepare: CloseParticipant["prepare"]): CloseParticipant => ({ prepare, freeze: () => true, thaw: () => undefined });

test("native surface cannot advertise an empty barrier and reports input preceding binding", () => fixture(async f => {
  assert.throws(() => f.surface.markReady(), /actual persistence participant/);
  f.fire("input"); f.surface.register("body", participant(async () => [proof])); f.surface.markReady(); f.deliver(bound); await f.tick();
  assert.equal(f.messages.find(message => message.type === "surface-edited")?.surfaceEditRevision, 1);
}));

test("a held persistence reply after new input cannot become a prepared close", () => fixture(async f => {
  const held = deferred<readonly CloseProof[]>(); f.surface.register("body", participant(() => held.promise)); f.surface.markReady(); f.deliver(bound);
  f.deliver(close("prepare-close", "old")); f.fire("compositionstart"); held.resolve([proof]); await f.tick();
  assert.equal(f.messages.some(message => message.type === "close-prepared"), false);
  assert.equal(f.messages.at(-1)?.type, "close-blocked"); assert.equal(f.root.inert, false);
}));

test("late acknowledgement of an earlier close cannot replace the newer prepared owner", () => fixture(async f => {
  const first = deferred<readonly CloseProof[]>(), second = deferred<readonly CloseProof[]>(); let count = 0, frozen = 0;
  f.surface.register("body", { prepare: () => ++count === 1 ? first.promise : second.promise, freeze: () => { frozen++; return true; }, thaw: () => undefined });
  f.surface.markReady(); f.deliver(bound); f.deliver(close("prepare-close", "first")); f.deliver(close("prepare-close", "second"));
  second.resolve([proof]); await f.tick(); first.resolve([proof]); await f.tick(); f.deliver(close("commit-close", "second")); await f.tick();
  assert.equal(frozen, 1); assert.equal(f.messages.at(-1)?.type, "close-frozen"); assert.equal(f.messages.at(-1)?.closeAttemptId, "second");
  f.deliver(close("close-cancelled", "second")); await f.tick(); assert.equal(f.root.inert, false);
}));

test("one producer refusing synchronous freeze thaws every producer and keeps the surface", () => fixture(async f => {
  let thawed = 0;
  f.surface.register("body", { prepare: async () => [proof], freeze: () => true, thaw: () => { thawed++; } });
  f.surface.register("proposal", { prepare: async () => [], freeze: () => false, thaw: () => { thawed++; } });
  f.surface.markReady(); f.deliver(bound); f.deliver(close("prepare-close", "request")); await f.tick(); f.deliver(close("commit-close", "request")); await f.tick();
  assert.equal(f.messages.at(-1)?.type, "close-blocked"); assert.equal(f.root.inert, false); assert.equal(thawed, 2);
  assert.equal(f.messages.some(message => message.type === "close-frozen"), false);
}));

test("retiring a registered barrier withdraws readiness until a complete registration is made", () => fixture(async f => {
  const dispose = f.surface.register("body", participant(async () => [proof])); f.surface.markReady(); f.deliver(bound); dispose();
  assert.equal(f.messages.at(-1)?.type, "surface-unready"); f.deliver(close("prepare-close", "old")); await f.tick();
  assert.equal(f.messages.at(-1)?.type, "close-blocked"); assert.throws(() => f.surface.markReady());
}));

test("native draft transport binds an original request to the real surface epoch and preserves exact source", () => fixture(async f => {
  f.surface.register("body", participant(async () => [proof])); f.surface.markReady(); f.deliver(bound);
  const source = "\ufeffA\r\n中文😀\nB\rC"; let settled = false;
  const result = nativeDraftRequest("/api/draft", { sessionId: "actual-session", source })!;
  result.then(() => { settled = true; });
  const call = f.messages.at(-1)!; assert.equal(call.type, "draft-call"); assert.deepEqual(call.request, { kind: "persist", body: { sessionId: "actual-session", source } });
  f.deliver({ ...bound, type: "core-draft-result", surfaceEpoch: "retired", requestId: call.requestId, body: { ok: true } }); await f.tick(); assert.equal(settled, false);
  f.deliver({ ...bound, type: "core-draft-result", requestId: call.requestId, body: { ok: true } }); assert.deepEqual(await result, { ok: true });
  assert.equal(nativeDraftRequest("/api/save", { source }), undefined);
}));

test("unsupported raw UTF16 is rejected before native transmission and is never normalized", () => fixture(async f => {
  f.surface.register("body", participant(async () => [proof])); f.surface.markReady(); f.deliver(bound); const before = f.messages.length;
  const source = "A\ud800B";
  await assert.rejects(nativeDraftRequest("/api/draft", { sessionId: "actual-session", source })!, /raw UTF16/);
  assert.equal(source.charCodeAt(1), 0xd800); assert.equal(f.messages.length, before);
}));

test("only the exact current close attempt may thaw a frozen author and an edited attempt keeps its cancellation identity", () => fixture(async f => {
  let thawed = 0;
  f.surface.register("body", { prepare: async () => [proof], freeze: () => true, thaw: () => { thawed++; } });
  f.surface.markReady(); f.deliver(bound); f.deliver(close("prepare-close", "current")); await f.tick();
  f.deliver(close("commit-close", "current")); await f.tick(); assert.equal(f.root.inert, true);
  for (const message of [{ type: "close-cancelled" }, close("close-cancelled", "old"), { ...close("close-cancelled", "current"), surfaceEpoch: "old-epoch" }]) {
    f.deliver(message); await f.tick(); assert.equal(f.root.inert, true); assert.equal(thawed, 0);
  }
  f.fire("input");
  f.deliver(close("close-cancelled", "current")); await f.tick();
  assert.equal(f.root.inert, false); assert.equal(thawed, 1);
}));

test("host actions need the exact bound incarnation and a new bound message cannot replace it", () => fixture(async f => {
  const delivered: Record<string, unknown>[] = [];
  f.surface.register("body", participant(async () => [proof])); f.surface.listen(message => delivered.push(message));
  f.surface.markReady(); f.deliver(bound);
  for (const message of [{ type: "open-reminders" }, { ...bound, type: "open-reminders", surfaceEpoch: "old-epoch" }]) f.deliver(message);
  f.deliver({ ...bound, type: "open-reminders" });
  f.deliver({ ...bound, surfaceEpoch: "replacement" }); await f.tick();
  assert.equal(delivered.length, 1); assert.equal(f.messages.at(-1)?.type, "surface-failed");
  f.surface.post({ type: "document-delivered", deliveryId: "original", accepted: false });
  assert.equal(f.messages.at(-1)?.surfaceEpoch, bound.surfaceEpoch);
}));

test("a second prepare cannot replace the cancellation identity of a frozen attempt", () => fixture(async f => {
  let thawed = 0;
  f.surface.register("body", { prepare: async () => [proof], freeze: () => true, thaw: () => { thawed++; } });
  f.surface.markReady(); f.deliver(bound);
  f.deliver(close("prepare-close", "original")); await f.tick();
  f.deliver(close("commit-close", "original")); await f.tick();
  f.deliver(close("prepare-close", "late")); await f.tick();
  assert.equal(f.root.inert, true);
  f.deliver(close("close-cancelled", "late")); await f.tick();
  assert.equal(f.root.inert, true); assert.equal(thawed, 0);
  f.deliver(close("close-cancelled", "original")); await f.tick();
  assert.equal(f.root.inert, false); assert.equal(thawed, 1);
}));

test("cancellation during persistence stops preparation before entering the next producer", () => fixture(async f => {
  const first = deferred<readonly CloseProof[]>(); let later = 0;
  f.surface.register("body", participant(() => first.promise));
  f.surface.register("proposal", participant(async () => { later++; return []; }));
  f.surface.markReady(); f.deliver(bound);
  f.deliver(close("prepare-close", "cancelled"));
  f.deliver(close("close-cancelled", "cancelled"));
  first.resolve([proof]); await f.tick();
  assert.equal(later, 0);
  assert.equal(f.messages.some(message => message.type === "close-prepared"), false);
  assert.equal(f.root.inert, false);
}));

test("duplicate preparation and commit do not restart or thaw the original producer", () => fixture(async f => {
  const held = deferred<readonly CloseProof[]>(); let prepared = 0, frozen = 0, thawed = 0;
  f.surface.register("body", { prepare: () => { prepared++; return held.promise; }, freeze: () => ++frozen === 1, thaw: () => { thawed++; } });
  f.surface.markReady(); f.deliver(bound);
  f.deliver(close("prepare-close", "original"));
  f.deliver(close("prepare-close", "original"));
  held.resolve([proof]); await f.tick();
  assert.equal(prepared, 1);
  f.deliver(close("commit-close", "original")); await f.tick();
  f.deliver(close("commit-close", "original")); await f.tick();
  assert.equal(frozen, 1); assert.equal(thawed, 0); assert.equal(f.root.inert, true);
  f.deliver(close("close-cancelled", "original"));
}));
