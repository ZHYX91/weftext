import { test } from "node:test";
import assert from "node:assert/strict";
import { SyncRevalidationSession, syncTextDigest } from "./sync-revalidation";
import { SyncFlow, type ObjectState, type SyncPending, type SyncRevalidationCut, type SyncRevalidationInput, type SyncRevalidationReview, type SyncPlan } from "./sync-state";

const node = { kind: "node" as const, id: "note" }, schema = { kind: "schema" as const }, identity = { workspaceId: "workspace", genesis: "genesis" };
const state = (): ObjectState => ({ parent: null, name: "Note", retired: false, files: { "Note.typ": "source-digest", "resource/unknown.bin": "opaque-digest", "weftext.annotations.json": "discussion-digest" }, fileRevisions: { "Note.typ": "source-revision", "resource/unknown.bin": "opaque-revision", "weftext.annotations.json": "discussion-revision" } });
function cut(): SyncRevalidationCut {
  return { expectedInstalled: [{ object: node, revisions: ["installed"] }], reviewedVersions: [{ object: node, revisions: ["branch-a", "branch-b"] }, { object: schema, revisions: ["schema-head"] }], objects: [
    { object: node, view: { versions: [{ commit: "commit-a", revision: "branch-a", state: state() }, { commit: "commit-b", revision: "branch-b", state: { ...state(), name: "Other" } }], commonBases: [] }, originalDependencies: [], unsettledDependencies: [schema] },
    { object: schema, view: { versions: [{ commit: "schema-commit", revision: "schema-head", state: { parent: null, name: "Schema", retired: false, files: { "schema.json": "schema-digest" }, fileRevisions: { "schema.json": "schema-file-revision" } } }], commonBases: [] }, originalDependencies: [], unsettledDependencies: [] },
  ], files: [], coverage: "partial" };
}
function review(input: SyncRevalidationInput): SyncRevalidationReview { return { operationId: input.operationId, expectedInstalled: [{ object: node, revisions: ["installed"] }], reviewedVersions: cut().reviewedVersions, resolutions: input.resolutions.map(r => ({ ...r, reviewedDependencies: [{ object: schema, revisions: ["schema-head"] }] })), annotationTargets: [{ owner: { workspaceId: "workspace", nodeId: "note" }, annotationId: "discussion", state: "unavailable", reason: "preserved-awaiting-exact-target-qualification" }] }; }
function session(id: () => string = () => "original"): SyncRevalidationSession { const result = new SyncRevalidationSession("root", identity, cut(), id); result.select(node, "branch-a"); return result; }
function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } { let resolve!: (value: T) => void; return { promise: new Promise<T>(r => { resolve = r; }), resolve: value => resolve(value) }; }

test("preserve names all branches and keeps exact source/resource/discussion state; unrelated partial candidates do not block review", async () => {
  const current = session(); await current.inspect(async input => review(input));
  const request = current.request(); assert.equal(request.intent.kind, "revalidateSync");
  if (request.intent.kind !== "revalidateSync") throw new Error("wrong intent");
  assert.deepEqual(request.intent.resolutions[0], { object: node, revisions: ["branch-a", "branch-b"], decision: { kind: "preserve", revision: "branch-a" }, state: state(), reviewedDependencies: [{ object: schema, revisions: ["schema-head"] }] });
  assert.deepEqual(request.intent.expectedInstalled, [{ object: node, revisions: ["installed"] }]); assert.deepEqual(request.intent.reviewedVersions, cut().reviewedVersions); assert.deepEqual(request.intent.blobs, {});
  assert.equal(request.intent.resolutions.length, 1); assert.equal(current.review?.annotationTargets[0].state, "unavailable");
  request.intent.resolutions[0].state.name = "mutated"; assert.equal(current.request().intent.kind === "revalidateSync" && current.selection(node)?.state.name, "Note");
});

test("repair freezes only explicit UTF-8 file bytes with a fresh revision, retaining other files and unknown members", async () => {
  const current = session(), original = state();
  const source = '#link("weftext-node://workspace/unavailable")[ordinary unresolved link]\r\n';
  await current.repairText(node, "Note.typ", source); await current.inspect(async input => review(input));
  const request = current.request(); if (request.intent.kind !== "revalidateSync") throw new Error("wrong intent");
  const result = request.intent.resolutions[0], digest = await syncTextDigest(new TextEncoder().encode(source));
  assert.deepEqual(result.decision, { kind: "repair", baseRevision: "branch-a" }); assert.equal(result.state.files["Note.typ"], digest); assert.notEqual(result.state.fileRevisions["Note.typ"], original.fileRevisions["Note.typ"]);
  assert.deepEqual(request.intent.blobs, { [digest]: Array.from(new TextEncoder().encode(source)) });
  for (const path of ["resource/unknown.bin", "weftext.annotations.json"]) { assert.equal(result.state.files[path], original.files[path]); assert.equal(result.state.fileRevisions[path], original.fileRevisions[path]); }
  assert.equal(current.repairedText(node, "Note.typ"), source);
});

test("restoring equal original bytes retains their original file revision and removes unused repair blobs", async () => {
  const source = "\ufeffexact\r\nbytes", metadata = cut(), digest = await syncTextDigest(new TextEncoder().encode(source));
  metadata.objects[0].view.versions[0].state.files["Note.typ"] = digest;
  const current = new SyncRevalidationSession("root", identity, metadata); current.select(node, "branch-a");
  await current.repairText(node, "Note.typ", "repair"); await current.repairText(node, "Note.typ", source);
  assert.deepEqual(current.selection(node)?.decision, { kind: "preserve", revision: "branch-a" }); assert.equal(current.selection(node)?.state.fileRevisions["Note.typ"], "source-revision"); assert.deepEqual(current.input().blobs, {});
});

test("edited inputs, equal-byte ABA selection, external receive and an older concurrent read all retire late review responses", async () => {
  for (const mutate of [(s: SyncRevalidationSession) => s.touch(), (s: SyncRevalidationSession) => { s.select(node, "branch-b"); s.select(node, "branch-a"); }, (s: SyncRevalidationSession) => s.invalidate()]) {
    const current = session(), wait = deferred<SyncRevalidationReview>(), input = current.input(), result = current.inspect(() => wait.promise); mutate(current); wait.resolve(review(input));
    assert.equal(await result, false); assert.throws(() => current.request());
  }
  const current = session(), first = deferred<SyncRevalidationReview>(), input = current.input(), result = current.inspect(() => first.promise);
  await current.inspect(async r => review(r)); first.resolve(review(input)); assert.equal(await result, false); assert.ok(current.review);
});

test("mismatched choices, missing global branches and changed or duplicate actual dependencies never become confirmable", async () => {
  for (const mutate of [(r: SyncRevalidationReview) => { r.resolutions[0].state.name = "server-changed"; }, (r: SyncRevalidationReview) => { r.resolutions[0].decision = { kind: "preserve", revision: "branch-b" }; }, (r: SyncRevalidationReview) => { r.reviewedVersions[0].revisions = ["branch-a"]; }, (r: SyncRevalidationReview) => { r.resolutions[0].reviewedDependencies[0].revisions = ["late-schema"]; }, (r: SyncRevalidationReview) => { r.resolutions[0].reviewedDependencies.push(structuredClone(r.resolutions[0].reviewedDependencies[0])); }]) {
    const current = session(); await assert.rejects(current.inspect(async input => { const r = review(input); mutate(r); return r; })); assert.throws(() => current.request());
  }
});

test("exact file reads reject wrong owner/revision/path/digest and text altered in transit", async () => {
  const text = "\ufefforiginal\r\nsource", bytes = new TextEncoder().encode(text), digest = await syncTextDigest(bytes), metadata = cut(); metadata.objects[0].view.versions[0].state.files["Note.typ"] = digest;
  const current = new SyncRevalidationSession("root", identity, metadata); current.select(node, "branch-a");
  const file = { object: node, revision: "branch-a", fileRevision: "source-revision", path: "Note.typ", digest, bytes: bytes.length, text };
  assert.equal(await current.verifyText(node, file), text);
  for (const change of [{ revision: "branch-b" }, { object: schema }, { path: "other.typ" }, { digest: "wrong" }, { text: "changed" }, { bytes: bytes.length + 1 }]) await assert.rejects(current.verifyText(node, { ...file, ...change }));
});

test("a revalidation request survives transport mutation/lost preparation and original-ID recovery through the one SyncFlow", async () => {
  const current = session(); await current.inspect(async input => review(input)); const request = current.request(), bodies: unknown[] = []; let calls = 0, saved: SyncPending | null = null;
  const plan: SyncPlan = { operationId: "original", planId: "same-plan", status: "prepared", changes: [], losses: [] };
  const flow = new SyncFlow({ store: p => { saved = structuredClone(p); }, request: async <T>(_route: string, body: unknown): Promise<T> => { bodies.push(structuredClone(body)); if (calls++ === 0) { (body as { root: string }).root = "mutated by transport"; throw new Error("response lost"); } return plan as T; } });
  await assert.rejects(flow.prepare(request)); assert.ok(saved); await flow.retryPrepare(); assert.deepEqual(bodies[0], bodies[1]);
  const reload = new SyncFlow({ store: () => {}, request: async <T>(route: string) => { assert.equal(route, "/api/sync/operation"); return { request, plan } as T; } }); await reload.recover("original"); assert.deepEqual(reload.pending?.request, request);
});

test("review retries retain one proposed ID and prepare uses its exact echo without review-only intent fields", async () => {
  let ids = 0; const current = session(() => `review-${++ids}`), inputs: SyncRevalidationInput[] = [];
  await assert.rejects(current.inspect(async input => { inputs.push(input); throw new Error("review response lost"); }));
  assert.throws(() => current.request());
  for (let i = 0; i < 2; i++) await current.inspect(async input => { inputs.push(input); return review(input); });
  assert.equal(ids, 1); assert.deepEqual(inputs.map(input => input.operationId), ["review-1", "review-1", "review-1"]);
  const request = current.request(); assert.equal(request.operationId, "review-1");
  assert.equal("operationId" in request.intent, false); assert.equal("annotationTargets" in request.intent, false);
  const copy = current.review!; copy.operationId = "another"; assert.equal(current.request().operationId, "review-1");
});

test("missing or foreign review ID never authorizes prepare, even with identical choices and dependencies", async () => {
  const current = session();
  for (const operationId of [undefined, "other-operation"]) {
    await assert.rejects(current.inspect(async input => ({ ...review(input), operationId })), /原操作编号/);
    assert.equal(current.review, null); assert.throws(() => current.request());
  }
  await current.inspect(async input => review(input)); assert.equal(current.request().operationId, "original");
});

test("selection, repair and draft-input changes retire proposed IDs while an older same-ID read cannot replace a newer review", async () => {
  let ids = 0; const current = session(() => `review-${++ids}`), first = deferred<SyncRevalidationReview>();
  let firstInput!: SyncRevalidationInput;
  const late = current.inspect(input => { firstInput = input; return first.promise; });
  await current.inspect(async input => review(input)); assert.equal(current.request().operationId, "review-1");
  first.resolve(review(firstInput)); assert.equal(await late, false); assert.equal(current.request().operationId, "review-1");
  for (const change of [() => { current.select(node, "branch-b"); current.select(node, "branch-a"); }, () => current.touch(), () => current.repairText(node, "Note.typ", "explicit repair")]) {
    await change(); assert.throws(() => current.request());
    await current.inspect(async input => review(input)); assert.equal(current.request().operationId, `review-${ids}`);
  }
  assert.equal(ids, 4);
  const old = deferred<SyncRevalidationReview>(); let oldInput!: SyncRevalidationInput;
  const pending = current.inspect(input => { oldInput = input; return old.promise; }); current.invalidate(); old.resolve(review(oldInput));
  assert.equal(await pending, false); assert.throws(() => current.request());
});

test("durable Flow handoff alone consumes review; storage failure retains repair input and unknown/cancel keep the original ID", async () => {
  let ids = 0, quota = true, consumed = 0, saved: SyncPending | null = null;
  const current = session(() => `action-${++ids}`); await current.repairText(node, "Note.typ", "my private repair");
  await current.inspect(async input => review(input)); const request = current.request(), bodies: { path: string; body: unknown }[] = [];
  const plan: SyncPlan = { operationId: request.operationId, planId: "original-plan", status: "prepared", changes: [], losses: [] };
  let prepareCalls = 0;
  const host = {
    store: (p: SyncPending | null): void => { if (p && quota) { quota = false; throw new Error("storage full"); } saved = structuredClone(p); },
    request: async <T>(path: string, body: unknown): Promise<T> => {
      assert.equal(saved?.request.operationId, "action-1"); bodies.push({ path, body: structuredClone(body) });
      if (path.endsWith("prepare") && prepareCalls++ === 0) throw new Error("prepare response lost");
      if (path.endsWith("commit")) throw new Error("commit response lost");
      return { ...plan, status: path.endsWith("cancel-prepared") ? "resolved" : "prepared" } as T;
    },
  };
  const flow = new SyncFlow(host), consume = (): void => { consumed++; current.invalidate(); };
  await assert.rejects(flow.prepare(request, consume), /storage full/);
  assert.equal(consumed, 0); assert.equal(bodies.length, 0); assert.equal(saved, null);
  assert.equal(current.repairedText(node, "Note.typ"), "my private repair"); assert.deepEqual(current.request(), request);
  flow.forget();
  await assert.rejects(flow.prepare(current.request(), consume), /prepare response lost/);
  assert.equal(consumed, 1); assert.throws(() => current.request()); assert.equal(ids, 1);
  const reload = new SyncFlow(host, saved); await reload.retryPrepare();
  assert.deepEqual(bodies[0], bodies[1]); assert.deepEqual(reload.pending?.request, request);
  await assert.rejects(reload.commit(), /commit response lost/); assert.throws(() => reload.forget());
  await reload.inspect(); await reload.cancelPrepared(); assert.equal(reload.pending?.status, "resolved"); reload.forget();
  assert.equal(consumed, 1); assert.throws(() => current.request()); assert.equal(ids, 1);
  assert.deepEqual(bodies.slice(2).map(call => call.body), [{ operationId: "action-1", planId: "original-plan" }, { operationId: "action-1" }, { operationId: "action-1", planId: "original-plan" }]);
  const next = session(() => `action-${++ids}`); await next.inspect(async input => review(input)); assert.equal(next.request().operationId, "action-2");
});
