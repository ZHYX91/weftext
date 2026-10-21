import assert from "node:assert/strict";
import test from "node:test";
import { CapturePersistence, CapturePersistenceConflict, captureText, captureString, captureUtf8Ready, emptyCaptureInput, type CaptureDraftRecord, type CaptureTransport } from "./capture-draft-state";
const id = "00000000-0000-4000-8000-000000000001", mutationId = "00000000-0000-4000-8000-000000000002";
const make = (): CaptureDraftRecord => ({ version: 1, draftId: id, revision: 2, inputRevision: 1, lastMutation: { mutationId, fingerprint: "a".repeat(64), resultingRevision: 2 }, currentInput: emptyCaptureInput(), pending: null, target: null });
const transport = (run: (path: string, body: unknown) => unknown): CaptureTransport => async <T>(path: string, body: unknown) => await run(path, body) as T;
test("code-unit codec retains lone surrogates, NUL and pairs without UTF-8 replacement", () => {
  for (const text of ["", "中文😀\r\n", "\ud800", "\udc00", "\ud800\u0000\udc00", "x".repeat(8192) + "\ud800"]) {
    assert.equal(captureString(captureText(text)), text);
  }
  assert.equal(captureUtf8Ready(captureText("😀")), "😀");
  assert.throws(() => captureUtf8Ready(captureText("\ud800")), /UTF-16/);
  assert.throws(() => captureString({ utf16: [65536] }));
});
test("lost mutation reply resolves the original mutation without a second write", async () => {
  const calls: string[] = [];
  const input = emptyCaptureInput();
  const store = new CapturePersistence(transport((path, body) => { calls.push(path); if (path.endsWith("/input")) throw new Error("lost"); assert.deepEqual(body, { draftId: id, mutationId, command: { kind: "input", expectedRevision: 1, input } });return { status: "applied", draft: make() }; }));
  const actual = await store.mutate("input", { draftId: id, mutationId, expectedRevision: 1, input });
  assert.equal(actual.revision, 2);assert.deepEqual(calls, ["/api/capture/draft/input", "/api/capture/draft/mutation"]);
});
test("reused mutation ID cannot claim an earlier different command as persisted", async () => {
  for (const expectedRevision of [1, 2]) {
    const input = emptyCaptureInput();input.body = captureText("new input");
    const latest = make();latest.currentInput.body = captureText("earlier input");
    const store = new CapturePersistence(transport((path, body) => {
      if (path.endsWith("/input")) throw new Error("mutationId belongs to different input");
      assert.deepEqual(body, { draftId: id, mutationId, command: { kind: "input", expectedRevision, input } });
      return { status: "differentCommand", draft: latest };
    }));
    await assert.rejects(store.mutate("input", { draftId: id, mutationId, expectedRevision, input }), CapturePersistenceConflict);
    assert.equal(captureString(input.body), "new input");
  }
});
test("mutation snapshots before await and cannot adopt a superseding writer as its own success", async () => {
  let release!: () => void;const gate = new Promise<void>(resolve => { release = resolve; });let received: unknown;
  const latest = make();latest.lastMutation.mutationId = "00000000-0000-4000-8000-000000000003";
  const store = new CapturePersistence(transport(async (path, body) => { if (path.endsWith("/input")) { received = body;await gate;throw new Error("lost"); }return { status: "supersededOrUnknown", draft: latest }; }));
  const payload = { draftId: id, mutationId, expectedRevision: 1, input: emptyCaptureInput() };payload.input.body = captureText("sent");
  const pending = store.mutate("input", payload);payload.input.body = captureText("new input");release();
  await assert.rejects(pending, CapturePersistenceConflict);
  assert.equal(captureString((received as typeof payload).input.body), "sent");assert.equal(captureString(payload.input.body), "new input");
});
test("a durable flush ACK cannot permit closing after newer input or another receipt", async () => {
  let release!: () => void;const gate = new Promise<void>(resolve => { release = resolve; });let serial = 7;
  const store = new CapturePersistence(transport(async () => { await gate;return { draftId: id, recordRevision: 2, inputRevision: 1, lastMutationId: mutationId, pendingOperationId: null, recordDigest: "b".repeat(64) }; }));
  const pending = store.flush(make(), serial, () => serial);serial++;release();await assert.rejects(pending, CapturePersistenceConflict);
  const wrong = new CapturePersistence(transport(() => ({ draftId: id, recordRevision: 1, inputRevision: 1, lastMutationId: mutationId, pendingOperationId: null, recordDigest: "b".repeat(64) })));
  await assert.rejects(wrong.flush(make(), serial, () => serial), CapturePersistenceConflict);
});
