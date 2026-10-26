import { CaptureForm, type CaptureAdvancedControls } from "./capture-form";
import { CaptureFields, type CaptureSchemaView } from "./capture-fields";
import { CaptureCloseGuard } from "./capture-close";
import { LocalCaptureSession, type LocalCaptureOptions, type CaptureResultIdentity } from "./capture-local";
import { captureString, captureText, observeCaptureText, type CaptureInput, type CaptureTransport } from "./capture-draft-state";
import { PersonalControlInput } from "./personal-control-input";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { AuthorInputFactory } from "./author-input";
import { action, ui, type DataSchema } from "./native-value-form";
import { disposeUiTree, errorUiText, message, replaceUiChildren, setUiAttr, setUiText, UiError } from "./ui-locale";

export interface CaptureMountPorts {
  request:CaptureTransport;
  options?:LocalCaptureOptions;
  /** Resolves only after the actual Library adopted this committed original. */
  deliverCommitted?(original:CaptureResultIdentity):Promise<void>;
  requestClose?():Promise<void>;
}
export interface MountedLocalCapture {
  readonly session:LocalCaptureSession;
  readonly form:CaptureForm<import("./capture-draft-state").CaptureDraftRecord>;
  readonly guard:CaptureCloseGuard<import("./capture-draft-state").CaptureDraftRecord>;
  assertCloseReady():void;
  dispose():void;
}
/** Presentation/transport composition. Native lifecycle is injected only after
 * the real host installs this participant; mounting never announces ready. */
export async function mountLocalCapture(host:HTMLElement,ports:CaptureMountPorts):Promise<MountedLocalCapture> {
  let live=true,form:MountedLocalCapture["form"]|undefined,fields:CaptureFields<import("./capture-draft-state").CaptureDraftRecord>|undefined;
  let closeFrozen=false,pendingActions=0;
  const activity=async<T>(run:()=>Promise<T>):Promise<T>=>{
    if(!live||closeFrozen)throw new UiError(message("desktop.unsavedClose"));
    pendingActions++;
    try{return await run();}finally{pendingActions--;}
  };
  const assertCloseReady=()=>{if(!live||pendingActions||form?.pendingAction)throw new UiError(message("desktop.unsavedClose"));};
  const recovery=ports.options?.recovery??new PersonalInputRecovery();
  const session=await LocalCaptureSession.open(ports.request,{...ports.options,recovery,changed:state=>{if(live){form?.update(state);ports.options?.changed?.(state);}}});
  const draftId=session.controller.state.draft.draftId;
  const inputListeners=new AbortController();
  const producerDocument={addEventListener(type:string,listener:EventListenerOrEventListenerObject,options?:boolean|AddEventListenerOptions){
    host.ownerDocument.addEventListener(type,listener,{...(typeof options==="object"?options:{capture:options}),signal:inputListeners.signal});
  }} as unknown as Document;
  const inputs=new PersonalControlInput(producerDocument,()=>live,()=>{throw new UiError(message("capture.rawRetained"));});
  try {
  const audience=JSON.stringify(["capture-surface",draftId]);
  const authorInputs:AuthorInputFactory=(owner,current,sink,revision)=>inputs.session(audience,1,owner,()=>live&&current(),sink,revision);
  const target=ui("section","capture-target"),targetStatus=ui("p","data-help"),root=ui("input","data-input"),parent=ui("input","data-input");
  const state=session.controller.state;
  root.value=captureString(state.input.rootInput);parent.value=captureString(state.input.parentInput);
  setUiAttr(root,"aria-label",message("capture.target"));setUiAttr(parent,"aria-label",message("capture.parent"));
  root.dataset.controlKey=JSON.stringify([draftId,"rootInput"]);parent.dataset.controlKey=JSON.stringify([draftId,"parentInput"]);
  root.placeholder=state.draft.target?.rootHint??"";
  const targetSession=authorInputs({kind:"capture",draftId},()=>live,snapshot=>{session.controller.observeAuthorCut(snapshot);const next=session.controller.state.input;observeCaptureText(next,snapshot.context.slot as "rootInput"|"parentInput",snapshot);session.controller.edit(next);},()=>session.controller.editRevision);
  if(!targetSession)throw new UiError(message("capture.rawRetained"));
  targetSession.bind(root,"rootInput");targetSession.bind(parent,"parentInput");
  const collectTarget=()=>{const next=session.controller.state.input;let changed=false;for(const [slot,control]of [["rootInput",root],["parentInput",parent]]as const){if(captureString(next[slot])!==control.value){next[slot]=captureText(control.value);changed=true;}}if(changed)session.controller.edit(next);};
  target.append(ui("h2","",message("capture.target")),root,ui("p","",message("capture.parent")),parent,targetStatus);
  let schemaVersion=0;
  async function loadFields(container:HTMLElement):Promise<void> {return activity(async()=>{
    const version=++schemaVersion;fields?.collectOwnedRaw();fields?.dispose();fields=undefined;replaceUiChildren(container);
    const target=session.controller.state.draft.target;if(!target){container.append(ui("p","data-help",message("capture.targetRequired")));return;}
    try {
      const result=await ports.request<{schema:DataSchema;roleRef:string|null;semanticDigest:string}>("/api/data/schema",{root:target.rootHint});
      if(!live||version!==schemaVersion)return;
      if(!result.roleRef){container.append(ui("p","data-help",message("schemaSettings.noExecutablePlan")));return;}
      fields=new CaptureFields(container,session.controller,authorInputs,{schema:result.schema,schemaRef:result.roleRef,semanticDigest:result.semanticDigest} satisfies CaptureSchemaView);
    }catch(error){if(live&&version===schemaVersion)container.append(ui("p","data-warning",errorUiText(error)));}
  });}
  let advancedHost:HTMLElement|undefined;
  target.append(action(message("capture.selectTarget"),async()=>{try{await activity(async()=>{collectTarget();await session.controller.drain();const current=session.controller.state.draft;const nextRoot=root.value||current.target?.rootHint;if(!nextRoot)throw new UiError(message("capture.targetRequired"));await session.selectTarget({root:nextRoot,parent:parent.value||nextRoot});root.placeholder=nextRoot;setUiText(targetStatus,nextRoot);if(advancedHost)await loadFields(advancedHost);});}catch(error){form?.showError(error);}}));
  const assertSubmittable=()=>{session.assertRecovered();fields?.assertSubmittable();const input=session.controller.state.input;if(!fields&&(input.fields.length||input.typeIds.length))throw new UiError(message("capture.rawRetained"));};
  let guard!:MountedLocalCapture["guard"];
  const requestClose=async()=>{
    if(ports.requestClose){await ports.requestClose();return;}
    const proof=await guard.prepare();if(!guard.freeze(proof))throw new UiError(message("desktop.unsavedClose"));
    host.ownerDocument.defaultView?.close();
    // A user-opened browser tab can refuse window.close. Keep it editable.
    if(!host.ownerDocument.defaultView?.closed)guard.thaw();
  };
  form=new CaptureForm({controller:session.controller,authorInputs,target,
    labels:{name:message("capture.name"),authorTitle:message("capture.authorTitle"),bodyMode:message("capture.bodyMode"),plain:message("capture.plain"),markup:message("capture.markup"),advanced:message("capture.advanced"),keepClose:message("capture.keepClose"),confirmOriginal:message("capture.confirmOriginal"),draftSaved:message("capture.draftSaved")},
    advanced:container=>{advancedHost=container;const controls:CaptureAdvancedControls={collectOwnedRaw:()=>{collectTarget();fields?.collectOwnedRaw();},update:(input,consumed)=>{if(consumed){root.value=captureString(input.rootInput);parent.value=captureString(input.parentInput);}fields?.update(input,consumed);},retireAuthorization:()=>{targetSession.retire();root.readOnly=parent.readOnly=true;fields?.retireAuthorization();},dispose:()=>{schemaVersion++;targetSession.retire();fields?.dispose();}};return controls;},
    canOpen:()=>!!ports.deliverCommitted,
    submit:async mode=>{if(!session.controller.state.draft.pending)assertSubmittable();await session.save(mode,async original=>{if(!ports.deliverCommitted)throw new UiError(message("capture.pending"));await ports.deliverCommitted(original);},requestClose);},
    confirmOriginal:async()=>{const fact=await session.confirmOriginal();if(fact)setUiText(targetStatus,()=>`${fact.operationId} · ${fact.status}`);},requestClose});
  guard=new CaptureCloseGuard(session.controller,inputs,{assertResolved:()=>session.assertRecovered(),collectOwnedRaw:()=>form!.collectOwnedRaw(),freeze:()=>{if(pendingActions)throw new UiError(message("desktop.unsavedClose"));closeFrozen=true;form!.freeze();},thaw:()=>{closeFrozen=false;form!.thaw();}});
  replaceUiChildren(host,form.element);
  await loadFields(advancedHost!);
  const retained=ui("section","capture-recovery");
  function renderOwnRecovery():void {
    replaceUiChildren(retained);
    const input=session.controller.state.input;
    if(session.controller.state.draft.inputCut==="authorOnly")for(const [slot,proofs]of Object.entries(input.authorTexts))for(const proof of proofs){const copy=ui("textarea","data-input");copy.readOnly=true;copy.value=captureString(proof.text);retained.append(ui("p","",slot),copy);}
    for(const proposal of session.unconfirmed){const section=ui("section");section.append(ui("p","",message("capture.rawRetained")));if(proposal.pending)section.append(ui("code","",proposal.pending.command.mutationId));for(const proofs of Object.values(proposal.input.texts))for(const proof of proofs){const copy=ui("textarea","data-input");copy.readOnly=true;copy.value=proof.text;section.append(copy);}section.append(action(message("common.cancel"),async()=>{try{await activity(async()=>{await session.discardUnconfirmed(proposal);renderOwnRecovery();});}catch(error){form?.showError(error);}}));retained.append(section);}
  }
  renderOwnRecovery();form.element.append(retained);
  const unloading=(event:BeforeUnloadEvent)=>{const current=session.controller.state;if(current.persisting||current.blocked||inputs.inspect().producers.some(producer=>producer.nativePending||producer.pendingDeliveries||producer.unapplied))event.preventDefault();};
  host.ownerDocument.defaultView?.addEventListener("beforeunload",unloading);
  return {session,form,guard,assertCloseReady,dispose(){if(!live)return;live=false;schemaVersion++;session.controller.retire();inputListeners.abort();form?.dispose();inputs.clear();fields?.dispose();host.ownerDocument.defaultView?.removeEventListener("beforeunload",unloading);disposeUiTree(host);}};
  } catch(error) {
    live=false;session.controller.retire();inputListeners.abort();form?.dispose();inputs.clear();fields?.dispose();disposeUiTree(host);
    throw error;
  }
}
