import assert from "node:assert/strict";
import test from "node:test";
import { CaptureCloseGuard } from "./capture-close";
import { CaptureController, type CapturePersistencePort } from "./capture-controller";
import { captureText, emptyCaptureInput, type CaptureDraftRecord, type CaptureMutationPayload } from "./capture-draft-state";
import { PersonalControlInput } from "./personal-control-input";

async function fixture(run:(f:ReturnType<typeof setup>)=>Promise<void>) {
  class Control { value="";dataset={};isConnected=true;readOnly=false;disabled=false;type="text";selectionStart=0;selectionEnd=0;matches(){return false;} }
  const previous=[globalThis.HTMLInputElement,globalThis.HTMLTextAreaElement];
  Object.assign(globalThis,{HTMLInputElement:Control,HTMLTextAreaElement:Control});
  function setup(){
    let record:CaptureDraftRecord={version:2,draftId:crypto.randomUUID(),revision:1,inputRevision:1,inputCut:"full",lastMutation:{kind:"input",mutationId:crypto.randomUUID(),fingerprint:"a".repeat(64),resultingRevision:1},currentInput:emptyCaptureInput(),target:null,pending:null};
    let unavailable=false,unresolved=false,inert=false,flushes=0;
    const port:CapturePersistencePort<CaptureDraftRecord>={
      async mutate(route,payload){
        if(unavailable)throw new Error("No Core durability ACK");
        assert.equal(route,"input");const request=payload as CaptureMutationPayload["input"];
        assert.equal(request.expectedRevision,record.revision);
        record={...record,revision:record.revision+1,inputRevision:record.inputRevision+1,currentInput:structuredClone(request.input),lastMutation:{kind:"input",mutationId:request.mutationId,fingerprint:"a".repeat(64),resultingRevision:record.revision+1}};
        return structuredClone(record);
      },
      async flush(){if(unavailable)throw new Error("No Core durability ACK");flushes++;return{draftId:record.draftId,recordRevision:record.revision,inputRevision:record.inputRevision,inputCut:"full",lastMutationId:record.lastMutation.mutationId,pendingOperationId:null,recordDigest:"b".repeat(64)};},
    };
    const controller=new CaptureController(port,record),listeners=new Map<string,((event:unknown)=>void)[]>();
    const doc={addEventListener(type:string,listener:(event:unknown)=>void){listeners.set(type,[...listeners.get(type)??[],listener]);}};
    const inputs=new PersonalControlInput(doc as unknown as Document,()=>true,()=>{throw new Error("Capture cannot use generic raw recovery");});
    const body=new Control() as unknown as HTMLInputElement;
    const session=inputs.session("actual-owner",1,{kind:"capture",draftId:record.draftId},()=>true,snapshot=>{controller.observeAuthorCut(snapshot);const input=controller.state.input;input.body=captureText(snapshot.rawValue);controller.edit(input);});
    session.bind(body,"body");
    const fire=(control:HTMLInputElement,type:string,data="")=>{for(const listener of listeners.get(type)??[])listener({target:control,data,inputType:"insertText"});};
    const type=(text:string)=>{fire(body,"beforeinput");body.value+=text;body.selectionStart=body.selectionEnd=body.value.length;fire(body,"input",text);};
    const guard=new CaptureCloseGuard(controller,inputs,{assertResolved(){if(unresolved)throw new Error("Original proposal unresolved");},collectOwnedRaw(){},freeze(){inert=true;},thaw(){inert=false;}});
    return{controller,inputs,session,body,type,fire,guard,offline(){unavailable=true;},unresolved(){unresolved=true;},inert:()=>inert,flushes:()=>flushes};
  }
  try{await run(setup());}finally{Object.assign(globalThis,{HTMLInputElement:previous[0],HTMLTextAreaElement:previous[1]});}
}

test("capture close covers only observed actual producer cuts and freezes their complete registry",()=>fixture(async f=>{
  f.type("Own\ud800\0");
  assert.equal(f.session.inspect().unapplied,true);
  const proof=await f.guard.prepare();
  assert.equal(f.flushes(),1);assert.equal(f.session.inspect().unapplied,false);
  assert.equal(f.guard.freeze(proof),true);assert.equal(f.body.readOnly,true);assert.equal(f.inert(),true);
  assert.throws(()=>f.inputs.session("actual-owner",1,{kind:"library",role:"schema"},()=>true),/冻结/);
  f.guard.thaw();assert.equal(f.body.readOnly,false);assert.equal(f.inert(),false);
}));

test("new producer membership and equal-byte edits invalidate an earlier Core cut",()=>fixture(async f=>{
  f.type("A");const proof=await f.guard.prepare();
  f.inputs.session("actual-owner",1,{kind:"library",role:"schema"},()=>true);
  assert.equal(f.guard.freeze(proof),false);
  const next=await f.guard.prepare();
  f.controller.edit(f.controller.state.input);
  assert.equal(f.guard.freeze(next),false);
  await f.controller.drain();
}));

test("unknown Core persistence, pending IME, async delivery and unresolved proposal never become a close ACK",()=>fixture(async f=>{
  const lease=f.session.capture(f.body)!;
  await assert.rejects(f.guard.prepare());assert.equal(f.flushes(),0);
  lease.release();f.fire(f.body,"compositionstart");
  await assert.rejects(f.guard.prepare());
  f.fire(f.body,"compositionend");f.unresolved();
  await assert.rejects(f.guard.prepare(),/unresolved/);
}));

test("a retained retired capture input needs real Core acknowledgement before its producer is disposed",()=>fixture(async f=>{
  f.type("retired-own");f.session.retire();
  assert.equal(f.inputs.inspect().producers[0].unapplied,true);
  f.offline();await assert.rejects(f.guard.prepare());
  assert.equal(f.inputs.inspect().producers[0].unapplied,true);
  assert.equal(f.inert(),false);
}));
