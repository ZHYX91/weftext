// Optional real-browser acceptance. Supply a qualified Playwright installation
// through WEFTEXT_PLAYWRIGHT_MODULE, plus EXE and a new owned evidence directory.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID, randomBytes } from 'node:crypto';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe=path.resolve(process.argv[2]), out=path.resolve(process.argv[3]);
await mkdir(out,{recursive:false});
const sha=b=>createHash('sha256').update(b).digest('hex');
const result={exe,sha256:sha(await readFile(exe)),version:execFileSync(exe,['--version'],{windowsHide:true}).toString().trim(),cases:[],pageErrors:[],consoleErrors:[],completed:false};
const pause=ms=>new Promise(r=>setTimeout(r,ms));
function bounded(promise,name){let timer;return Promise.race([promise,new Promise((_,reject)=>timer=setTimeout(()=>reject(new Error('Timed out '+name)),30000))]).finally(()=>clearTimeout(timer));}
let local,server,browser,page,base,localToken,config,stderr='';
const routeErrors=[];result.routeErrors=routeErrors;
const token={alice:randomBytes(32).toString('hex'),bob:randomBytes(32).toString('hex'),tagger:randomBytes(32).toString('hex'),writer:randomBytes(32).toString('hex'),admin:randomBytes(32).toString('hex')};
async function start(args,hosted=false){
 const p=spawn(exe,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});let text='';p.stdout.on('data',b=>text+=b);p.stderr.on('data',b=>stderr+=b);
 for(let i=0;i<300;i++){const match=text.match(hosted?/(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/:/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);if(match){base=match[1];localToken=match[2];return p;}if(p.exitCode!==null)throw new Error(stderr);await pause(100);}p.kill();throw new Error('startup timeout');
}
async function stop(p){if(p && p.exitCode===null && p.signalCode===null){const done=new Promise(resolve=>p.once('exit',resolve));p.kill();await done;}}
async function api(route,data,actor){
 const r=await fetch(base+route,{method:data===undefined?'GET':'POST',headers:{'Content-Type':'application/json',...(actor?{Authorization:'Bearer '+token[actor]}:{'X-Weftext-Token':localToken})},body:data===undefined?undefined:JSON.stringify(data)});const body=await r.json();assert.ok(r.ok,JSON.stringify(body));return body;
}
async function click(name){await page.getByRole('button',{name,exact:true}).click();}
async function readField(){const response=page.waitForResponse(r=>r.url().endsWith('/api/hosted/fields/read'));await click('读取字段');assert.equal((await response).status(),200);await page.locator('#fields fieldset').first().waitFor();}
async function changePolicy(edit){const p=await api('/api/hosted/admin/policy',undefined,'admin');edit(p.users);return api('/api/hosted/admin/policy',{expectedGeneration:p.generation,users:p.users},'admin');}
async function passed(message){result.cases.push(message);await writeFile(path.join(out,'progress.json'),JSON.stringify(result,null,2));console.log(message);}
const tagRefs=value=>(value?.kind==='array'?value.value:[]).flatMap(item=>item.kind==='record'?item.value.filter(member=>member.key==='ref'&&member.value.kind==='text').map(member=>member.value.value):[]);
async function tagProfile(){return api('/api/hosted/fields/read',{resource:'shared',fieldId:'core/tags'},'tagger');}
async function registry(){return api('/api/hosted/navigation/registry',undefined,'tagger');}
function observePage(target){target.on('pageerror',error=>result.pageErrors.push(String(error)));target.on('dialog',dialog=>dialog.accept());target.on('console',message=>{if(message.type()==='error')result.consoleErrors.push(message.text());});}
async function tagReady(){await page.getByLabel('选择获权字段').selectOption({label:'shared · core/tags'});await readField();await page.getByLabel('新建并赋值标签名称',{exact:true}).waitFor();await page.waitForFunction(()=>!document.querySelector('#fields fieldset')?.disabled);const rebase=page.getByRole('button',{name:'已核对，采用当前字段依据',exact:true});if(await rebase.isEnabled())await rebase.click();}
async function saveTagField(){const response=page.waitForResponse(r=>r.url().endsWith('/api/hosted/fields/write'));await click('保存字段');const received=await response;assert.equal(received.status(),200,await received.text());await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();return received.request().postDataJSON();}
async function waitTagAction(name){await page.waitForFunction(name=>Array.from(document.querySelectorAll('#fields button')).some(button=>button.textContent===name&&!button.disabled),name);}
async function cachedTagJob(){return page.evaluate(()=>Object.keys(sessionStorage).filter(key=>key.startsWith('weftext-hosted-field:')).map(key=>JSON.parse(sessionStorage[key])).find(draft=>draft.tagCreation)?.tagCreation);}
function maskFixtureTags(source){const key=source.indexOf('tags:');assert.ok(key>=0);let start=key+5;while(/\s/.test(source[start]))start++;assert.equal(source[start],'(');let depth=0,string=false;for(let i=start;i<source.length;i++){const c=source[i];if(string){if(c==='\\')i++;else if(c==='"')string=false;continue;}if(c==='"')string=true;else if(c==='(')depth++;else if(c===')'&&!--depth)return source.slice(0,start)+'<tags-value>'+source.slice(i+1);}throw new Error('owned fixture tag tuple did not close');}
try{
 const data=path.join(out,'state'), root=path.join(out,'library');
 local=await start(['--no-browser','--data-dir',data]);
 await api('/api/library/create',{path:root,operationId:randomUUID()});
 const schema=await api('/api/data/schema',{root});const op=randomUUID();
 const testSchema=JSON.parse(schema.source);testSchema.fields.find(f=>f.id==='tasks/priority').shape={kind:'scalar',semantic:'int'};
 const plan=await api('/api/data/schema/prepare',{root,source:JSON.stringify(testSchema,null,2),beforeSource:schema.source,beforeRevision:schema.revision,operationId:op});
 await api('/api/data/schema/install',{operationId:op,planId:plan.planId});
 const makeTag=async displayName=>{const registry=await api('/api/navigation/tags',{root}),prepared=await api('/api/navigation/create-tag',{root,displayName,parentRef:null,baseId:registry.baseId,operationId:randomUUID()});assert.equal((await api('/api/operations/commit',{operationId:prepared.preview.operationId,planId:prepared.preview.planId})).status,'committed');return prepared.targetRef;};
 const parentRef=await makeTag('归组 / Parent'),otherRef=await makeTag('直接 / Other');result.tagFixture={parentRef,otherRef};
 const note=await api('/api/note/create',{root,parent:root,name:'Shared',operationId:randomUUID()});
 const initial=note.source.replace('types: ()',`types: ("task",), status: "open", priority: 7, aliases: ("中文😀",), tags: ((ref: "${otherRef}"), (text: "literal / 保留")), hidden: read("PRIVATE_BODY")`)+'\nHIDDEN_BODY_987\n';
 await api('/api/save',{sessionId:note.sessionId,baseId:note.baseId,source:initial,operationId:randomUUID()});
 await stop(local);local=null;
 const grant={fields:{'tasks/status':{read:true,write:true},'tasks/priority':{read:true,write:true},'core/aliases':{read:true,write:true}}};
 config={deployment:'field-browser',bind:'127.0.0.1:0',root,adminTokenSha256:sha(token.admin),files:{shared:{path:'Shared/Shared.typ',title:'共同文档',editable:true}},users:{
  alice:{enabled:true,tokenSha256:sha(token.alice),grants:{shared:grant}},
  bob:{enabled:true,tokenSha256:sha(token.bob),grants:{shared:grant}},
  tagger:{enabled:true,tokenSha256:sha(token.tagger),grants:{shared:{fields:{'core/tags':{read:true,write:true}}}},navigation:{tags:{read:true,create:true,rename:false,move:false}}},
  writer:{enabled:true,tokenSha256:sha(token.writer),grants:{shared:{read:true,write:true,history:true}}}}};
 const configPath=path.join(out,'server.json');await writeFile(configPath,JSON.stringify(config,null,2));
 server=await start(['--no-browser','--data-dir',data,'--server-config',configPath],true);
 browser=await chromium.launch({channel:'chrome',headless:true});result.browser=browser.version();
 const ctx=await browser.newContext({viewport:{width:1440,height:1100}});page=await ctx.newPage();
 observePage(page);
 await page.goto(base);await page.getByLabel('访问凭据',{exact:true}).fill(token.alice);await click('连接');
 await page.getByLabel('选择获权字段').waitFor();assert.equal(await page.locator('#documents button').count(),0);
 await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/status'});await readField();
 assert.equal(await page.getByLabel('字段输入',{exact:true}).inputValue(),'open');
 assert.ok(!(await page.locator('body').innerText()).includes('HIDDEN_BODY_987'));
 await page.getByLabel('查询字段').selectOption('tasks/status');await page.getByLabel('CEL 筛选条件').fill('status == "open"');
 await click('运行查询');await page.getByRole('table',{name:'获权字段查询结果'}).waitFor();assert.match(await page.getByRole('table').innerText(),/open/);
 await page.screenshot({path:path.join(out,'field-query.png'),fullPage:true});
 result.cases.push('field-only form and typed CEL Query; no source document or hidden body delivered');
 const doc=await api('/api/hosted/open',{resource:'shared'},'writer');
 const hidden=doc.source+'\n新的隐藏正文😀\n';
 await api('/api/hosted/save',{sessionId:doc.sessionId,baseId:doc.baseId,source:hidden,operationId:randomUUID()},'writer');
 await page.getByLabel('字段输入',{exact:true}).selectOption('done');
 await click('保存字段');await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();
 assert.equal(await readFile(path.join(root,'Shared/Shared.typ'),'utf8'),hidden.replace('status: "open"','status: "done"'));
 result.cases.push('field write rebases across hidden-only body change and preserves all other source bytes');
 await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/priority'});await readField();
 let invalidWrites=0;const countInvalidWrites=request=>{if(request.url().endsWith('/api/hosted/fields/write'))invalidWrites++;};page.on('request',countInvalidWrites);
 await page.getByLabel('字段输入',{exact:true}).fill('-');await click('保存字段');
 assert.equal(invalidWrites,0);assert.equal(await page.getByLabel('字段输入',{exact:true}).inputValue(),'-');
 const invalidQuery=page.waitForResponse(r=>r.url().endsWith('/api/hosted/query'));await click('运行查询');assert.equal((await invalidQuery).status(),200);
 await page.waitForFunction(()=>!document.querySelector('#fields fieldset').disabled);
 assert.equal(await page.getByLabel('字段输入',{exact:true}).inputValue(),'-');
 await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/status'});await readField();
 await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/priority'});await readField();
 assert.equal(await page.getByLabel('字段输入',{exact:true}).inputValue(),'-');
 await page.reload();await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/priority'});await readField();
 assert.equal(await page.getByLabel('字段输入',{exact:true}).inputValue(),'-');await click('已核对，采用当前字段依据');await click('保存字段');
 assert.equal(invalidWrites,0);assert.equal((await api('/api/hosted/fields/read',{resource:'shared',fieldId:'tasks/priority'},'bob')).field.cell.value.value,'7');
 await page.getByLabel('字段输入',{exact:true}).fill('9');await click('保存字段');await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();
 assert.equal(invalidWrites,1);page.off('request',countInvalidWrites);
 assert.equal((await api('/api/hosted/fields/read',{resource:'shared',fieldId:'tasks/priority'},'bob')).field.cell.value.value,'9');
 result.cases.push('invalid integer raw input survives save validation, Query, field switching and reload without submitting the previous valid value');
 await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/status'});await readField();
 await click('重新读取字段');await click('已核对，采用当前字段依据');
 await page.getByLabel('字段输入',{exact:true}).selectOption('cancelled');
 let pending;
 await page.route('**/api/hosted/fields/write',async route=>{pending=route.request().postDataJSON();const response=await route.fetch();assert.equal(response.status(),200);await route.abort('failed');});
 await click('保存字段');await page.waitForFunction(()=>Object.keys(sessionStorage).some(k=>k.startsWith('weftext-hosted-field:') && JSON.parse(sessionStorage[k]).pending));
 await page.waitForTimeout(300);await page.unroute('**/api/hosted/fields/write');await page.reload();
 await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/status'});await readField();
 await click('核对原字段操作');await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();
 const receipt=await api('/api/hosted/fields/operation',{resource:'shared',fieldId:'tasks/status',operationId:pending.operationId},'alice');assert.equal(receipt.status,'committed');
 assert.ok(await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-field:')).every(k=>!JSON.parse(sessionStorage[k]).pending)));
 result.cases.push('lost committed response survives reload and resolves the original field operation');
 await click('重新读取字段');await click('已核对，采用当前字段依据');await page.getByLabel('字段输入',{exact:true}).selectOption('done');
 let committed;const committedWrite=new Promise(resolve=>committed=resolve);
 await page.route('**/api/hosted/fields/write',async route=>{pending=route.request().postDataJSON();const response=await route.fetch();assert.equal(response.status(),200);committed();await route.abort('failed');});
 await click('保存字段');await committedWrite;await page.waitForFunction(()=>!Array.from(document.querySelectorAll('#fields button')).find(b=>b.textContent==='核对原字段操作').disabled);await page.unroute('**/api/hosted/fields/write');
 const notePath=path.join(root,'Shared/Shared.typ'), accepted=await readFile(notePath,'utf8');await writeFile(notePath,accepted+'\nEXTERNAL_UNADMITTED_CHANGE\n');
 await page.reload();await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · tasks/status'});
 const unavailableRead=page.waitForResponse(r=>r.url().endsWith('/api/hosted/fields/read'));await click('读取字段');assert.equal((await unavailableRead).status(),409);
 await click('核对原字段操作');await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();
 assert.equal((await api('/api/hosted/fields/operation',{resource:'shared',fieldId:'tasks/status',operationId:pending.operationId},'alice')).status,'committed');
 assert.ok(await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-field:')).every(k=>!JSON.parse(sessionStorage[k]).pending)));
 const externalDoc=await api('/api/hosted/open',{resource:'shared'},'writer');await api('/api/hosted/save',{sessionId:externalDoc.sessionId,baseId:externalDoc.baseId,source:accepted.replace('status: "done"','status: "cancelled"'),operationId:randomUUID()},'writer');
 result.cases.push('lost committed operation remains recoverable after reload when external source changes make fields/read return 409');
 const restoredRead=page.waitForResponse(r=>r.url().endsWith('/api/hosted/fields/read'));await click('重新读取字段');assert.equal((await restoredRead).status(),200);await click('已核对，采用当前字段依据');
 await page.getByLabel('字段输入',{exact:true}).selectOption('done');await page.route('**/api/hosted/fields/write',route=>route.abort('failed'));await click('保存字段');
 await page.waitForFunction(()=>!Array.from(document.querySelectorAll('#fields button')).find(b=>b.textContent==='核对原字段操作').disabled);await page.unroute('**/api/hosted/fields/write');
 // Inject only the terminal receipt to verify the client state machine; this is
 // not evidence of a real crash recovery producing the Core resolved state.
 await page.route('**/api/hosted/fields/operation',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({status:'resolved'})}));
 await click('核对原字段操作');await page.locator('#fields').getByText('原操作已明确结束，未确认提交；输入保留。请重新读取并核对。',{exact:true}).waitFor();await page.unroute('**/api/hosted/fields/operation');
 assert.ok(await page.getByRole('button',{name:'保存字段',exact:true}).isDisabled());assert.ok(await page.getByRole('button',{name:'已核对，采用当前字段依据',exact:true}).isDisabled());
 assert.ok(await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-field:')).every(k=>!JSON.parse(sessionStorage[k]).pending)));
 result.cases.push('injected resolved terminal receipt clears pending without claiming a commit and requires a newly read basis');
 await page.getByLabel('选择获权字段').selectOption({label:'shared · core/aliases'});await readField();
 const alias=page.getByLabel('字段输入 第 1 项',{exact:true});await ctx.setOffline(true);await alias.fill('离线中文😀输入');
 assert.ok((await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-field:')).map(k=>sessionStorage[k]).join(''))).includes('离线中文😀输入'));
 await ctx.setOffline(false);await page.reload();await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · core/aliases'});await readField();
 assert.equal(await alias.inputValue(),'离线中文😀输入');await click('已核对，采用当前字段依据');await click('保存字段');
 await page.locator('#fields').getByText('原字段操作已保存。继续编辑前，请重新读取并核对当前字段。',{exact:true}).waitFor();
 assert.ok((await readFile(path.join(root,'Shared/Shared.typ'),'utf8')).includes('离线中文😀输入'));
 result.cases.push('native array form preserves offline Chinese/non-BMP draft, reload and explicit rebase');
 // The following UI chain has only a narrow field grant plus independently
 // declared tag read/create rights. The writer token remains fixture-only.
 const alicePage=page,tagContext=await browser.newContext({viewport:{width:1440,height:1100}});page=await tagContext.newPage();observePage(page);await page.goto(base);await page.getByLabel('访问凭据',{exact:true}).fill(token.tagger);await click('连接');await page.getByLabel('选择获权字段').waitFor();
 const actualPolicy=await api('/api/hosted/admin/policy',undefined,'admin'),narrow=actualPolicy.users.tagger.grants.shared;
 assert.deepEqual(Object.keys(narrow.fields),['core/tags']);assert.equal(narrow.read??false,false);assert.equal(narrow.write??false,false);assert.equal(narrow.history??false,false);assert.equal(actualPolicy.users.tagger.navigation.tags.read,true);assert.equal(actualPolicy.users.tagger.navigation.tags.create,true);assert.equal(actualPolicy.users.tagger.navigation.tags.rename,false);assert.equal(actualPolicy.users.tagger.navigation.tags.move,false);
 const sourceDenied=await fetch(base+'/api/hosted/open',{method:'POST',headers:{'Content-Type':'application/json',Authorization:'Bearer '+token.tagger},body:JSON.stringify({resource:'shared'})});assert.equal(sourceDenied.status,403);
 let tagPrepares=0,tagCommits=0,tagCancels=0,tagLookups=0;page.on('request',request=>{const route=new URL(request.url()).pathname;if(route.endsWith('/navigation/registry/prepare'))tagPrepares++;if(route.endsWith('/navigation/registry/commit'))tagCommits++;if(route.endsWith('/navigation/registry/settle'))tagCancels++;if(route.endsWith('/navigation/registry/operation'))tagLookups++;});
 await tagReady();assert.equal(await page.locator('#documents button').count(),0);assert.equal(await page.locator('#source .cm-content').count(),0);assert.ok(!(await page.locator('body').innerText()).includes('HIDDEN_BODY_987'));
 const picker=page.locator('#fields .data-tag-reference-picker select').first();assert.equal(await picker.inputValue(),otherRef);assert.match(await picker.locator('option').filter({hasText:'归组 / Parent'}).innerText(),/"归组 \/ Parent"/);
 await picker.selectOption(parentRef);const direct=await saveTagField();assert.deepEqual(Object.keys(direct).sort(),['basis','fieldId','operationId','resource','value']);assert.deepEqual(tagRefs((await tagProfile()).field.cell.value),[parentRef]);
 await tagReady();await picker.selectOption(otherRef);await saveTagField();assert.deepEqual(tagRefs((await tagProfile()).field.cell.value),[otherRef]);
 await passed('narrow field-only tag picker displays slash names as one segment and explicitly assigns only the selected direct TagRef without receiving source rights or hidden body');
 await tagReady();await page.getByLabel('新建并赋值标签名称',{exact:true}).fill('UI_CANCELLED_CHILD');await page.getByLabel('首次赋值新标签的父级',{exact:true}).selectOption(parentRef);
 await picker.selectOption(parentRef);const beforeUnsubmitted=tagPrepares;await click('预览新建并赋值');await page.locator('#fields').getByText(/字段还有未保存输入/).waitFor();assert.equal(tagPrepares,beforeUnsubmitted);assert.deepEqual(tagRefs((await tagProfile()).field.cell.value),[otherRef]);await picker.selectOption(otherRef);
 const cancelPrepare=page.waitForResponse(r=>r.url().endsWith('/api/hosted/navigation/registry/prepare'));await click('预览新建并赋值');const preparedForCancel=await cancelPrepare;assert.equal(preparedForCancel.status(),200,await preparedForCancel.text());const cancelProposal=preparedForCancel.request().postDataJSON();await waitTagAction('明确提交原新建与赋值计划');assert.deepEqual(Object.keys(cancelProposal.intent.assignTo).sort(),['basis','fieldId','resource']);assert.equal(cancelProposal.intent.assignTo.resource,'shared');assert.equal(cancelProposal.intent.assignTo.fieldId,'core/tags');assert.equal(cancelProposal.intent.parentRef,parentRef);assert.equal(typeof cancelProposal.intent.assignTo.basis,'string');
 const cancelResponse=page.waitForResponse(r=>r.url().endsWith('/api/hosted/navigation/registry/settle'));await click('取消原未提交新建与赋值');const cancelled=await (await cancelResponse).json();assert.equal(cancelled.operationId,cancelProposal.operationId);assert.equal(cancelled.status,'cancelled');await waitTagAction('结束已核对记录，继续字段编辑');
 assert.deepEqual(tagRefs((await tagProfile()).field.cell.value),[otherRef]);assert.ok(!(await registry()).tags.items.some(tag=>tag.displayName==='UI_CANCELLED_CHILD'));assert.equal((await api('/api/hosted/navigation/registry/prepare',cancelProposal,'tagger')).status,'cancelled');await click('结束已核对记录，继续字段编辑');
 await passed('new-tag first assignment rejects unsaved field input, then actual same-plan cancellation preserves the source and durably fences delayed original preparation without creating a tag');
 const beforeAssignedSource=await readFile(notePath,'utf8');await page.getByLabel('新建并赋值标签名称',{exact:true}).fill('UI_CHILD / 叶😀');let assignedRequest,assignedPrepared,assignedCommit,prepareLostResolve;const prepareWasLost=new Promise(resolve=>prepareLostResolve=resolve);
 await page.route('**/api/hosted/navigation/registry/prepare',async route=>{try{assignedRequest=route.request().postDataJSON();const reply=await route.fetch();assignedPrepared=await reply.json();assert.equal(reply.status(),200,JSON.stringify(assignedPrepared));await route.abort('failed');prepareLostResolve();}catch(error){routeErrors.push(String(error));prepareLostResolve();await route.abort('failed').catch(()=>{});}});
 await click('预览新建并赋值');await bounded(prepareWasLost,'actual tag preparation');assert.deepEqual(routeErrors,[]);await waitTagAction('核对原标签新建与赋值操作');await page.unroute('**/api/hosted/navigation/registry/prepare');const afterOriginalPrepare=tagPrepares;assert.equal((await cachedTagJob()).request.operationId,assignedRequest.operationId);assert.ok(!JSON.stringify(assignedPrepared).includes('HIDDEN_BODY_987'));assert.ok(!(await registry()).tags.items.some(tag=>tag.reference===assignedPrepared.navigationRef));
 await page.reload();await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · core/tags'});await readField();await waitTagAction('核对原标签新建与赋值操作');
 const recoveredPrepare=page.waitForResponse(r=>r.url().endsWith('/api/hosted/navigation/registry/operation'));await click('核对原标签新建与赋值操作');const knownPrepare=await (await recoveredPrepare).json();assert.equal(knownPrepare.operationId,assignedRequest.operationId);assert.equal(knownPrepare.status,'prepared');assert.equal(knownPrepare.planId,assignedPrepared.planId);assert.equal(tagPrepares,afterOriginalPrepare);await waitTagAction('明确提交原新建与赋值计划');
 let commitLostResolve;const commitWasLost=new Promise(resolve=>commitLostResolve=resolve);await page.route('**/api/hosted/navigation/registry/commit',async route=>{try{const submitted=route.request().postDataJSON();assert.equal(submitted.operationId,assignedRequest.operationId);assert.equal(submitted.planId,assignedPrepared.planId);const reply=await route.fetch();assignedCommit=await reply.json();assert.equal(reply.status(),200,JSON.stringify(assignedCommit));assert.equal(assignedCommit.status,'committed');await route.abort('failed');commitLostResolve();}catch(error){routeErrors.push(String(error));commitLostResolve();await route.abort('failed').catch(()=>{});}});
 await click('明确提交原新建与赋值计划');await bounded(commitWasLost,'actual tag commit');assert.deepEqual(routeErrors,[]);await waitTagAction('核对原标签新建与赋值操作');await page.unroute('**/api/hosted/navigation/registry/commit');assert.equal(assignedCommit.operationId,assignedRequest.operationId);assert.equal(assignedCommit.navigationRef,assignedPrepared.navigationRef);assert.ok(!JSON.stringify(assignedCommit).includes('HIDDEN_BODY_987'));
 const committedCount=tagCommits;config.bind=new URL(base).host;await writeFile(configPath,JSON.stringify(config,null,2));await stop(server);server=await start(['--no-browser','--data-dir',data,'--server-config',configPath],true);await page.reload();await page.getByLabel('选择获权字段').waitFor();await page.getByLabel('选择获权字段').selectOption({label:'shared · core/tags'});await readField();await waitTagAction('核对原标签新建与赋值操作');
 const recoveredCommit=page.waitForResponse(r=>r.url().endsWith('/api/hosted/navigation/registry/operation'));await click('核对原标签新建与赋值操作');const knownCommit=await (await recoveredCommit).json();assert.equal(knownCommit.operationId,assignedRequest.operationId);assert.equal(knownCommit.status,'committed');assert.equal(knownCommit.planId,assignedPrepared.planId);assert.equal(knownCommit.navigationRef,assignedPrepared.navigationRef);await waitTagAction('结束已核对记录，继续字段编辑');assert.equal(tagCommits,committedCount);assert.equal(tagPrepares,afterOriginalPrepare);
 const actualTags=(await tagProfile()).field.cell.value;assert.deepEqual(tagRefs(actualTags),[otherRef,knownCommit.navigationRef]);assert.ok(JSON.stringify(actualTags).includes('literal / 保留'));assert.ok(!tagRefs(actualTags).includes(parentRef));const newTag=(await registry()).tags.items.find(tag=>tag.reference===knownCommit.navigationRef);assert.equal(newTag.parentTagId,parentRef.split('/').at(-1));const assignedSource=await readFile(notePath,'utf8');assert.equal(maskFixtureTags(assignedSource),maskFixtureTags(beforeAssignedSource));assert.ok(!(await page.locator('body').innerText()).includes('HIDDEN_BODY_987'));assert.equal(await page.locator('#source .cm-content').count(),0);
 await page.screenshot({path:path.join(out,'tag-first-assignment-recovered.png'),fullPage:true});await click('结束已核对记录，继续字段编辑');assert.equal(await page.getByRole('button',{name:'保存字段',exact:true}).isDisabled(),true);await click('已核对，采用当前字段依据');
 result.tagOperations={prepareCount:tagPrepares,commitCount:tagCommits,cancelCount:tagCancels,lookupCount:tagLookups,operationId:assignedRequest.operationId,planId:assignedPrepared.planId,reference:knownCommit.navigationRef};
 await passed('narrow tag create plus first assignment survives actual lost prepare and commit replies, browser reload and server restart under one original operation/Plan; leaf-only append preserves literal tags and every other source byte without source permission');
 await tagContext.close();page=alicePage;
 await page.getByLabel('选择获权字段').selectOption({label:'shared · core/aliases'});await readField();
 let release, observed;const held=new Promise(r=>observed=r), resume=new Promise(r=>release=r);
 await page.route('**/api/hosted/fields/read',async route=>{const response=await route.fetch();assert.equal(response.status(),200);observed();await resume;await route.fulfill({response});});
 await click('重新读取字段');await held;await changePolicy(users=>users.alice.grants={});
 await page.locator('#status').filter({hasText:'缓存已清除'}).waitFor({timeout:12000});release();await pause(400);
 assert.equal(await page.locator('#fields').isHidden(),true);assert.equal(await page.getByLabel('字段输入 第 1 项',{exact:true}).count(),0);
 assert.deepEqual(await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-field:'))),[]);
 assert.equal((await api('/api/hosted/fields/read',{resource:'shared',fieldId:'tasks/status'},'bob')).field.cell.value.value,'cancelled');
 result.cases.push('revocation clears field drafts/results and held late delivery; unrelated actor remains authorized');
 assert.deepEqual(result.pageErrors,[]);assert.deepEqual(routeErrors,[]);result.completed=true;
}catch(e){result.failure=String(e.stack??e);if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});throw e;}
finally{await browser?.close();await stop(local);await stop(server);result.ownedProcessesClosed=(!local||local.exitCode!==null||local.signalCode!==null)&&(!server||server.exitCode!==null||server.signalCode!==null)&&(!browser||!browser.isConnected());await writeFile(path.join(out,'results.json'),JSON.stringify(result,null,2));await writeFile(path.join(out,'server.stderr.log'),stderr);console.log(JSON.stringify({out,cases:result.cases,completed:result.completed,pageErrors:result.pageErrors,routeErrors}));}
