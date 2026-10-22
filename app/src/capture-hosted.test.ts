import assert from "node:assert/strict";
import test from "node:test";
import { HostedCaptureSession, type HostedCaptureRequest, type HostedCaptureView } from "./capture-hosted";
import { captureText, captureString, emptyCaptureAuthorTexts } from "./capture-draft-state";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";
import type { StructureCatalog } from "./hosted-structure-state";
function storage() {
  let bundle: PersonalInputBundle | undefined;
  return { store: new PersonalInputRecovery({ async read() { return structuredClone(bundle); }, async compareAndWrite(_audience, expected, metadata, texts) { bundle = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1; } }), bytes: () => bundle ? bundle.metadata + [...bundle.texts.values()].join("") : "" };
}
function view(): HostedCaptureView {
  const draftId = crypto.randomUUID(), provenance = { producerId: crypto.randomUUID(), inputId: "a".repeat(32), revision: 2 }, own = { fieldId: "private/text", valuePath: [] as [], controlRole: "text", text: captureText("OWN\ud800\0"), provenance };
  return { recovery: { draftId, revision: 3, inputRevision: 2, inputCut: "full", lastMutation: { kind:"input", mutationId: crypto.randomUUID(), fingerprint: "a".repeat(64), resultingRevision: 3 }, pending: null,
    texts:{...emptyCaptureAuthorTexts(),body:[{text:captureText("OWN BODY"),provenance}]}, bodyMode: "text", rawInputs: [own], detachedRawInputs: [] },
  active: { slot: "approved", textInputs:{name:captureText(""),title:captureText(""),body:captureText("PRIVATE BODYOWN BODY"),rootInput:captureText(""),parentInput:captureText(""),parametersText:captureText("")},bodyMode:"text", typeIds: ["private/type"], fields: [{ fieldId: "private/text", valueState: { kind: "valid", value: { kind: "text", value: "PRIVATE DEFAULT" } } }], templateId: null, formBasis: { schemaRef: "private-schema", semanticDigest: "a".repeat(64) }, rawInputs: [{ fieldId: own.fieldId, valuePath: [], controlRole: "text", text: captureText("PRIVATE DEFAULTOWN\ud800\0"), binding: "exact", author: { text: own.text, provenance } }], detachedRawInputs: [] } };
}
const catalog: StructureCatalog = { scope: "configuredAuthorizedStructure", coverage: "complete", sources: [], slots: [], templates: [] };
test("retired hosted producer stages only proved text; a fresh pure projection cannot recover its former protected candidate", async () => {
  const initial = view(), persisted = storage(); let current = true, privateReads = 0;
  const request: HostedCaptureRequest = async <T>(path: string): Promise<T> => { assert.equal(path, "capture/draft/read"); privateReads++; return structuredClone(initial) as T; };
  const options = { deployment: "deployment", actor: "alice", draftId: initial.recovery.draftId, recovery: persisted.store, current: () => current };
  const first = await HostedCaptureSession.open(request, options); assert.equal(first.controller.state.input.fields.length, 1);
  current = false; first.controller.edit(first.controller.state.input); await assert.rejects(first.controller.drain());
  await persisted.store.flush(JSON.stringify(["deployment", "alice"]));
  assert.doesNotMatch(persisted.bytes(), /PRIVATE BODY|PRIVATE DEFAULT|private-schema|formBasis|templateId|wire/); assert.equal(privateReads, 2);
  const pure: HostedCaptureView = { recovery: initial.recovery, active: null }; current = true;
  const second = await HostedCaptureSession.open(async <T>() => structuredClone(pure) as T, options);
  assert.equal(second.controller.state.formReady,false); assert.deepEqual(second.controller.state.input.fields, []); assert.equal(second.controller.state.input.rawInputs[0].binding, "copyOnly"); assert.equal(captureString(second.controller.state.input.rawInputs[0].text), "OWN\ud800\0");
  assert.throws(() => second.prepare(catalog), /原 producer|恢复投影/);
  await assert.rejects(first.controller.retryPersistence()); assert.equal(privateReads, 2); // Equal actor/current ABA cannot revive the retired transport.
});
test("committed creation delivery failure keeps its original pending identity and retries only that original fact", async () => {
  const original = view(), persisted = storage(), operationId = crypto.randomUUID(), paths: string[] = [];
  original.recovery.pending = { operationId, requestKind: "hostedStructure", inputRevision: original.recovery.inputRevision };
  const fact = { resource: "approved", operationId, status: "committed", commitAllowed: false, result: { resource: "created-member", canOpen: true, status: "available", nodeRef: { workspaceId: crypto.randomUUID(), nodeId: crypto.randomUUID() } } };
  const request: HostedCaptureRequest = async <T>(path: string): Promise<T> => { paths.push(path); if (path === "capture/draft/read") return structuredClone(original) as T; assert.equal(path, "capture/draft/operation"); return structuredClone(fact) as T; };
  const session = await HostedCaptureSession.open(request, { deployment: "deployment", actor: "alice", draftId: original.recovery.draftId, recovery: persisted.store, current: () => true });
  for (let attempt = 0; attempt < 2; attempt++) await assert.rejects(session.save("open", catalog, async () => { throw new Error("controlled document delivery failure"); }, async () => assert.fail("No close")), /controlled document delivery failure/);
  assert.equal(session.controller.state.draft.pending!.operationId, operationId);
  assert.deepEqual(paths, ["capture/draft/read", "capture/draft/read", "capture/draft/operation", "capture/draft/operation"]);
  assert.equal(persisted.bytes(), "");
});

test("author-only restart ignores former authorized private form and keeps proved own input copy-only",async()=>{
 const original=view();original.recovery.inputCut="authorOnly";original.recovery.lastMutation.kind="authorInput";
 const recovery=storage();
 const session=await HostedCaptureSession.open(async<T>()=>structuredClone(original) as T,{deployment:"deployment",actor:"alice",draftId:original.recovery.draftId,recovery:recovery.store,current:()=>true});
 assert.equal(session.controller.state.formReady,false);assert.equal(session.controller.state.draft.active,null);
 assert.equal(captureString(session.controller.state.input.body),"");assert.deepEqual(session.controller.state.input.fields,[]);
 assert.equal(session.controller.state.input.rawInputs[0].binding,"copyOnly");assert.equal(captureString(session.controller.state.input.rawInputs[0].text),"OWN\ud800\0");
 assert.throws(()=>session.prepare(catalog),/恢复投影/);
});
