/** Actual local mount/forms/fields/producer retirement in owned headless Chrome.
 * Controlled in-memory transport; no native host, Core or OS acceptance. */
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),".."),out=process.env.WEFTEXT_BROWSER_REPORT;
if(!out||!path.isAbsolute(out))throw new Error("WEFTEXT_BROWSER_REPORT must be a new absolute directory");
await mkdir(out);
const {chromium}=createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE||"playwright");
const {createServer}=await import(pathToFileURL(path.join(repo,"app/node_modules/vite/dist/node/index.js")).href);
const result={scope:"Actual mountLocalCapture, CaptureForm/Fields, PersonalControlInput and LocalCaptureSession in Chrome; controlled transport, no Core/native/IME/AT acceptance",checks:[],errors:[],source:{},ownership:{directory:out,profile:path.join(out,"profile"),browserClosed:false,viteClosed:false}};
for(const file of ["scripts/test_capture_mount_browser.mjs","app/src/capture-mount.ts","app/src/capture-form.ts","app/src/capture-fields.ts","app/src/capture-controller.ts","app/src/capture-local.ts","app/src/capture-local-persistence.ts","app/src/capture-input-persistence.ts","app/src/capture-author-outbox.ts","app/src/personal-control-input.ts","app/src/personal-input-recovery.ts"])result.source[file]=createHash("sha256").update(await readFile(path.join(repo,file))).digest("hex");
let vite,browser;
try{
  vite=await createServer({root:path.join(repo,"app"),configFile:false,server:{host:"127.0.0.1",port:0,hmr:false}});await vite.listen();
  browser=await chromium.launchPersistentContext(result.ownership.profile,{channel:"chrome",headless:true});
  const page=await browser.newPage();page.on("pageerror",error=>result.errors.push(error.message));
  await page.route("**/capture-mount-fixture",route=>route.fulfill({contentType:"text/html",body:'<!doctype html><meta charset="utf-8"><main id="capture"></main>'}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/capture-mount-fixture`);result.browser=await page.evaluate(()=>navigator.userAgent);
  const keys=await page.evaluate(async()=>{
    const [{mountLocalCapture},state,{PersonalInputRecovery},{captureAuthorInput},{nativeControlAddress},{t,message}]=await Promise.all([import("/src/capture-mount.ts"),import("/src/capture-draft-state.ts"),import("/src/personal-input-recovery.ts"),import("/src/author-input.ts"),import("/src/native-control-identity.ts"),import("/src/ui-locale.ts")]);
    const device=crypto.randomUUID(),a=crypto.randomUUID(),b=crypto.randomUUID(),basis={schemaRef:"weftext-config://fixture/schema",semanticDigest:"a".repeat(64)};
    const schema={definitions:{},aliases:{note:"fixture/note"},types:[{id:"fixture/note",fields:[{id:"fixture/amount",required:false}],requires:[],conflicts:[]}],fields:[{id:"fixture/amount",key:"amount",nullable:false,shape:{kind:"scalar",semantic:"int"}}]};
    const make=(id,root,body)=>{const input=state.emptyCaptureInput();input.body=state.captureText(body);input.typeIds=["fixture/note"];input.formBasis=basis;input.fields=[{fieldId:"fixture/amount",valueState:{kind:"valid",value:{kind:"int",value:"1"}}}];return{version:2,draftId:id,revision:1,inputRevision:1,inputCut:"full",lastMutation:{kind:"create",mutationId:crypto.randomUUID(),fingerprint:"b".repeat(64),resultingRevision:1},currentInput:input,target:{rootHint:root,parent:{kind:"libraryRoot"},hostedSlot:null},pending:null};};
    const records=new Map([[a,make(a,"fixture-A","A body")],[b,make(b,"fixture-B","B body")]]),calls=[];
    let holdSchema=false,resolveSchema,schemaReturned;
    const request=async(route,body)=>{
      calls.push({route,body:structuredClone(body)});
      if(route==="/api/capture/identity")return{id:device,isCurrent:true};
      if(route==="/api/data/schema"){
        if(holdSchema&&body.root==="fixture-A"){
          window.captureMountSchemaEntered=true;
          return new Promise(resolve=>{resolveSchema=resolve;}).then(value=>{schemaReturned=true;return value;});
        }
        return{schema,roleRef:basis.schemaRef,semanticDigest:basis.semanticDigest};
      }
      const record=records.get(body.draftId);if(!record)throw new Error("unexpected draft");
      if(route==="/api/capture/draft/read")return structuredClone(record);
      if(route==="/api/capture/draft/input"||route==="/api/capture/draft/target"){
        if(body.expectedRevision!==record.revision)throw new Error("actual controller CAS mismatch");
        const kind=route.endsWith("/input")?"input":"target";
        const next={...record,revision:record.revision+1,inputRevision:record.inputRevision+(kind==="input"?1:0),currentInput:kind==="input"?structuredClone(body.input):record.currentInput,target:kind==="target"?{rootHint:body.target.root,parent:{kind:"libraryRoot"},hostedSlot:null}:record.target,lastMutation:{kind,mutationId:body.mutationId,fingerprint:kind==="input"?await state.captureInputFingerprint(body.expectedRevision,body.input):"c".repeat(64),resultingRevision:record.revision+1}};
        records.set(body.draftId,next);return structuredClone(next);
      }
      if(route==="/api/capture/draft/input-mutation")return{status:record.lastMutation.mutationId===body.mutationId&&record.lastMutation.fingerprint===body.fingerprint?"applied":"unknown",draft:structuredClone(record)};
      if(route==="/api/capture/draft/flush")return{draftId:record.draftId,recordRevision:record.revision,inputRevision:record.inputRevision,inputCut:"full",lastMutationId:record.lastMutation.mutationId,pendingOperationId:null,recordDigest:"d".repeat(64)};
      throw new Error(`Unexpected fixture route ${route}`);
    };
    const signals=[],nativeAdd=document.addEventListener.bind(document);
    document.addEventListener=(type,listener,options)=>{if(options&&typeof options==="object"&&options.signal)signals.push(options.signal);return nativeAdd(type,listener,options);};
    const host=document.getElementById("capture"),recovery=new PersonalInputRecovery();
    const mounted=await mountLocalCapture(host,{request,options:{draftId:a,recovery}});
    const root=host.querySelector(`[data-control-key='${JSON.stringify([a,"rootInput"])}']`);
    const body=host.querySelector(`[data-control-key='${JSON.stringify(["capture",a,"body"])}']`);
    const field=[...host.querySelectorAll(".capture-field input")].find(control=>nativeControlAddress(control).role==="value:int");
    if(!root||!body||!field)throw new Error("actual local target/body/field controls not mounted");
    const oldSignals=[...new Set(signals)];if(!oldSignals.length)throw new Error("actual mount input listeners have no lifetime signal");
    const old={root,body,field,rootLease:captureAuthorInput(root),bodyLease:captureAuthorInput(body),fieldLease:captureAuthorInput(field)};
    if(!old.rootLease||!old.bodyLease||!old.fieldLease)throw new Error("actual old control leases missing");
    window.localCaptureMount={mounted,records,calls,a,b,old,async retireAndInstallB(){
      await mounted.session.controller.drain();await mounted.session.settleForHandoff();
      mounted.dispose();
      const next=await mountLocalCapture(host,{request,options:{draftId:b,recovery}});
      this.next=next;this.bBefore=JSON.stringify(next.session.controller.state);
      resolveSchema({schema,roleRef:basis.schemaRef,semanticDigest:basis.semanticDigest});
    },snapshot(){return{a:records.get(a),b:records.get(b),calls:structuredClone(calls),bState:JSON.stringify(this.next.session.controller.state),schemaReturned,oldListenersAborted:oldSignals.every(signal=>signal.aborted),newListenersLive:signals.some(signal=>!oldSignals.includes(signal)&&!signal.aborted)};},startSchema(){holdSchema=true;[...host.querySelectorAll("button")].find(button=>button.textContent===t(message("capture.selectTarget"))).click();}};
    return{a,b,root:root.dataset.controlKey,body:body.dataset.controlKey,field:field.dataset.controlKey};
  });
  // Existing real local target control triggers real selectTarget + loadFields.
  // Hold that actual schema request after old fields have been retired.
  await page.evaluate(()=>window.localCaptureMount.startSchema());
  await page.waitForFunction(()=>window.captureMountSchemaEntered===true,{},{timeout:10000});
  await page.evaluate(()=>window.localCaptureMount.retireAndInstallB());
  const before=await page.evaluate(()=>window.localCaptureMount.snapshot());
  const after=await page.evaluate(async()=>{
    const f=window.localCaptureMount,accepted=[],retiredControls=document.createElement("section");retiredControls.hidden=true;document.body.append(retiredControls);
    for(const name of ["root","body","field"]){const control=f.old[name],lease=f.old[name+"Lease"];accepted.push(lease.apply("LATE OLD INPUT"));lease.release();retiredControls.append(control);control.value="LATE OLD EVENT";for(const type of ["beforeinput","input","change","compositionstart","compositionend"])control.dispatchEvent(new InputEvent(type,{bubbles:true,inputType:"insertText",data:"LATE OLD EVENT"}));}
    f.mounted.form.collectOwnedRaw();await f.next.session.controller.drain();
    return{accepted,...f.snapshot(),bBefore:f.bBefore,liveFields:document.querySelectorAll(".capture-field").length,body:document.querySelector(`[data-control-key='${JSON.stringify(["capture",f.b,"body"])}']`).value};
  });
  assert.deepEqual(after.accepted,[false,false,false]);assert.equal(after.bState,after.bBefore);assert.deepEqual(after.b,before.b);assert.deepEqual(after.a,before.a);assert.equal(after.calls.length,before.calls.length);assert.equal(after.schemaReturned,true);assert.equal(after.oldListenersAborted,true);assert.equal(after.newListenersLive,true);assert.equal(after.liveFields,1);assert.equal(after.body,"B body");
  result.checks.push("actual old target/body/field leases and events retire; late actual schema result cannot recreate A fields or alter B/current controller");
  assert.equal(result.errors.length,0);await page.screenshot({path:path.join(out,"capture-mount-retired.png"),fullPage:true});result.completed=true;
}catch(error){result.failure=error.stack||String(error);process.exitCode=1;}
finally{if(browser){await browser.close();result.ownership.browserClosed=true;}if(vite){await vite.close();result.ownership.viteClosed=true;}await writeFile(path.join(out,"result.json"),JSON.stringify(result,null,2));}
