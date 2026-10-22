import assert from "node:assert/strict";
import test from "node:test";
import type { NativeValue } from "./data-panel";
import { CapturePersistence, CapturePersistenceConflict, captureInputForPersistence, restoreCaptureField, captureFieldCandidate, captureText, captureString, captureUtf8Ready, emptyCaptureInput, type CaptureDraftRecord, type CaptureTransport } from "./capture-draft-state";
const id = "00000000-0000-4000-8000-000000000001", mutationId = "00000000-0000-4000-8000-000000000002";
const make = (): CaptureDraftRecord => ({ version: 2, draftId: id, revision: 2, inputRevision: 1, inputCut: "full", lastMutation: {kind:"input", mutationId, fingerprint: "a".repeat(64), resultingRevision: 2 }, currentInput: emptyCaptureInput(), pending: null, target: null });
const transport = (run: (path: string, body: unknown) => unknown): CaptureTransport => async <T>(path: string, body: unknown) => await run(path, body) as T;
test("code-unit codec retains lone surrogates, NUL and pairs without UTF-8 replacement", () => {
  for (const text of ["", "中文😀\r\n", "\ud800", "\udc00", "\ud800\u0000\udc00", "x".repeat(8192) + "\ud800"]) {
    assert.equal(captureString(captureText(text)), text);
  }
  assert.equal(captureUtf8Ready(captureText("😀")), "😀");
  assert.throws(() => captureUtf8Ready(captureText("\ud800")), /UTF-16/);
  assert.throws(() => captureString({ utf16: [65536] }));
});
test("an incomplete UTF-16 native candidate cannot reject persistence of its corresponding control raw", () => {
  const input = emptyCaptureInput();
  input.fields = [{ fieldId: "tasks/text", valueState: { kind: "valid", value: { kind: "array", value: [{ kind: "text", value: "\ud800" }] } } }];
  assert.throws(() => captureInputForPersistence(input), /原文控件/);
  input.rawInputs = [{ fieldId: "tasks/text", valuePath: [{ kind: "index", index: 0 }], controlRole: "text", text: captureText("\ud800"), binding: "exact", author: null }];
  const saved = captureInputForPersistence(input);
  assert.equal(saved.fields[0].valueState.kind, "incomplete");
  assert.equal(captureString(saved.rawInputs[0].text), "\ud800");
  assert.equal(input.fields[0].valueState.kind, "valid");
});
test("one private incomplete field preserves siblings, union tags and list positions through a JSON restart boundary", () => {
  const input = emptyCaptureInput();
  const basis = { schemaRef: "weftext-config://original/schema", semanticDigest: "a".repeat(64) };
  input.formBasis = basis;
  const value: NativeValue = {kind:"record",value:[
    {key:"kind",value:{kind:"text",value:"absolute"}},
    {key:"siblings",value:{kind:"array",value:[{kind:"int",value:"9007199254740993"},{kind:"record",value:[{key:"raw",value:{kind:"text",value:"PRIVATE\ud800\0"}},{key:"other",value:{kind:"text",value:"preserved😀"}}]}]}},
    {key:"tail",value:{kind:"bool",value:true}},
  ]};
  const path = [{kind:"member",key:"siblings"},{kind:"index",index:1},{kind:"member",key:"raw"}] as const;
  input.fields = [{fieldId:"tasks/reminder",valueState:{kind:"valid",value}}];
  input.rawInputs = [{fieldId:"tasks/reminder",valuePath:[...path],controlRole:"exact-instant",text:captureText("PRIVATE\ud800\0"),binding:"exact",author:null}];
  const saved = captureInputForPersistence(input), restarted = JSON.parse(JSON.stringify(saved)) as typeof input;
  assert.equal(restarted.fields[0].valueState.kind,"incomplete");
  assert.deepEqual(restoreCaptureField(restarted.fields[0],restarted,{fieldId:"tasks/reminder",...basis}),value);
  assert.throws(()=>captureFieldCandidate(restarted.fields[0]),/未完成/);
  assert.throws(()=>restoreCaptureField(restarted.fields[0],restarted,{fieldId:"tasks/other",...basis}),/身份/);
  assert.throws(()=>restoreCaptureField(restarted.fields[0],restarted,{fieldId:"tasks/reminder",...basis,semanticDigest:"b".repeat(64)}),/身份/);
  const retired=structuredClone(restarted);retired.rawInputs[0].binding="copyOnly";
  assert.throws(()=>restoreCaptureField(retired.fields[0],retired,{fieldId:"tasks/reminder",...basis}),/原文控件/);
  const shifted=structuredClone(restarted);shifted.rawInputs[0].valuePath[1]={kind:"index",index:0};
  assert.throws(()=>captureInputForPersistence(shifted),/原文控件/);
  assert.deepEqual(restarted,saved);
});
test("unfinished field reader rejects discarded JSON meaning, arbitrary forms and old null formats", () => {
  const input=emptyCaptureInput();input.rawInputs=[{fieldId:"field",valuePath:[],controlRole:"text",text:captureText("\ud800"),binding:"exact",author:null}];
  for(const wire of ['{"kind":"text","kind":"text","value":"\\ud800"}', '{"kind":"text","value":"\\ud800","schema":{}}', '{"kind":"text","value":"complete"}', '{"schema":{},"fields":[],"body":"\\ud800"}', '{"kind":"record","value":[{"key":"a","value":{"kind":"text","value":"\\ud800"}},{"key":"a","value":{"kind":"none"}}]}']) {
    input.fields=[{fieldId:"field",valueState:{kind:"incomplete",wire:captureText(wire)}}];
    const before=structuredClone(input);assert.throws(()=>captureInputForPersistence(input));assert.deepEqual(input,before);
  }
  input.fields=[{fieldId:"field",value:null} as unknown as typeof input.fields[0]];
  assert.throws(()=>captureInputForPersistence(input),/格式/);
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
  const store = new CapturePersistence(transport(async () => { await gate;return { draftId: id, recordRevision: 2, inputRevision: 1, inputCut: "full", lastMutationId: mutationId, pendingOperationId: null, recordDigest: "b".repeat(64) }; }));
  const pending = store.flush(make(), serial, () => serial);serial++;release();await assert.rejects(pending, CapturePersistenceConflict);
  const wrong = new CapturePersistence(transport(() => ({ draftId: id, recordRevision: 1, inputRevision: 1, inputCut: "full", lastMutationId: mutationId, pendingOperationId: null, recordDigest: "b".repeat(64) })));
  await assert.rejects(wrong.flush(make(), serial, () => serial), CapturePersistenceConflict);
});
