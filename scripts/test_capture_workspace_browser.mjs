/** Real current Capture workspace/DOM/IndexedDB against an explicit immutable
 * CLI Core. No mock Core, native-window, OS IME, AT or whole-package claim. */
import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, writeFile, readdir } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import os from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnServe, serveReady, stopServe, stringifyServeJson } from "./_cli_serve.mjs";
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),".."),exe=process.env.WEFTEXT_TEST_EXECUTABLE,out=process.env.WEFTEXT_BROWSER_REPORT;
if(!exe||!path.isAbsolute(exe)||!out||!path.isAbsolute(out))throw new Error("Explicit absolute WEFTEXT_TEST_EXECUTABLE and new WEFTEXT_BROWSER_REPORT required");
await mkdir(out);
const fixtureParent=process.env.WEFTEXT_BROWSER_FIXTURE_PARENT??os.tmpdir();
if(!path.isAbsolute(fixtureParent))throw new Error("WEFTEXT_BROWSER_FIXTURE_PARENT must be absolute when specified");
const fixture=await mkdtemp(path.join(fixtureParent,"weftext-cw-")),data=path.join(fixture,"p"),chrome=path.join(fixture,"chrome"),libraries=[path.join(fixture,"a"),path.join(fixture,"b")];
const sha=value=>createHash("sha256").update(value).digest("hex"),pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const sourceFiles=["scripts/test_capture_workspace_browser.mjs","scripts/_cli_serve.mjs","app/src/capture-workspace.ts","app/src/capture-mount.ts","app/src/capture-local.ts","app/src/capture-controller.ts","app/src/capture-form.ts","app/src/capture-fields.ts","app/src/capture-input-persistence.ts","app/src/capture-local-persistence.ts","app/src/capture-author-outbox.ts","app/src/capture-close.ts","app/src/personal-control-input.ts","app/src/personal-input-recovery.ts","app/src/personal-input-storage.ts","app/src/api.ts","app/src/ui-locale.ts","app/src/ui-messages.ts","app/src/capture.css"];
const source={};for(const file of sourceFiles)source[file]=sha(await readFile(path.join(repo,file)));
const cliSha256=sha(await readFile(exe));if(process.env.WEFTEXT_TEST_EXECUTABLE_SHA256)assert.equal(cliSha256,process.env.WEFTEXT_TEST_EXECUTABLE_SHA256);
const result={scope:"Current Capture workspace + real local CLI Core + headless Chrome/IndexedDB; no native/IME/AT/full-package acceptance",exe,cliSha256,source,fixture:{parent:fixtureParent,root:fixture,data,chrome,libraries},cases:[],pageErrors:[],owners:[],createRequests:[],lostCreate:null,completed:false,ownership:{browserClosed:false,viteClosed:false}};
for(const file of ["artifact-catalog.json","build-input-receipt.json"]){const bytes=await readFile(path.join(path.dirname(exe),file));result[file]={path:path.join(path.dirname(exe),file),sha256:sha(bytes)};}
await writeFile(path.join(out,"inputs.json"),stringifyServeJson(result,2));
const {chromium}=createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE||"playwright");
const {createServer}=await import(pathToFileURL(path.join(repo,"app/node_modules/vite/dist/node/index.js")).href);
let child,ready,browser,page,vite,port,stopAttempted=false,apiCounter=0;
async function startOwner(){stopAttempted=false;child=spawnServe(exe,["--data-dir",data],{windowsHide:true,stdio:["ignore","pipe","pipe"]});const deadline=Date.now()+30000;while(Date.now()<deadline){const value=serveReady(child);if(value){ready=value;result.owners.push({ready:value,spawnedPid:child.pid});return;}await pause(80);}throw new Error("Owned CLI Ready timed out; do not invent identity");}
async function stopOwner(){if(!child||stopAttempted)return;stopAttempted=true;const receipt=await stopServe(child);assert.equal(child.exitCode,0);assert.equal(child.signalCode,null);Object.assign(result.owners.at(-1),{receipt,actualExitCode:child.exitCode,actualSignal:child.signalCode});child=undefined;}
async function api(route,body){const name=String(++apiCounter).padStart(3,"0");await writeFile(path.join(out,`${name}-request.json`),JSON.stringify({route,body},null,2),{flag:"wx"});const response=await fetch(ready.base+route,{method:"POST",headers:{"Content-Type":"application/json","X-Weftext-Token":ready.token},body:JSON.stringify(body),signal:AbortSignal.timeout(45000)}),bytes=Buffer.from(await response.arrayBuffer());await writeFile(path.join(out,`${name}-response.json`),bytes,{flag:"wx"});const value=JSON.parse(bytes);assert.equal(response.status,200,route+": "+JSON.stringify(value));return value;}
async function startBrowser(){
  result.ownership.browserClosed=false;result.ownership.viteClosed=false;
  vite=await createServer({root:path.join(repo,"app"),configFile:false,server:{host:"127.0.0.1",port:port??0,strictPort:true,hmr:false,watch:{ignored:["**/*"]},proxy:{"/api":{target:ready.base,changeOrigin:true,configure(instance){instance.on("proxyReq",(request,incoming)=>{if(incoming.headers.origin)request.setHeader("Origin",ready.base);});}}}},plugins:[{name:"capture-workspace-owned-fixture",configureServer(server){server.middlewares.use("/capture-workspace-fixture",(_request,response)=>{response.setHeader("Content-Type","text/html;charset=utf-8");response.end('<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/style.css"><link rel="stylesheet" href="/src/data-view.css"><link rel="stylesheet" href="/src/capture.css"><main id="capture"></main>');});}}]});
  await vite.listen();const actualPort=vite.httpServer.address().port;if(port!==undefined)assert.equal(actualPort,port);port=actualPort;
  browser=await chromium.launchPersistentContext(chrome,{channel:"chrome",headless:true,viewport:{width:1280,height:900}});page=await browser.newPage();page.setDefaultTimeout(15000);page.on("pageerror",error=>result.pageErrors.push(error.message));
  page.on("request",request=>{if(new URL(request.url()).pathname==="/api/capture/draft/create"){const wire=request.postData();result.createRequests.push({draftId:JSON.parse(wire).draftId,mutationId:JSON.parse(wire).mutationId,wireSha256:sha(wire)});}});
  await page.goto(`http://127.0.0.1:${port}/capture-workspace-fixture#token=${ready.token}`);
  await page.evaluate(async()=>{const[{LocalCaptureWorkspace},{api},{t,message}]=await Promise.all([import("/src/capture-workspace.ts"),import("/src/api.ts"),import("/src/ui-locale.ts")]);window.workspace=new LocalCaptureWorkspace(document.getElementById("capture"),{request:api,options:{pointerStorage:localStorage}});await window.workspace.open();window.captureState=()=>window.workspace.mounted.session.controller.state;window.captureLabel=key=>t(message(key));});
  result.browser=await page.evaluate(()=>navigator.userAgent);
}
async function closeBrowser(){if(browser){await browser.close();browser=undefined;result.ownership.browserClosed=true;}if(vite){await vite.close();vite=undefined;result.ownership.viteClosed=true;}}
const label=key=>page.evaluate(key=>window.captureLabel(key),key);
const button=async key=>page.getByRole("button",{name:await label(key),exact:true}).click();
const state=()=>page.evaluate(()=>window.captureState());
const keyControl=key=>page.locator(`[data-control-key=${JSON.stringify(key)}]`);
async function settle(){await page.evaluate(async()=>{const mount=window.workspace.mounted;mount.form.collectOwnedRaw();await mount.session.settleForHandoff();});}
async function targetControls(id){const root=page.getByRole("textbox",{name:await label("capture.target"),exact:true}),parent=page.getByRole("textbox",{name:await label("capture.parent"),exact:true});assert.equal(await root.getAttribute("data-control-key"),JSON.stringify([id,"rootInput"]));assert.equal(await parent.getAttribute("data-control-key"),JSON.stringify([id,"parentInput"]));return{root,parent};}
async function selectTarget(root){const id=(await state()).draft.draftId,controls=await targetControls(id);await controls.root.fill(root);await controls.parent.fill(root);await button("capture.selectTarget");await page.waitForFunction(root=>window.captureState().draft.target?.rootHint===root,root);await page.waitForFunction(()=>!window.captureState().persisting&&!window.workspace.mounted.form.pendingAction);}
async function restore(id){await page.getByRole("combobox",{name:await label("capture.savedDrafts"),exact:true}).selectOption(id);await button("capture.restoreDraft");await page.waitForFunction(id=>window.captureState().draft.draftId===id&&!window.workspace.switching,id);}
async function install(root){const initial=await api("/api/data/schema",{root}),schema=JSON.parse(initial.source);schema.fields.push({id:"test/count",key:"count",shape:{kind:"scalar",semantic:"int"},nullable:false,default:{kind:"absent"}});schema.types.push({id:"test/capture",fields:[{id:"test/count",required:false}],requires:[],conflicts:[],constraints:[]});schema.aliases.capturetest="test/capture";const operationId=randomUUID(),source=JSON.stringify(schema,null,2);const preview=await api("/api/data/schema/prepare",{root,source,beforeSource:initial.source,beforeRevision:initial.revision,operationId});const receipt=await api("/api/data/schema/install",{operationId:preview.operationId,planId:preview.planId});assert.equal(receipt.status,"committed");return{root,operationId,receipt,sourceSha256:sha(source)};}
try{
  await startOwner();result.libraryReceipts=[];result.schemas=[];
  for(const root of libraries){result.libraryReceipts.push(await api("/api/library/create",{path:root,operationId:randomUUID()}));result.schemas.push(await install(root));}
  await startBrowser();const a=(await state()).draft.draftId;result.a=a;
  await selectTarget(libraries[0]);await keyControl(JSON.stringify(["capture",a,"body"])).fill("A retained 中文 body\nCW-A-20261026");
  await page.locator("#capture summary").click();await page.locator("#capture select[multiple]").selectOption("test/capture");
  const row=page.locator("#capture .capture-field").filter({has:page.getByRole("heading",{name:"count",exact:true})});await row.getByRole("button",{name:await label("quickCapture.fillField"),exact:true}).click();
  const fieldKey=await page.evaluate(async()=>{const{nativeControlAddress}=await import("/src/native-control-identity.ts");return[...document.querySelectorAll("#capture .capture-field input")].map(control=>nativeControlAddress(control)).find(address=>address.identity.fieldId==="test/count"&&address.role==="value:int").key;});
  await keyControl(fieldKey).fill("-");const rawTarget=libraries[1]+"-not-selected";await (await targetControls(a)).root.fill(rawTarget);await settle();
  const aBefore=await api("/api/capture/draft/read",{draftId:a});assert.equal(aBefore.target.rootHint,libraries[0]);assert.deepEqual(aBefore.currentInput.rootInput.utf16,[...rawTarget].map(c=>c.charCodeAt(0)));assert.ok(aBefore.currentInput.rawInputs.some(raw=>raw.fieldId==="test/count"&&String.fromCharCode(...raw.text.utf16)==="-"));
  await writeFile(path.join(out,"a-before.json"),JSON.stringify(aBefore,null,2));
  let dropped=false;
  await page.route("**/api/capture/draft/create",async route=>{
    if(dropped){await route.continue();return;}dropped=true;
    const wire=route.request().postData(),command=JSON.parse(wire),response=await route.fetch(),bytes=await response.body();assert.equal(response.status(),200);
    const actual=JSON.parse(bytes);assert.equal(actual.draftId,command.draftId);assert.equal(actual.lastMutation.mutationId,command.mutationId);
    await writeFile(path.join(out,"lost-create-request.json"),wire,{flag:"wx"});await writeFile(path.join(out,"lost-create-core-response.json"),bytes,{flag:"wx"});
    result.lostCreate={draftId:command.draftId,mutationId:command.mutationId,requestSha256:sha(wire),responseSha256:sha(bytes),actualHttpStatus:response.status(),droppedAfterConfirmedCoreResponse:true};
    await writeFile(path.join(out,"lost-create-identity.json"),JSON.stringify(result.lostCreate,null,2),{flag:"wx"});
    await route.abort("connectionreset");
  });
  await button("capture.startOtherDraft");await page.waitForFunction(a=>window.captureState().draft.draftId!==a&&!window.workspace.switching,a);
  const b=(await state()).draft.draftId;result.b=b;assert.equal(b,result.lostCreate.draftId);assert.equal((await state()).draft.target,null);assert.deepEqual((await state()).input.body.utf16,[]);assert.deepEqual((await state()).input.fields,[]);assert.equal(result.createRequests.filter(request=>request.draftId===b).length,1);
  const list=await api("/api/capture/draft/list",{});assert.deepEqual(list.map(item=>item.draftId).sort(),[a,b].sort());result.cases.push("one confirmed Core empty create reply lost; original mutation reconciled without replacement draft");
  await selectTarget(libraries[1]);await keyControl(JSON.stringify(["capture",b,"body"])).fill("B independent 中文 body\nCW-B-20261026");await settle();const bBefore=await api("/api/capture/draft/read",{draftId:b});await writeFile(path.join(out,"b-before.json"),JSON.stringify(bBefore,null,2));
  await restore(a);assert.deepEqual((await state()).input,aBefore.currentInput);assert.equal(await keyControl(fieldKey).inputValue(),"-");assert.equal(await (await targetControls(a)).root.inputValue(),rawTarget);
  await restore(b);assert.deepEqual((await state()).input,bBefore.currentInput);assert.equal((await state()).draft.target.rootHint,libraries[1]);result.cases.push("actual A/B mounts round-trip exact Core inputs, raw target/advanced fragments and original IDs");
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),"1280 viewport must not overflow horizontally");await page.screenshot({path:path.join(out,"workspace-1280.png"),fullPage:true});await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),"390 viewport must not overflow horizontally");await page.screenshot({path:path.join(out,"workspace-390.png"),fullPage:true});
  await settle();await closeBrowser();await stopOwner();await startOwner();await startBrowser();
  assert.equal((await state()).draft.draftId,b);assert.deepEqual((await state()).input,bBefore.currentInput);
  await restore(a);assert.deepEqual((await state()).input,aBefore.currentInput);assert.equal(await keyControl(fieldKey).inputValue(),"-");
  const finalList=await api("/api/capture/draft/list",{});assert.deepEqual(finalList.map(item=>item.draftId).sort(),[a,b].sort());result.cases.push("normal browser close plus typed exact-owner stop; new owner and same Chrome/IndexedDB profile reopen both original drafts");
  await settle();await closeBrowser();await stopOwner();assert.deepEqual(result.pageErrors,[]);
  for(const file of sourceFiles)assert.equal(sha(await readFile(path.join(repo,file))),source[file],"source changed: "+file);assert.equal(sha(await readFile(exe)),cliSha256);
  result.completed=true;
}catch(error){result.failure=error.stack||String(error);process.exitCode=1;}
finally{
  try{await closeBrowser();}catch(error){result.browserCleanupError=String(error);process.exitCode=1;}
  try{await stopOwner();}catch(error){result.ownerCleanupError=String(error);process.exitCode=1;}
  result.lifecycleEvidence=[];for(const entry of await readdir(fixture,{withFileTypes:true})){if(entry.isDirectory()&&entry.name.startsWith("owner-control-")){const dir=path.join(fixture,entry.name),files=[];for(const file of await readdir(dir)){const actual=path.join(dir,file);files.push({path:actual,sha256:sha(await readFile(actual))});}result.lifecycleEvidence.push({directory:dir,files});}}
  await writeFile(path.join(out,"result.json"),stringifyServeJson(result,2));
}
console.log(JSON.stringify({completed:result.completed,cases:result.cases.length,out,fixture}));
