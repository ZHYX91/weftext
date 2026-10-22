import assert from "node:assert/strict";
import test from "node:test";
import { LocalCaptureSession, type CaptureResultIdentity } from "./capture-local";
import { emptyCaptureInput, type CaptureDraftRecord, type CaptureTransport } from "./capture-draft-state";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";
import type { OpenedDocument } from "./types";

test("local committed-result redelivery reuses its Core frozen request and advances only after actual document delivery",async()=>{
  let bundle:PersonalInputBundle|undefined;
  const recovery=new PersonalInputRecovery({async read(){return structuredClone(bundle);},async compareAndWrite(_audience,expected,metadata,texts){bundle={commitId:expected+1,metadata,texts:new Map(texts)};return expected+1;}});
  const operationId=crypto.randomUUID(),draftId=crypto.randomUUID();
  let draft:CaptureDraftRecord={version:2,draftId,revision:2,inputRevision:1,inputCut:"full",lastMutation:{kind:"input",mutationId:crypto.randomUUID(),fingerprint:"a".repeat(64),resultingRevision:2},currentInput:emptyCaptureInput(),target:null,pending:{operationId,requestKind:"capture",wire:"original controlled fixture wire",wireDigest:"b".repeat(64),inputRevision:1}};
  const document:OpenedDocument={sessionId:"actual-delivery-fixture-session",path:"fixture.typ",root:"fixture",source:"original created bytes",baseId:"fixture-base",kind:"managed"};
  let allowDelivery=false,delivered=0;
  const calls:{path:string;body:unknown}[]=[];
  const request:CaptureTransport=async<T>(path:string,body:unknown):Promise<T>=>{
    calls.push({path,body:structuredClone(body)});
    if(path==="/api/capture/draft/read")return structuredClone(draft) as T;
    if(path==="/api/capture/identity")return{id:crypto.randomUUID(),isCurrent:true} as T;
    if(path==="/api/operations/status")return{operationId,status:"committed"} as T;
    if(path==="/api/capture/draft/open-result"){
      assert.deepEqual(body,{draftId,operationId});assert.equal(draft.pending?.operationId,operationId);
      if(!allowDelivery)throw new Error("controlled original open failure");
      return structuredClone(document) as T;
    }
    if(path==="/api/capture/draft/advance"){
      const command=body as {expectedRevision:number;mutationId:string;operationId:string;resetInput:boolean};
      assert.equal(delivered,1);assert.equal(command.operationId,operationId);assert.equal(command.expectedRevision,draft.revision);
      draft={...draft,revision:draft.revision+1,lastMutation:{kind:"input",mutationId:command.mutationId,fingerprint:"c".repeat(64),resultingRevision:draft.revision+1},pending:null};
      return structuredClone(draft) as T;
    }
    throw new Error(`Unexpected request ${path}`);
  };
  const session=await LocalCaptureSession.open(request,{draftId,recovery});
  const deliver=async(original:CaptureResultIdentity)=>{const actual=await request<OpenedDocument>("/api/capture/draft/open-result",original);assert.deepEqual(actual,document);delivered++;};
  await assert.rejects(session.save("open",deliver,async()=>assert.fail("No close")),/controlled original open failure/);
  assert.equal(session.controller.state.draft.pending?.operationId,operationId);
  assert.equal(calls.some(call=>call.path.endsWith("/advance")),false);
  allowDelivery=true;
  await session.save("open",deliver,async()=>assert.fail("No close"));
  assert.equal(session.controller.state.draft.pending,null);
  assert.equal(calls.filter(call=>call.path.endsWith("/open-result")).length,2);
  assert.equal(calls.some(call=>call.path.endsWith("/freeze")||call.path.endsWith("/create")||call.path.endsWith("/submit")),false);
});
