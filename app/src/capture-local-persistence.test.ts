import assert from "node:assert/strict";
import test from "node:test";
import { CaptureController } from "./capture-controller";
import { LocalCapturePersistence } from "./capture-local-persistence";
import { captureInputFingerprint, captureText, emptyCaptureInput, type CaptureDraftRecord, type CaptureMutationPayload, type CaptureTransport } from "./capture-draft-state";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";

async function fixture(commitBeforeFailure:boolean) {
  let bundle:PersonalInputBundle|undefined;
  const backend={async read(){return structuredClone(bundle);},async compareAndWrite(_audience:string,expected:number,metadata:string,texts:ReadonlyMap<string,string>){bundle={commitId:expected+1,metadata,texts:new Map(texts)};return expected+1;}};
  const store=new PersonalInputRecovery(backend),audience=JSON.stringify(["device","local"]);await store.open(audience);
  let draft:CaptureDraftRecord={version:2,draftId:crypto.randomUUID(),revision:1,inputRevision:0,inputCut:"full",lastMutation:{kind:"create",mutationId:crypto.randomUUID(),fingerprint:"a".repeat(64),resultingRevision:1},target:null,pending:null,currentInput:emptyCaptureInput()};
  const calls:{route:string;body:unknown}[]=[];let fail=true;
  const request:CaptureTransport=async<T>(route:string,body:unknown):Promise<T>=>{
    calls.push({route,body:structuredClone(body)});
    if(route.endsWith("/input-mutation")) {
      const query=body as {expectedRevision:number;mutationId:string;fingerprint:string};
      return {status:draft.lastMutation.kind==="input" && draft.lastMutation.mutationId===query.mutationId && draft.lastMutation.fingerprint===query.fingerprint?"applied":"supersededOrUnknown",draft:structuredClone(draft)} as T;
    }
    if(route.endsWith("/mutation"))return {status:"supersededOrUnknown",draft:structuredClone(draft)} as T;
    if(route.endsWith("/input")) {
      const command=body as CaptureMutationPayload["input"];
      if(!fail||commitBeforeFailure) {
        assert.equal(command.expectedRevision,draft.revision);
        draft={...draft,revision:draft.revision+1,inputRevision:draft.inputRevision+1,inputCut:"full",currentInput:structuredClone(command.input),lastMutation:{kind:"input",mutationId:command.mutationId,fingerprint:await captureInputFingerprint(command.expectedRevision,command.input),resultingRevision:draft.revision+1}};
      }
      if(fail)throw new Error("controlled full response failure");
      return structuredClone(draft) as T;
    }
    throw new Error(`Unexpected ${route}`);
  };
  const persistence=new LocalCapturePersistence(request,store,audience,crypto.randomUUID()),controller=new CaptureController(persistence,draft);
  const input=emptyCaptureInput();input.body=captureText("PROTECTED DEFAULT+OWN\ud800\0");
  input.authorTexts.body=[{text:captureText("OWN\ud800\0"),provenance:{producerId:crypto.randomUUID(),inputId:"a".repeat(32),revision:1}}];
  input.fields=[{fieldId:"private/default",valueState:{kind:"valid",value:{kind:"text",value:"PROTECTED DEFAULT"}}}];
  return {backend,store,audience,calls,controller,input,persistence,proceed(){fail=false;},bundle:()=>bundle,draft:()=>draft};
}
test("normal authorized full input is one atomic Core command with own proof; lost response queries original digest",async()=>{
  const f=await fixture(true);f.controller.edit(f.input);await f.controller.drain();
  assert.equal(f.calls.filter(c=>c.route.endsWith("/input")).length,1);
  assert.equal(f.calls.filter(c=>c.route.endsWith("/author-input")).length,0);
  const full=f.calls.find(c=>c.route.endsWith("/input"))!.body as CaptureMutationPayload["input"],query=f.calls.find(c=>c.route.endsWith("/input-mutation"))!.body;
  assert.deepEqual(query,{draftId:full.draftId,expectedRevision:1,mutationId:full.mutationId,fingerprint:await captureInputFingerprint(1,full.input)});
  assert.equal(f.draft().revision,2);assert.equal(f.controller.state.blocked,false);assert.equal(f.store.captureTransports(f.audience).length,0);
});
test("unknown full failure retains pure proposal and original identity, never author-only overwrite; retry queries then reuses command",async()=>{
  const f=await fixture(false);f.controller.edit(f.input);await assert.rejects(f.controller.drain());
  assert.equal(f.controller.state.blocked,true);await assert.rejects(f.controller.prepareClose(()=>{}));
  const row=f.store.captureTransports(f.audience)[0];assert.equal(row.pending!.command.kind,"fullInput");assert.equal(row.input.texts.body[0].text,"OWN\ud800\0");
  assert.doesNotMatch(f.bundle()!.metadata+[...f.bundle()!.texts.values()].join(""),/PROTECTED DEFAULT/);
  const restart=new PersonalInputRecovery(f.backend);await restart.open(f.audience);
  const recovery=new LocalCapturePersistence(async<T>(route:string,body:unknown):Promise<T>=>{assert.equal(route,"/api/capture/draft/input-mutation");assert.deepEqual(body,{draftId:row.owner.draftId,expectedRevision:row.pending!.command.expectedRevision,mutationId:row.pending!.command.mutationId,fingerprint:row.pending!.command.fingerprint});return {status:"supersededOrUnknown",draft:f.draft()} as T;},restart,f.audience,crypto.randomUUID());
  assert.equal((await recovery.recover(row.owner.draftId)).length,1);
  const first=f.calls.find(c=>c.route.endsWith("/input"))!;f.proceed();await f.controller.retryPersistence();
  assert.deepEqual(f.calls.filter(c=>c.route.endsWith("/input"))[1],first);
  assert.equal(f.calls.filter(c=>c.route.endsWith("/author-input")).length,0);assert.equal(f.draft().revision,2);
});
