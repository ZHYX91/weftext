import test from "node:test";
import assert from "node:assert/strict";
import { LocalCaptureWorkspace } from "./capture-workspace";
import { LocalCaptureSession, captureDraftPointerKey } from "./capture-local";
import { CaptureCloseGuard } from "./capture-close";
import { PersonalControlInput } from "./personal-control-input";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";
import { emptyCaptureInput, captureText, captureInputFingerprint, type CaptureDraftRecord, type CaptureTransport } from "./capture-draft-state";
import type { MountedLocalCapture, mountLocalCapture } from "./capture-mount";
import type { NativeSurface } from "./native-host";
import { thawNativeApi } from "./native-api-gate";

function held<T>() { let resolve!: (value:T)=>void; const promise=new Promise<T>(accept=>{resolve=accept;});return{promise,resolve}; }
class Node {
  nodeType=1; childNodes:Node[]=[]; dataset:Record<string,string>={}; textContent=""; value=""; className=""; inert=false; isConnected=true; readOnly=false;disabled=false;type="text";id="";selectionStart=0;selectionEnd=0;
  constructor(readonly ownerDocument:Doc){}
  get options(){return this.childNodes;}
  get firstChild(){return this.childNodes[0]??null;}
  appendChild(node:Node){this.childNodes.push(node);return node;}
  append(...nodes:Node[]){for(const node of nodes)this.appendChild(node);}
  insertBefore(node:Node,before:Node|null){const at=before?this.childNodes.indexOf(before):-1;if(at<0)this.childNodes.push(node);else this.childNodes.splice(at,0,node);return node;}
  replaceChildren(...nodes:Node[]){this.childNodes=nodes;}
  before(..._nodes:Node[]){}
  setAttribute(_name:string,_value:string){}
  remove(){this.isConnected=false;}
  matches(){return false;}
}
class TextArea extends Node {}
class Doc {
  listeners=new Map<string,((event:any)=>void)[]>();
  createElement(tag:string){return tag==="textarea"?new TextArea(this):new Node(this);}
  createTextNode(text:string){const node=new Node(this);node.nodeType=3;node.textContent=text;return node;}
  addEventListener(type:string,listener:(event:any)=>void){this.listeners.set(type,[...this.listeners.get(type)??[],listener]);}
  fire(type:string,target:Node){for(const listener of this.listeners.get(type)??[])listener({target,inputType:"insertText",data:target.value});}
}
const close={surfaceId:"capture-surface",surfaceEpoch:"surface-epoch",closeAttemptId:"attempt"};
const clone=<T>(value:T):T=>structuredClone(value);

// Real LocalCaptureSession + ordered controller/outbox/PersonalRecovery + real
// producer registry/close guard/NativeCapture/workspace. Only DOM rendering and
// Core transport are substituted; no handoff/close decision is copied.
async function fixture(run:(f:Awaited<ReturnType<typeof setup>>)=>Promise<void>){
  const prior=["document","HTMLInputElement","HTMLTextAreaElement"].map(key=>Object.getOwnPropertyDescriptor(globalThis,key));
  const document=new Doc();
  Object.defineProperties(globalThis,{document:{configurable:true,value:document},HTMLInputElement:{configurable:true,value:Node},HTMLTextAreaElement:{configurable:true,value:TextArea}});
  thawNativeApi();
  async function setup(){
    const device=crypto.randomUUID(),a=crypto.randomUUID(),data=new Map<string,CaptureDraftRecord>(),storage=new Map<string,string>(),calls:{path:string;body:any}[]=[];
    let browserFailures=0,identityOverride:unknown;
    let deviceId:string=device,failBrowserWrites=false,failNewPointer=false,failInput=false,failCreate=false,loseCreateReply=false,failLookup=false,failMount=false,createWait:ReturnType<typeof held<void>>|undefined,inputWait:ReturnType<typeof held<void>>|undefined,flushWaits:ReturnType<typeof held<void>>[]=[],failPointer=false;
    const createEntered=held<void>();
    const make=(id:string,mutationId=crypto.randomUUID(),input=emptyCaptureInput()):CaptureDraftRecord=>({version:2,draftId:id,revision:1,inputRevision:1,inputCut:"full",lastMutation:{kind:"create",mutationId,fingerprint:"a".repeat(64),resultingRevision:1},currentInput:clone(input),target:null,pending:null});
    const original=make(a);original.currentInput.body=captureText("A original\r\n你");data.set(a,original);storage.set(captureDraftPointerKey,a);
    const request:CaptureTransport=async<T>(path:string,body:any):Promise<T>=>{
      calls.push({path,body:clone(body)});
      if(path==="/api/capture/identity")return(identityOverride??{id:deviceId,isCurrent:true}) as T;
      if(path==="/api/capture/draft/list")return[...data.values()].map(clone) as T;
      if(path==="/api/capture/draft/read"){const current=data.get(body.draftId);if(!current)throw new Error("unavailable original draft");return clone(current) as T;}
      if(path==="/api/capture/draft/create"){
        createEntered.resolve();
        if(createWait)await createWait.promise;
        if(failCreate)throw new Error("create did not arrive");
        const existing=data.get(body.draftId),fingerprint=await captureInputFingerprint(0,body.input);
        if(existing){
          if(existing.lastMutation.mutationId!==body.mutationId)throw new Error("original draft ID already exists");
          if(existing.lastMutation.fingerprint!==fingerprint)throw new Error("same mutation belongs to different input");
        }else{const created=make(body.draftId,body.mutationId,body.input);created.lastMutation.fingerprint=fingerprint;data.set(body.draftId,created);}
        if(loseCreateReply)throw new Error("create reply lost");return clone(data.get(body.draftId)) as T;
      }
      if(path==="/api/capture/draft/mutation"){
        if(failLookup)throw new Error("lookup unavailable");const current=data.get(body.draftId);if(!current)throw new Error("lookup unavailable");
        const fingerprint=body.command.kind==="create"?await captureInputFingerprint(0,body.command.input):current.lastMutation.fingerprint;
        return{status:current.lastMutation.mutationId!==body.mutationId?"supersededOrUnknown":current.lastMutation.fingerprint===fingerprint?"applied":"differentCommand",draft:clone(current)} as T;
      }
      const current=data.get(body.draftId);if(!current)throw new Error("missing draft");
      if(path==="/api/capture/draft/input"){
        if(inputWait)await inputWait.promise;
        if(failInput)throw new Error("original input persistence unavailable");
        assert.equal(body.expectedRevision,current.revision);
        const next={...current,revision:current.revision+1,inputRevision:current.inputRevision+1,currentInput:clone(body.input),lastMutation:{kind:"input" as const,mutationId:body.mutationId,fingerprint:await captureInputFingerprint(body.expectedRevision,body.input),resultingRevision:current.revision+1}};
        data.set(body.draftId,next);return clone(next) as T;
      }
      if(path==="/api/capture/draft/input-mutation")return{status:current.lastMutation.mutationId===body.mutationId&&current.lastMutation.fingerprint===body.fingerprint?"applied":"unknown",draft:clone(current)} as T;
      if(path==="/api/capture/draft/flush"){
        const wait=flushWaits.shift();if(wait)await wait.promise;
        assert.equal(body.recordRevision,current.revision);
        return{draftId:current.draftId,recordRevision:current.revision,inputRevision:current.inputRevision,inputCut:"full",lastMutationId:current.lastMutation.mutationId,pendingOperationId:current.pending?.operationId??null,recordDigest:"b".repeat(64)} as T;
      }
      throw new Error(`Unexpected ${path}`);
    };
    const bundles=new Map<string,PersonalInputBundle>();
    const recovery=new PersonalInputRecovery({async read(audience){return clone(bundles.get(audience));},async compareAndWrite(audience,expected,metadata,texts){if(failBrowserWrites){browserFailures++;throw new Error("browser storage write unavailable");}assert.equal(expected,bundles.get(audience)?.commitId??0);bundles.set(audience,{commitId:expected+1,metadata,texts:new Map(texts)});return expected+1;}});
    const pointerStorage={getItem(key:string){return storage.get(key)??null;},setItem(key:string,value:string){if(failNewPointer&&key==="weftext:capture:new-draft")throw new Error("new locator unavailable");if(failPointer&&key===captureDraftPointerKey)throw new Error("active pointer unavailable");storage.set(key,value);}};
    const mounts:{mounted:MountedLocalCapture;deliver:NonNullable<import("./capture-mount").CaptureMountPorts["deliverCommitted"]>;control:Node;rootControl:Node;inputs:PersonalControlInput;retired:boolean;busy:boolean}[]=[];
    const mount:typeof mountLocalCapture=async(host,ports)=>{
      if(failMount)throw new Error("offscreen mount failed");
      const session=await LocalCaptureSession.open(request,{...ports.options,recovery});
      let live=true,inert=false;
      const inputs=new PersonalControlInput(document as unknown as Document,()=>live,()=>{throw new Error("No borrowed recovery");});
      const control=new TextArea(document);control.value=String.fromCharCode(...session.controller.state.input.body.utf16);
      const rootControl=new Node(document);rootControl.value=String.fromCharCode(...session.controller.state.input.rootInput.utf16);
      const author=inputs.session("fixture-capture",1,{kind:"capture",draftId:session.controller.state.draft.draftId},()=>live,snapshot=>{session.controller.observeAuthorCut(snapshot);const input=session.controller.state.input;input[snapshot.context.slot as "body"|"rootInput"]=captureText(snapshot.rawValue);session.controller.edit(input);},()=>session.controller.editRevision);
      author.bind(control as unknown as HTMLTextAreaElement,"body");author.bind(rootControl as unknown as HTMLInputElement,"rootInput");
      const collect=()=>{if(live){const input=session.controller.state.input;let changed=false;for(const[slot,node]of[["body",control],["rootInput",rootControl]]as const){if(String.fromCharCode(...input[slot].utf16)!==node.value){input[slot]=captureText(node.value);changed=true;}}if(changed)session.controller.edit(input);}};
      const row={} as typeof mounts[number];
      const guard=new CaptureCloseGuard(session.controller,inputs,{assertResolved:()=>session.assertRecovered(),collectOwnedRaw:collect,freeze(){inert=true;},thaw(){inert=false;}});
      const mounted={session,guard,form:{collectOwnedRaw:collect,showError(){}},assertCloseReady(){if(!live||row.busy)throw new Error("form action pending");},dispose(){live=false;row.retired=true;session.controller.retire();inputs.clear();}} as unknown as MountedLocalCapture;
      Object.assign(row,{mounted,deliver:ports.deliverCommitted!,control,rootControl,inputs,retired:false,busy:false});mounts.push(row);host.appendChild?.(control as unknown as HTMLElement);
      return mounted;
    };
    let participant:import("./native-host").CloseParticipant|undefined;
    const listeners=new Set<(value:Record<string,unknown>)=>void>(),sent:Record<string,unknown>[]=[];
    const surface={listen(listener:(value:Record<string,unknown>)=>void){listeners.add(listener);return()=>{listeners.delete(listener);};},post(value:Record<string,unknown>){sent.push(value);},register(_owner:string,value:typeof participant){participant=value;return()=>{participant=undefined;};},markReady(){}} as unknown as NativeSurface;
    const workspaces:LocalCaptureWorkspace[]=[];
    const workspace=()=>{const value=new LocalCaptureWorkspace(new Node(document) as unknown as HTMLElement,{request,options:{pointerStorage,recovery}},surface,mount);workspaces.push(value);return value;};
    const first=workspace();await first.open();
    const type=(row:typeof mounts[number],text:string)=>{document.fire("beforeinput",row.control);row.control.value+=text;row.control.selectionStart=row.control.selectionEnd=row.control.value.length;document.fire("input",row.control);};
    return{a,data,storage,calls,mounts,first,workspace,request,recovery,browserFailures:()=>browserFailures,type,typeTarget(row:typeof mounts[number],text:string){document.fire("beforeinput",row.rootControl);row.rootControl.value=text;document.fire("input",row.rootControl);},sent,deliver(value:Record<string,unknown>){for(const listener of listeners)listener(value);},participant:()=>participant!,dispose(){for(const value of workspaces)value.dispose();},setFailNewPointer(value:boolean){failNewPointer=value;},setFailInput(value:boolean){failInput=value;},setFailCreate(value:boolean){failCreate=value;},setLoseReply(value:boolean){loseCreateReply=value;failLookup=value;},setFailMount(value:boolean){failMount=value;},holdCreate(){createWait=held<void>();return{...createWait,entered:createEntered.promise};},holdInput(){inputWait=held<void>();return inputWait;},holdFlush(){const wait=held<void>();flushWaits.push(wait);return wait;},setBrowserFailure(value:boolean){failBrowserWrites=value;},setIdentity(value:unknown){identityOverride=value;},setDevice(value:string){deviceId=value;},setFailPointer(value:boolean){failPointer=value;}};
  }
  let f:Awaited<ReturnType<typeof setup>>|undefined;
  try{f=await setup();await run(f);}finally{f?.dispose();thawNativeApi();for(const[index,key]of["document","HTMLInputElement","HTMLTextAreaElement"].entries()){if(prior[index])Object.defineProperty(globalThis,key,prior[index]!);else Reflect.deleteProperty(globalThis,key);}}
}

test("switch flushes and retires A, B close proof belongs to B, and A/B original drafts reopen",()=>fixture(async f=>{
  const old=f.mounts[0];f.type(old," retained author input");f.typeTarget(old,"D:/different-library/raw-target-intent");await f.first.startOtherDraft();
  const next=f.mounts.at(-1)!;const b=next.mounted.session.controller.state.draft.draftId;
  assert.notEqual(b,f.a);assert.equal(old.retired,true);assert.equal(old.inputs.inspect().producers.length,0);
  assert.equal(f.data.get(f.a)!.currentInput.body.utf16.map(value=>String.fromCharCode(value)).join(""),old.control.value);
  assert.deepEqual(f.data.get(f.a)!.currentInput.rootInput,captureText("D:/different-library/raw-target-intent"));
  assert.deepEqual(next.mounted.session.controller.state.input.rootInput,captureText(""));
  assert.deepEqual(next.mounted.session.controller.state.input.body,captureText(""));assert.equal(f.data.get(b)!.target,null);
  const proofs=await f.participant().prepare(close);assert.equal(proofs[0].kind,"capture");if(proofs[0].kind==="capture")assert.equal(proofs[0].draftId,b);f.participant().thaw();
  const before=JSON.stringify(f.data.get(b));f.type(old," late old input");await Promise.resolve();assert.equal(JSON.stringify(f.data.get(b)),before);
  await f.first.restore(f.a);assert.equal(f.mounts.at(-1)!.control.value,old.control.value.replace(" late old input",""));await f.first.restore(b);
  assert.equal(f.mounts.at(-1)!.mounted.session.controller.state.draft.draftId,b);
}));

test("native prepare closes the handoff gate synchronously, and handoff closes native prepare synchronously",()=>fixture(async f=>{
  const held=f.holdFlush();const preparing=f.participant().prepare(close);
  await assert.rejects(f.first.startOtherDraft());assert.equal(f.calls.some(call=>call.path.endsWith("/create")),false);
  held.resolve();await preparing;f.participant().thaw();
  const input=f.holdInput();f.type(f.mounts[0]," delayed");const switching=f.first.startOtherDraft();
  await assert.rejects(f.participant().prepare(close));input.resolve();await switching;
}));

test("pending original and active composition refuse handoff before any new create",()=>fixture(async f=>{
  const current=f.mounts[0].mounted.session.controller;
  current.beginComposition();await assert.rejects(f.first.startOtherDraft());current.endComposition();
  f.mounts[0].busy=true;await assert.rejects(f.first.startOtherDraft());f.mounts[0].busy=false;
  // This is the actual controller record used by state/prepare, not a workspace flag.
  (current as unknown as {draft:CaptureDraftRecord}).draft.pending={operationId:crypto.randomUUID(),requestKind:"capture",wire:"original",wireDigest:"c".repeat(64),inputRevision:1};
  await assert.rejects(f.first.startOtherDraft());assert.equal(f.calls.some(call=>call.path.endsWith("/create")),false);
}));

test("create never arrived: restart retries the same empty wire and IDs, retaining A",()=>fixture(async f=>{
  f.setFailCreate(true);await assert.rejects(f.first.startOtherDraft());assert.equal(f.mounts[0].retired,false);assert.equal(f.storage.get(captureDraftPointerKey),f.a);
  const failed=f.calls.find(call=>call.path.endsWith("/create"))!.body;
  f.first.dispose();f.setFailCreate(false);const reopened=f.workspace();await reopened.open();await reopened.startOtherDraft();
  const creates=f.calls.filter(call=>call.path.endsWith("/create"));assert.equal(creates.length,2);assert.deepEqual(creates[1].body,failed);assert.equal(f.mounts.at(-1)!.mounted.session.controller.state.draft.draftId,failed.draftId);
}));

test("lost create reply and failed offscreen mount reopen original B without replacement create",()=>fixture(async f=>{
  f.setLoseReply(true);await assert.rejects(f.first.startOtherDraft());const created=f.calls.find(call=>call.path.endsWith("/create"))!.body;
  assert.ok(f.data.has(created.draftId));assert.equal(f.mounts[0].retired,false);
  f.setLoseReply(false);f.setFailMount(true);await assert.rejects(f.first.startOtherDraft());assert.equal(f.storage.get(captureDraftPointerKey),f.a);
  f.first.dispose();f.setFailMount(false);const reopened=f.workspace();await reopened.open();await reopened.startOtherDraft();
  assert.equal(f.calls.filter(call=>call.path.endsWith("/create")).length,1);assert.equal(f.mounts.at(-1)!.mounted.session.controller.state.draft.draftId,created.draftId);
}));

test("pending new locator cannot cross device scope or replay a changed empty-input encoding",()=>fixture(async f=>{
  f.setFailCreate(true);await assert.rejects(f.first.startOtherDraft());const before=f.calls.filter(call=>call.path.endsWith("/create")).length;
  const pendingKey="weftext:capture:new-draft",pending=JSON.parse(f.storage.get(pendingKey)!);pending.wireDigest="0".repeat(64);f.storage.set(pendingKey,JSON.stringify(pending));
  f.setFailCreate(false);await assert.rejects(f.first.startOtherDraft());assert.equal(f.calls.filter(call=>call.path.endsWith("/create")).length,before);
  f.setDevice(crypto.randomUUID());await assert.rejects(f.first.startOtherDraft());assert.equal(f.calls.filter(call=>call.path.endsWith("/create")).length,before);
}));

test("active locator failure leaves the actual mounted B current, with its retained create IDs",()=>fixture(async f=>{
  f.setFailPointer(true);await assert.rejects(f.first.startOtherDraft());const b=f.mounts.at(-1)!;
  assert.equal(f.mounts[0].retired,true);assert.notEqual(b.mounted.session.controller.state.draft.draftId,f.a);assert.equal(f.storage.get(captureDraftPointerKey),f.a);
  assert.equal((f.first as unknown as {drafts:HTMLSelectElement}).drafts.value,b.mounted.session.controller.state.draft.draftId);
  const proof=await f.participant().prepare(close);if(proof[0].kind==="capture")assert.equal(proof[0].draftId,b.mounted.session.controller.state.draft.draftId);f.participant().thaw();
  assert.equal(JSON.parse(f.storage.get("weftext:capture:new-draft")!).draftId,b.mounted.session.controller.state.draft.draftId);
}));


test("native original delivery excludes handoff and a retired A delivery callback cannot target B",()=>fixture(async f=>{
  const old=f.mounts[0],original={draftId:f.a,operationId:crypto.randomUUID()};
  const delivery=old.deliver(original);assert.equal(f.sent.at(-1)!.type,"capture-open");
  await assert.rejects(f.first.startOtherDraft());
  f.deliver({type:"capture-open-result",requestId:f.sent.at(-1)!.requestId,accepted:true});await delivery;
  await f.first.startOtherDraft();const count=f.sent.length;
  await assert.rejects(old.deliver(original));assert.equal(f.sent.length,count);
}));


test("cancelling a refused native close cannot thaw A's separate handoff freeze",()=>fixture(async f=>{
  const hold=f.holdCreate();const switching=f.first.startOtherDraft();
  await hold.entered;
  assert.equal(f.mounts[0].control.readOnly,true);
  await assert.rejects(f.participant().prepare(close));f.participant().thaw();
  assert.equal(f.mounts[0].control.readOnly,true);
  assert.throws(()=>f.mounts[0].mounted.session.controller.edit(emptyCaptureInput()));
  hold.resolve();await switching;assert.equal(f.mounts[0].retired,true);
}));


test("unconfirmed input and unavailable durable new IDs cannot retire A or dispatch create",()=>fixture(async f=>{
  f.setFailNewPointer(true);await assert.rejects(f.first.startOtherDraft());assert.equal(f.calls.some(call=>call.path.endsWith("/create")),false);assert.equal(f.mounts[0].retired,false);
  f.setFailNewPointer(false);f.setFailInput(true);f.type(f.mounts[0]," unconfirmed author cut");
  await assert.rejects(f.first.startOtherDraft());assert.equal(f.mounts[0].retired,false);assert.equal(f.mounts[0].mounted.session.controller.state.blocked,true);
  assert.equal(f.calls.some(call=>call.path.endsWith("/create")),false);assert.ok(f.mounts[0].control.value.endsWith(" unconfirmed author cut"));
}));

test("after an encoding change, an existing original blank draft can still be read without recreating",()=>fixture(async f=>{
  f.setLoseReply(true);await assert.rejects(f.first.startOtherDraft());const key="weftext:capture:new-draft",pending=JSON.parse(f.storage.get(key)!);
  pending.wireDigest="0".repeat(64);f.storage.set(key,JSON.stringify(pending));f.setLoseReply(false);
  await f.first.startOtherDraft();assert.equal(f.calls.filter(call=>call.path.endsWith("/create")).length,1);
  assert.equal(f.mounts.at(-1)!.mounted.session.controller.state.draft.draftId,pending.draftId);
}));


test("C1 cancelled then C2 preparing: late C1 rejection cannot clear C2's close attempt",()=>fixture(async f=>{
  const first=f.holdFlush(),second=f.holdFlush();
  const c1=f.participant().prepare(close);f.participant().thaw();
  const c2Request={...close,closeAttemptId:"C2"},c2=f.participant().prepare(c2Request);
  first.resolve();await assert.rejects(c1);second.resolve();await c2;
  assert.equal(f.participant().freeze(c2Request),true);f.participant().thaw();
}));

test("identity and pending locator DTOs reject coercible arrays, truthy flags and extra fields",()=>fixture(async f=>{
  const id=crypto.randomUUID();
  for(const value of [{id,isCurrent:"true"},{id,isCurrent:1},{id:[id],isCurrent:true},{id,isCurrent:true,extra:"unqualified"}]){
    f.setIdentity(value);await assert.rejects(f.first.startOtherDraft());
  }
  f.setIdentity(undefined);f.setFailCreate(true);await assert.rejects(f.first.startOtherDraft());
  const key="weftext:capture:new-draft",original=JSON.parse(f.storage.get(key)!),before=f.calls.filter(call=>call.path.endsWith("/create")).length;
  for(const patch of [{draftId:[original.draftId]},{mutationId:[original.mutationId]},{wireDigest:[original.wireDigest]},{deviceId:[original.deviceId]},{extra:"unqualified"}]){
    f.storage.set(key,JSON.stringify({...original,...patch}));await assert.rejects(f.first.startOtherDraft());
  }
  assert.equal(f.calls.filter(call=>call.path.endsWith("/create")).length,before);assert.equal(f.mounts[0].retired,false);
}));

test("Core-confirmed full input survives browser staging/ACK storage failure without a permanent handoff veto",()=>fixture(async f=>{
  f.setBrowserFailure(true);const old=f.mounts[0];f.type(old," durable in Core despite browser IO");
  await f.first.startOtherDraft();assert.equal(old.retired,true);assert.ok(f.browserFailures()>0);
  assert.deepEqual(f.data.get(f.a)!.currentInput.body,captureText(old.control.value));
  assert.ok(f.calls.some(call=>call.path==="/api/capture/draft/flush"&&call.body.draftId===f.a));
}));

test("another producer's unconfirmed outbox remains a real barrier even when current Core input is valid",()=>fixture(async f=>{
  const device=(await f.request<{id:string}>("/api/capture/identity",{})).id,audience=JSON.stringify(["local-device",device]);
  await f.recovery.stageCaptureCurrent(audience,{kind:"captureTransport",id:`capture:${f.a}:other`,owner:{kind:"capture",draftId:f.a},producerId:crypto.randomUUID(),producerRevision:1,baseRevision:1,input:{texts:{name:[],title:[],body:[],rootInput:[],parentInput:[],parametersText:[]},bodyMode:null,rawInputs:[],detachedRawInputs:[]}});
  await assert.rejects(f.first.startOtherDraft());assert.equal(f.mounts[0].retired,false);assert.equal(f.calls.some(call=>call.path.endsWith("/create")),false);
}));

test("controlled Core create fixture follows original ID/mutation/input replay constraints",()=>fixture(async f=>{
  await f.first.startOtherDraft();const command=clone(f.calls.find(call=>call.path.endsWith("/create"))!.body);
  const replay=await f.request<CaptureDraftRecord>("/api/capture/draft/create",command);assert.equal(replay.draftId,command.draftId);
  const before=JSON.stringify(f.data.get(command.draftId));
  await assert.rejects(f.request("/api/capture/draft/create",{...command,input:{...command.input,bodyMode:"markup"}}),/different input/);
  await assert.rejects(f.request("/api/capture/draft/create",{...command,mutationId:crypto.randomUUID()}),/already exists/);
  assert.equal(JSON.stringify(f.data.get(command.draftId)),before);
  const next=f.mounts.at(-1)!;f.type(next," later author mutation");await next.mounted.session.controller.drain();
  await assert.rejects(f.request("/api/capture/draft/create",command),/already exists/);
}));
