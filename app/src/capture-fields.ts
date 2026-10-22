import type { AuthorInputFactory, AuthorInputSession, AuthorInputSnapshot } from "./author-input";
import { CaptureController, type CaptureMutableRecord } from "./capture-controller";
import type { CaptureAdvancedControls } from "./capture-form";
import { captureString, captureText, restoreCaptureField, type CaptureInput, type CaptureRawInput } from "./capture-draft-state";
import { action, emptyNativeValue, fieldNames, fieldsForTypes, nativeControlAddress, nativeFormControls, typeNames, ui, valueEditor, type DataSchema, type FormReferences, type NativeValue } from "./native-value-form";
import { reminderForm, type ReminderForm } from "./reminder-form";
import { newReminder } from "./reminder-values";
import { disposeUiTree, errorUiText, message, replaceUiChildren, setUiValidity, t, UiError, type UiText } from "./ui-locale";

export interface CaptureSchemaView { schema:DataSchema; schemaRef:string; semanticDigest:string; allowedTypes?:readonly string[]; allowedFields?:readonly string[]; references?:FormReferences }
interface FieldView { element:HTMLElement; session:AuthorInputSession; reminder?:ReminderForm }
/** One consumer of the shared schema/reminder forms. The actual Capture owner
 * supplies every raw sink; a current document is never a fallback identity. */
export class CaptureFields<R extends CaptureMutableRecord> implements CaptureAdvancedControls {
  private views=new Map<string,FieldView>();
  private types=ui("select","data-input");
  private fields=ui("section","capture-fields");
  private recovery=ui("section","capture-field-recovery");
  private active=true;
  private typeSession:AuthorInputSession;
  private typeChoice?:AuthorInputSnapshot;
  private fieldChoices=new Map<AuthorInputSession,AuthorInputSnapshot>();
  private rendered="";
  private unrestored=new Set<string>();
  private rawKey(raw:Pick<CaptureRawInput,"fieldId"|"valuePath"|"controlRole">):string{return JSON.stringify([raw.fieldId,raw.valuePath,raw.controlRole]);}
  constructor(private host:HTMLElement,private controller:CaptureController<R>,private authorInputs:AuthorInputFactory,private view:CaptureSchemaView) {
    if(!view.schemaRef || !/^[a-f0-9]{64}$/.test(view.semanticDigest))throw new UiError(message("capture.rawRetained"));
    const session=authorInputs({kind:"capture",draftId:controller.state.draft.draftId},()=>this.active,snapshot=>{this.typeChoice=snapshot;},()=>controller.editRevision);
    if(!session)throw new UiError(message("capture.rawRetained"));this.typeSession=session;
    this.types.multiple=true;this.types.dataset.controlKey=JSON.stringify(["capture",controller.state.draft.draftId,"types"]);
    const allowed=view.allowedTypes??view.schema.types.map(type=>type.id);
    for(const type of view.schema.types.filter(type=>allowed.includes(type.id))){const alias=Object.entries(view.schema.aliases).find(([,id])=>id===type.id)?.[0]??type.id;const option=ui("option","",typeNames[alias]??alias);option.value=type.id;this.types.append(option);}
    session.bindChoice(this.types,"types");
    this.types.onchange=()=>{if(!this.active)return;try{this.collectOwnedRaw();const next=this.controller.state.input;this.assertBasisChange(next);next.typeIds=[...this.types.selectedOptions].map(option=>option.value);next.formBasis={schemaRef:view.schemaRef,semanticDigest:view.semanticDigest};this.controller.edit(next);if(this.typeChoice){this.controller.observeAuthorCut(this.typeChoice);this.typeChoice=undefined;}this.render(true);}catch(error){this.recovery.append(ui("p","data-warning",errorUiText(error)));}};
    host.append(ui("h3","",message("queryPlan.noteTypes")),this.types,this.fields,this.recovery);this.render(true);
  }
  private assertBasisChange(input:CaptureInput):void {if(input.fields.length && !this.currentBasis(input))throw new UiError(message("capture.rawRetained"));}
  private currentBasis(input:CaptureInput):boolean {return !!input.formBasis && input.formBasis.schemaRef===this.view.schemaRef && input.formBasis.semanticDigest===this.view.semanticDigest;}
  private fieldSession(fieldId:string):AuthorInputSession {
    let session!:AuthorInputSession;
    const addresses=new Map<string,{role:string;path:readonly import("./author-input").ValuePath[]}>();
    const sink=(snapshot:AuthorInputSnapshot)=>{
      if(snapshot.kind==="choice"){this.fieldChoices.set(session,snapshot);return;}
      const address=addresses.get(JSON.stringify([snapshot.context.slot,snapshot.context.path]));
      if(!address)throw new UiError(message("capture.rawRetained"));
      this.unrestored.delete(this.rawKey({fieldId,valuePath:[...address.path],controlRole:address.role}));
      this.controller.authorSnapshot(snapshot,fieldId,address.role);
    };
    const original=this.authorInputs({kind:"capture",draftId:this.controller.state.draft.draftId},()=>this.active && this.views.get(fieldId)?.session===session,sink,()=>this.controller.editRevision);
    if(!original)throw new UiError(message("capture.rawRetained"));
    const wrapped=this.controller.wrapSession(original,fieldId);
    session={...wrapped,bind:(control,slot,path=[])=>{
      const address=nativeControlAddress(control);
      if(address.identity.fieldId!==fieldId || JSON.stringify(address.identity.path)!==JSON.stringify(path))throw new UiError(message("capture.rawRetained"));
      addresses.set(JSON.stringify([slot,path]),{role:address.role,path:address.identity.path});
      return wrapped.bind(control,slot,path);
    }};
    return session;
  }
  private retireViews():void {for(const view of this.views.values()){view.session.retire();this.fieldChoices.delete(view.session);view.reminder?.dispose();disposeUiTree(view.element);}this.views.clear();replaceUiChildren(this.fields);}
  private render(force=false,collect=true):void {
    if(!this.active)return;
    if(collect)this.collectOwnedRaw();
    const input=this.controller.state.input;
    const key=JSON.stringify([input.typeIds,input.fields.map(field=>[field.fieldId,field.valueState]),input.formBasis]);
    if(!force && key===this.rendered)return;
    this.retireViews();this.unrestored.clear();this.rendered=key;
    this.types.disabled=input.fields.length>0&&!this.currentBasis(input);
    for(const option of this.types.options)option.selected=input.typeIds.includes(option.value);
    const selected=fieldsForTypes(this.view.schema,input.typeIds);
    for(const {field,required} of selected) {
      if(this.view.allowedFields && !this.view.allowedFields.includes(field.id))continue;
      const row=ui("section","capture-field");const label:UiText=fieldNames[field.key]??field.key;row.append(ui("h4","",label));this.fields.append(row);
      const stored=input.fields.find(value=>value.fieldId===field.id);
      if(!stored) {
        const add=action(message("quickCapture.fillField"),()=>{
          this.collectOwnedRaw();const next=this.controller.state.input;this.assertBasisChange(next);
          if(next.fields.some(value=>value.fieldId===field.id))return;
          // A new reminder intent receives its UUID once, when explicitly added.
          next.fields.push({fieldId:field.id,valueState:{kind:"valid",value:field.id==="tasks/reminder"?newReminder("absolute"):emptyNativeValue(field.shape,this.view.schema)}});
          next.formBasis={schemaRef:this.view.schemaRef,semanticDigest:this.view.semanticDigest};this.controller.edit(next);this.render(true);
        });add.disabled=input.fields.length>0&&!this.currentBasis(input);row.append(add);
        if(required)row.dataset.required="true";
        continue;
      }
      if(!this.currentBasis(input)){
        const copy=ui("textarea","data-input");copy.readOnly=true;copy.value=JSON.stringify(stored.valueState);
        row.append(ui("p","data-warning",message("capture.rawRetained")),copy,action(message("nativeEditor.removeMember"),()=>{this.collectOwnedRaw();this.controller.retireRaw(field.id,[],"schema");const next=this.controller.state.input;next.fields=next.fields.filter(value=>value.fieldId!==field.id);this.controller.edit(next);this.render(true);}));continue;
      }
      let initial:NativeValue;
      try{initial=restoreCaptureField(stored,input,{fieldId:field.id,schemaRef:this.view.schemaRef,semanticDigest:this.view.semanticDigest});}
      catch(error){row.append(ui("p","data-warning",errorUiText(error)));continue;}
      const session=this.fieldSession(field.id);
      // Publish the real proposal owner before shared controls bind. Binding
      // checks current() immediately, before their DOM is appended.
      this.views.set(field.id,{element:row,session});
      const changed=(value:NativeValue)=>{if(!this.active || this.views.get(field.id)?.session!==session)return;const next=this.controller.state.input;const entry=next.fields.find(item=>item.fieldId===field.id);if(!entry)throw new UiError(message("capture.rawRetained"));entry.valueState={kind:"valid",value};this.controller.edit(next);const choice=this.fieldChoices.get(session);if(choice){this.controller.observeAuthorCut(choice);this.fieldChoices.delete(session);}};
      const identity={fieldId:field.id,path:[]};const slot=JSON.stringify(["field",field.id]);
      const reminder=field.id==="tasks/reminder"?reminderForm(initial,this.view.schema,changed,identity,{session,slot}):undefined;
      const element=reminder?.element??valueEditor(field.shape,initial,this.view.schema,changed,label,identity,0,this.view.references,{session,slot});
      this.views.set(field.id,{element,session,reminder});row.append(element);
      row.append(action(message("nativeEditor.removeMember"),()=>{this.collectOwnedRaw();this.controller.retireRaw(field.id,[],"structure");const next=this.controller.state.input;next.fields=next.fields.filter(value=>value.fieldId!==field.id);this.controller.edit(next);this.render(true);}));
      this.restoreRaw(field.id,input.rawInputs.filter(raw=>raw.fieldId===field.id));
    }
    const renderedIds=new Set(selected.map(item=>item.field.id));
    for(const stored of input.fields.filter(field=>!renderedIds.has(field.fieldId)||this.view.allowedFields&&!this.view.allowedFields.includes(field.fieldId))){
      const row=ui("section","capture-field"),copy=ui("textarea","data-input");copy.readOnly=true;copy.value=JSON.stringify(stored.valueState);
      row.append(ui("p","",stored.fieldId),ui("p","data-warning",message("capture.rawRetained")),copy,action(message("nativeEditor.removeMember"),()=>{this.controller.retireRaw(stored.fieldId,[],"schema");const next=this.controller.state.input;next.fields=next.fields.filter(field=>field.fieldId!==stored.fieldId);this.controller.edit(next);this.render(true);}));this.fields.append(row);
    }
    this.renderRecovery();
  }
  private restoreRaw(fieldId:string,raw:readonly CaptureRawInput[]):void {
    const view=this.views.get(fieldId);if(!view)return;
    const exact=raw.filter(input=>input.binding==="exact");
    if(view.reminder){
      const before=new Map(nativeFormControls(view.element).map(([,control])=>{const address=nativeControlAddress(control);return[this.rawKey({fieldId,valuePath:[...address.identity.path],controlRole:address.role}),{control,text:control.value}];}));
      const restored=view.reminder.restoreRaw(exact.map(input=>({path:input.valuePath,role:input.controlRole,text:captureString(input.text)})));
      for(const rejected of restored.rejected)this.unrestored.add(this.rawKey({fieldId,valuePath:[...rejected.input.path],controlRole:rejected.input.role}));
      for(const applied of restored.applied){const key=this.rawKey({fieldId,valuePath:[...applied.path],controlRole:applied.role}),prior=before.get(key);if(!prior || prior.text!==applied.text){this.unrestored.add(key);if(prior)prior.control.setCustomValidity(t(message("reminder.invalidInput")));}}
      return;
    }
    const controls=nativeFormControls(view.element);
    for(const input of exact) {
      const matches=controls.filter(([,control])=>{const address=nativeControlAddress(control);return address.identity.fieldId===fieldId && address.role===input.controlRole && JSON.stringify(address.identity.path)===JSON.stringify(input.valuePath);});
      if(matches.length!==1){this.unrestored.add(this.rawKey(input));continue;}
      const control=matches[0][1];if(!(control instanceof HTMLInputElement||control instanceof HTMLTextAreaElement)||control.type==="checkbox"){this.unrestored.add(this.rawKey(input));continue;}
      const old=control.value,text=captureString(input.text);control.value=text;
      if(control.value!==text){control.value=old;this.unrestored.add(this.rawKey(input));continue;}
      // Restoration is not an author event. A different lexical raw is retained
      // as invalid instead of letting an older valid candidate be submitted.
      if(old!==text)setUiValidity(control,message("reminder.invalidInput"));
    }
  }
  private renderRecovery():void {
    replaceUiChildren(this.recovery);
    const input=this.controller.state.input,retained=[...input.detachedRawInputs.map(entry=>entry.input),...input.rawInputs.filter(raw=>raw.binding==="copyOnly" || !this.views.has(raw.fieldId) || this.unrestored.has(this.rawKey(raw)))];
    if(retained.length)this.recovery.append(ui("p","data-help",message("capture.rawRetained")));
    for(const raw of retained) {
      const text=ui("textarea","data-input");text.readOnly=true;text.value=captureString(raw.text);this.recovery.append(ui("p","",raw.fieldId),text);
    }
  }
  collectOwnedRaw():void {
    if(!this.active)return;
    const next=this.controller.state.input;let changed=false;
    for(const [fieldId,view]of this.views)for(const [,control]of nativeFormControls(view.element)) {
      if(!(control instanceof HTMLInputElement||control instanceof HTMLTextAreaElement)||control.type==="checkbox")continue;
      const address=nativeControlAddress(control);
      if(this.unrestored.has(this.rawKey({fieldId,valuePath:[...address.identity.path],controlRole:address.role})))continue;
      const index=next.rawInputs.findIndex(raw=>raw.fieldId===fieldId && raw.controlRole===address.role && JSON.stringify(raw.valuePath)===JSON.stringify(address.identity.path));
      const value=captureText(control.value);
      if(index>=0){if(JSON.stringify(next.rawInputs[index].text)!==JSON.stringify(value)){next.rawInputs[index].text=value;changed=true;}}
      else {next.rawInputs.push({fieldId,valuePath:structuredClone([...address.identity.path]),controlRole:address.role,text:value,binding:"exact",author:null});changed=true;}
    }
    if(changed)this.controller.edit(next);
  }
  assertSubmittable():void {
    if(!this.active)throw new UiError(message("capture.rawRetained"));
    const input=this.controller.state.input;
    if(this.unrestored.size)throw new UiError(message("capture.rawRetained"));
    if((input.fields.length||input.typeIds.length)&&!this.currentBasis(input))throw new UiError(message("capture.rawRetained"));
    for(const view of this.views.values())for(const [,control]of nativeFormControls(view.element))if(!control.reportValidity())throw new UiError(message("reminder.invalidInput"));
    if(input.fields.some(field=>!this.views.has(field.fieldId)))throw new UiError(message("capture.rawRetained"));
  }
  update(input:CaptureInput,consumedCut=false):void {if(consumedCut)this.render(true,false);this.renderRecovery();}
  retireAuthorization():void {this.collectOwnedRaw();this.active=false;this.typeSession.retire();this.types.disabled=true;for(const view of this.views.values()){view.session.retire();for(const [,control]of nativeFormControls(view.element))control.disabled=true;}}
  dispose():void {this.active=false;this.typeSession.retire();this.retireViews();disposeUiTree(this.host);}
}
