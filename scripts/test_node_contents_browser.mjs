// Real headless DOM/component regression with controlled API boundaries.
// This does not establish full-server or real-system IME acceptance.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const require=createRequire(import.meta.url),{chromium}=require(process.env.WEFTEXT_PLAYWRIGHT_MODULE||'playwright');
const out=path.resolve(process.argv[2]);await mkdir(out,{recursive:false});
const repo=path.resolve(import.meta.dirname,'..'),{createServer}=await import(pathToFileURL(path.join(repo,'app/node_modules/vite/dist/node/index.js')).href);
const vite=await createServer({root:path.join(repo,'app'),configFile:false,server:{host:'127.0.0.1',port:0,hmr:false},plugins:[{name:'component-document',configureServer(server){server.middlewares.use('/__node-contents-test__',(_req,res)=>{res.setHeader('Content-Type','text/html');res.end('<!doctype html><meta charset="utf-8"><textarea aria-label="独立正文草稿">保留草稿</textarea><section aria-label="节点内容" style="height:650px;overflow:auto"></section>');});}}]});
const result={cases:[],pageErrors:[],completed:false,limits:['Current-source component in owned headless Chrome; controlled APIs, not full-server, OS IME, AT or touch acceptance.']};let browser,page;
const pass=message=>result.cases.push(message);
try{
  await vite.listen();browser=await chromium.launch({headless:true,...(process.env.WEFTEXT_BROWSER_EXECUTABLE?{executablePath:process.env.WEFTEXT_BROWSER_EXECUTABLE}:{channel:'chrome'})});result.browser=browser.version();page=await browser.newPage({viewport:{width:1100,height:1000}});page.on('pageerror',error=>result.pageErrors.push(String(error)));
  await page.goto('http://127.0.0.1:'+vite.httpServer.address().port+'/__node-contents-test__');
  await page.evaluate(async()=>{
    const {mountNodeContents}=await import('/src/node-contents.ts');await import('/src/everyday.css');
    const ids=['11111111-1111-4111-8111-111111111111','22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333','44444444-4444-4444-8444-444444444444'];
    const root='D:/Owned/Library',folder=root+'/Parent',ref=id=>'weftext-node://'+ids[0]+'/'+id,parentRef=ref(ids[1]),a=ref(ids[2]),b=ref(ids[3]);
    const entries=[{path:folder+'/A',parentPath:folder,relativePath:'Parent/A',name:'A',kind:'directory',identityStatus:'unmanaged'},{path:folder+'/B',parentPath:folder,relativePath:'Parent/B',name:'B',kind:'directory',identityStatus:'unmanaged'},...['A','B'].map((name,index)=>({path:folder+'/'+name+'/'+name+'.typ',parentPath:folder+'/'+name,relativePath:'Parent/'+name+'/'+name+'.typ',name:name+'.typ',kind:'node',identityStatus:'admitted',nodeId:ids[index+2],source:'Owned '+name+' body',excerpt:'摘要 '+name}))];
    const inventory={root,workspaceId:ids[0],status:'localMvp',entries,warnings:[],complete:true};
    const fixture=window.fixture={requests:[],reads:[],opened:[],browsed:[],menus:[],order:[b,a],operations:new Map(),operationOrders:new Map(),losePrepare:true,loseCommit:true,changed:0,notices:[],parentRef,a,b,root,folder,release:[],generation:0,previewSerial:0,orderReads:0,readFault:null,failChanged:false,failRemove:false,failSet:false,failSetAfterCommit:false,holdRead:false,heldReads:[],controller:undefined};
    const removeItem=Storage.prototype.removeItem;Storage.prototype.removeItem=function(key){if(fixture.failRemove&&key===`weftext-node-contents:${encodeURIComponent(parentRef)}:operation`)throw Error('Owned operation retirement storage failure');return removeItem.call(this,key);};
    const setItem=Storage.prototype.setItem;Storage.prototype.setItem=function(key,value){if(fixture.failSet&&key===`weftext-node-contents:${encodeURIComponent(parentRef)}:operation`)throw Error('Owned terminal record storage failure');return setItem.call(this,key,value);};
    const canvas=document.createElement('canvas');canvas.width=2;canvas.height=2;canvas.getContext('2d').fillRect(0,0,2,2);const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));fixture.blob=blob;
    const request=async(route,body)=>{
      fixture.requests.push({route,body:structuredClone(body)});
      if(route==='/api/navigation/capture')return{targetRef:parentRef,status:'managed'};
      if(route==='/api/navigation/order/read'){
        const read={parentRef,baseId:'basis:'+ ++fixture.orderReads,beforeSource:null,members:[{targetRef:a,path:folder+'/A',name:'A'},{targetRef:b,path:folder+'/B',name:'B'}],orderedRefs:[...fixture.order],complete:true};
        if(fixture.holdRead)await new Promise(resolve=>fixture.heldReads.push(resolve));
        const fault=fixture.readFault;fixture.readFault=null;
        if(fault==='network')throw Error('Owned fresh order read failure');
        if(fault==='incomplete')read.complete=false;
        if(fault==='parent')read.parentRef=ref('55555555-5555-4555-8555-555555555555');
        return read;
      }
      if(route==='/api/navigation/order/prepare'){
        const plan={operationId:body.operationId,planId:'plan:'+body.operationId,status:'prepared',changes:[{path:root+'/.weftext-settings.json',action:'create',bytes:450}]};fixture.operations.set(body.operationId,plan);fixture.operationOrders.set(body.operationId,[...body.children]);
        if(fixture.losePrepare){fixture.losePrepare=false;throw Error('Owned lost prepare reply');}return plan;
      }
      if(route==='/api/operations/status')return fixture.operations.get(body.operationId);
      if(route==='/api/operations/commit'){
        const plan=fixture.operations.get(body.operationId);if(plan.status!=='prepared')throw Error('Original plan is not prepared');fixture.order=[...fixture.operationOrders.get(body.operationId)];plan.status='committed';
        if(fixture.failSetAfterCommit){fixture.failSetAfterCommit=false;fixture.failSet=true;}
        if(fixture.loseCommit){fixture.loseCommit=false;throw Error('Owned lost commit reply');}return plan;
      }
      if(route==='/api/navigation/order/cancel'){const plan=fixture.operations.get(body.operationId);plan.status='resolved';return plan;}
      if(route==='/api/navigation/preview'){
        const generation=fixture.generation,serial=++fixture.previewSerial;await new Promise(resolve=>fixture.release.push({generation,serial,resolve}));
        return{state:'ready',sourceBasisKind:'observed',sourceBaseId:'row:'+generation,sourceRevision:null,previewBasis:'currentResourceAtRowSource',descriptor:{resourceId:'asset',resource:'weftext-resource://'+ids[0]+'/'+ids[2],nodeId:body.entityKey.split('/').at(-1),name:'pixel.png',revision:'asset-revision',readBasis:'resource:'+generation,bytes:blob.size,contentType:'image/png',metadata:{kind:'image',width:2,height:2},annotationReady:true,currentness:'accepted'}};
      }
      throw Error('Unexpected component route '+route);
    };
    fixture.inventory=inventory;
    window.mount=(currentFolder=folder)=>{fixture.controller?.dispose();fixture.generation++;fixture.controller=mountNodeContents(document.querySelector('section'),inventory,currentFolder,'',new Set(),{request,readBlob:async(route,body)=>{fixture.reads.push({route,body});return blob;},open:entry=>{fixture.opened.push(structuredClone(entry));},browse:path=>{fixture.browsed.push(path);},menu:(entries,folder)=>{fixture.menus.push({entries:structuredClone(entries),folder});},preview:()=>{},changed:async()=>{fixture.changed++;if(fixture.failChanged)throw Error('Owned library inventory refresh failure');},notify:message=>fixture.notices.push(message)});};window.mount();
  });
  const rows=page.locator('.node-content-row'),sort=page.getByLabel('节点内容个人排序',{exact:true}),dialog=page.getByRole('dialog',{name:'作者同级顺序变更',exact:true});
  await page.waitForFunction(()=>window.fixture.requests.some(item=>item.route==='/api/navigation/order/read'));
  assert.deepEqual(await rows.locator(':scope > button:first-of-type').allTextContents(),['A · 笔记','B · 笔记']);
  const nodeA=rows.filter({has:page.getByRole('button',{name:'A · 笔记',exact:true})}),nodeB=rows.filter({has:page.getByRole('button',{name:'B · 笔记',exact:true})});
  await nodeA.getByRole('button',{name:'A · 笔记',exact:true}).click();
  await page.waitForFunction(()=>window.fixture.opened.length===1);
  assert.equal(await page.evaluate(()=>window.fixture.opened[0].path),await page.evaluate(()=>window.fixture.folder+'/A/A.typ'));
  await nodeA.getByRole('button',{name:'A · 笔记',exact:true}).dblclick();
  await page.waitForFunction(()=>window.fixture.opened.length===2);
  await nodeB.getByRole('button',{name:'B · 笔记',exact:true}).focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>window.fixture.opened.length===3);
  assert.equal(await page.evaluate(()=>window.fixture.opened[2].nodeId),await page.evaluate(()=>window.fixture.b.split('/').at(-1)));
  await nodeA.getByRole('button',{name:'浏览内容',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.fixture.browsed),[await page.evaluate(()=>window.fixture.folder+'/A')]);
  assert.equal(await page.evaluate(()=>window.fixture.opened.length),3);
  await nodeA.getByRole('button',{name:'打开笔记',exact:true}).click();await page.waitForFunction(()=>window.fixture.opened.length===4);
  await nodeA.press('Shift+F10');
  assert.deepEqual(await page.evaluate(()=>window.fixture.menus.at(-1).entries.map(entry=>entry.path)),[await page.evaluate(()=>window.fixture.folder+'/A/A.typ')]);
  assert.equal(await page.evaluate(()=>window.fixture.menus.at(-1).folder),await page.evaluate(()=>window.fixture.folder+'/A'));
  await nodeB.getByRole('checkbox').check();await page.getByRole('button',{name:'所选项目操作',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.fixture.menus.at(-1).entries.map(entry=>entry.path)),await page.evaluate(()=>[window.fixture.folder+'/A/A.typ',window.fixture.folder+'/B/B.typ']));
  assert.equal(await page.evaluate(()=>window.fixture.requests.some(item=>item.route==='/api/navigation/order/prepare')),false);
  pass('Healthy child nodes open their exact admitted source by one click or keyboard, double-click opens once, browsing stays separate and all menus retain exact source identities.');
  await page.evaluate(()=>{const fixture=window.fixture;fixture.inventory.entries.push({path:fixture.folder+'/Conflict/Conflict.typ',parentPath:fixture.folder+'/Conflict',relativePath:'Parent/Conflict/Conflict.typ',name:'Conflict.typ',kind:'node',identityStatus:'bindingConflict'});window.mount(fixture.folder+'/Conflict');});
  await rows.filter({hasText:'Conflict.typ'}).waitFor();assert.equal(await rows.count(),1);assert.equal(await page.getByRole('button',{name:'浏览内容',exact:true}).count(),0);
  pass('A conflicting same-coordinate main source remains visible inside its own directory and receives no guessed healthy-node browse action.');
  await page.evaluate(()=>window.mount());await sort.selectOption('authored');assert.deepEqual(await rows.locator(':scope > button:first-of-type').allTextContents(),['B · 笔记','A · 笔记']);
  await page.getByRole('button',{name:'A 作者顺序上移',exact:true}).click();await dialog.getByRole('button',{name:'取消',exact:true}).click();assert.equal(await page.evaluate(()=>window.fixture.requests.filter(item=>item.route==='/api/navigation/order/prepare').length),0);pass('Personal sorting and canceled author proposal produce no author write.');
  await page.getByRole('button',{name:'A 作者顺序上移',exact:true}).click();await dialog.getByRole('button',{name:'准备此顺序变更',exact:true}).click();await page.waitForFunction(()=>window.fixture.notices.includes('Owned lost prepare reply'));await dialog.getByRole('button',{name:'取消',exact:true}).click();
  const original=await page.evaluate(()=>window.fixture.requests.find(item=>item.route==='/api/navigation/order/prepare').body.operationId);
  await page.evaluate(()=>window.mount());await page.getByRole('button',{name:'检查原作者顺序操作',exact:true}).click();await page.getByRole('button',{name:'查看原顺序变更并确认',exact:true}).click();
  await dialog.getByRole('button',{name:'确认提交原作者顺序',exact:true}).click();await page.waitForFunction(()=>window.fixture.notices.includes('Owned lost commit reply'));await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();await page.getByRole('button',{name:'检查原作者顺序操作',exact:true}).click();await page.waitForFunction(()=>window.fixture.changed===1);
  assert.equal(await page.evaluate(()=>window.fixture.requests.filter(item=>['/api/navigation/order/prepare','/api/operations/commit'].includes(item.route)).every(item=>item.body.operationId===window.fixture.requests.find(item=>item.route==='/api/navigation/order/prepare').body.operationId)),true);assert.ok(original);pass('Lost prepare/commit replies, component restart and original-operation inspection preserve one request and its plan.');
  const pending=()=>page.evaluate(()=>JSON.parse(localStorage.getItem(`weftext-node-contents:${encodeURIComponent(window.fixture.parentRef)}:operation`)||'null'));
  const awaitRetired=()=>page.waitForFunction(()=>!localStorage.getItem(`weftext-node-contents:${encodeURIComponent(window.fixture.parentRef)}:operation`)&&document.querySelectorAll('.node-sibling-actions button').length>0);
  await awaitRetired();assert.equal(await page.getByRole('button',{name:'结束已确认顺序操作',exact:true}).count(),0);assert.deepEqual(await rows.locator(':scope > button:first-of-type').allTextContents(),['A · 笔记','B · 笔记']);
  const prepareMove=async()=>{
    const name=await page.evaluate(()=>window.fixture.order[1]===window.fixture.a?'A':'B');
    await page.getByRole('button',{name:name+' 作者顺序上移',exact:true}).click();await dialog.getByRole('button',{name:'准备此顺序变更',exact:true}).click();await dialog.waitFor({state:'hidden'});
    const operation=await pending();assert.equal(operation.status,'prepared');
    await page.getByRole('button',{name:'查看原顺序变更并确认',exact:true}).click();
    return operation.request.operationId;
  };
  const commit=()=>dialog.getByRole('button',{name:'确认提交原作者顺序',exact:true}).click();
  const commitCount=id=>page.evaluate(id=>window.fixture.requests.filter(item=>item.route==='/api/operations/commit'&&item.body.operationId===id).length,id);
  let next=await prepareMove();await commit();await dialog.waitFor({state:'hidden'});await awaitRetired();
  assert.equal(await commitCount(next),1);assert.deepEqual(await rows.locator(':scope > button:first-of-type').allTextContents(),['B · 笔记','A · 笔记']);
  const latestPrepare=await page.evaluate(()=>window.fixture.requests.filter(item=>item.route==='/api/navigation/order/prepare').at(-1));assert.notEqual(latestPrepare.body.operationId,original);assert.notEqual(latestPrepare.body.baseId,'basis:1');
  pass('A recovered committed operation automatically retires only after a fresh exact order, allowing the next explicit prepare/confirm/commit without a finish click or remount.');
  next=await prepareMove();await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();await page.getByRole('button',{name:'取消原作者顺序计划',exact:true}).click();await awaitRetired();assert.equal(await commitCount(next),0);assert.equal(await page.evaluate(id=>window.fixture.operations.get(id).status,next),'resolved');
  pass('Prepared cancellation refreshes the real order and automatically retires the original plan without committing a replacement.');
  for(const fault of ['network','incomplete','parent']){
    next=await prepareMove();await page.evaluate(fault=>{window.fixture.readFault=fault;},fault);await commit();
    await page.waitForFunction(id=>JSON.parse(localStorage.getItem(`weftext-node-contents:${encodeURIComponent(window.fixture.parentRef)}:operation`)).status==='committed',next);
    await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();
    assert.equal((await pending()).request.operationId,next);assert.equal(await page.locator('.node-sibling-actions button').count(),0);assert.equal(await commitCount(next),1);
    await page.getByRole('button',{name:'检查原作者顺序操作',exact:true}).click();await awaitRetired();assert.equal(await commitCount(next),1);
  }
  pass('Network failure, incomplete membership and a wrong-parent refresh retain the committed original and block new sorting; inspecting that same operation recovers without another commit.');
  next=await prepareMove();await page.evaluate(()=>{window.fixture.failChanged=true;});await commit();await page.waitForFunction(()=>window.fixture.notices.includes('Owned library inventory refresh failure'));await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();assert.equal((await pending()).request.operationId,next);assert.equal(await page.locator('.node-sibling-actions button').count(),0);
  await page.evaluate(()=>{window.fixture.failChanged=false;window.mount();});await awaitRetired();assert.equal(await commitCount(next),1);assert.ok(await page.evaluate(id=>window.fixture.requests.some(item=>item.route==='/api/operations/status'&&item.body.operationId===id),next));
  pass('Committed-but-failed library refresh keeps the original terminal record; a remount verifies the Core outcome, refreshes and retires it without repeating the write.');
  next=await prepareMove();await page.evaluate(()=>{window.fixture.failRemove=true;});await commit();await page.waitForFunction(()=>window.fixture.notices.includes('Owned operation retirement storage failure'));await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();assert.equal((await pending()).request.operationId,next);assert.equal(await page.locator('.node-sibling-actions button').count(),0);
  await page.evaluate(()=>{window.fixture.failRemove=false;});await page.getByRole('button',{name:'检查原作者顺序操作',exact:true}).click();await awaitRetired();assert.equal(await commitCount(next),1);
  pass('A failed durable-record removal leaves both the original record and in-memory pending state intact until original-operation recovery succeeds.');
  next=await prepareMove();await page.evaluate(()=>{window.fixture.failSetAfterCommit=true;});await commit();await page.waitForFunction(()=>window.fixture.notices.includes('Owned terminal record storage failure'));await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();assert.equal((await pending()).request.operationId,next);assert.equal((await pending()).status,'unknown');assert.equal(await page.locator('.node-sibling-actions button').count(),0);assert.equal(await commitCount(next),1);
  await page.evaluate(()=>{window.fixture.failSet=false;window.mount();});await page.getByRole('button',{name:'检查原作者顺序操作',exact:true}).click();await awaitRetired();assert.equal(await commitCount(next),1);
  pass('A committed response whose terminal record cannot be stored preserves the durable unknown request across restart and recovers only by querying that same operation.');
  next=await prepareMove();await page.evaluate(()=>{window.fixture.holdRead=true;});await commit();await page.waitForFunction(()=>window.fixture.heldReads.length===1);await page.evaluate(()=>{window.fixture.holdRead=false;window.mount();});await awaitRetired();
  const replacement=await prepareMove();await dialog.getByRole('button',{name:'保留原计划，暂不提交',exact:true}).click();await page.evaluate(()=>{for(const release of window.fixture.heldReads.splice(0))release();});await page.waitForTimeout(100);
  assert.equal((await pending()).request.operationId,replacement);assert.notEqual(replacement,next);assert.equal((await pending()).status,'prepared');assert.equal(await commitCount(next),1);
  await page.getByRole('button',{name:'取消原作者顺序计划',exact:true}).click();await awaitRetired();
  pass('A retired owner’s late fresh-order response cannot erase a replacement owner’s new prepared operation or alter its original recovery identity.');
  await page.getByLabel('节点内容缩略图',{exact:true}).uncheck();const firstPreviewSerial=await page.evaluate(()=>window.fixture.previewSerial);await page.getByLabel('节点内容缩略图',{exact:true}).check();await page.waitForFunction(serial=>window.fixture.release.some(item=>item.generation===window.fixture.generation&&item.serial>serial),firstPreviewSerial);
  const editor=page.getByRole('textbox',{name:'独立正文草稿',exact:true});await editor.focus();await page.evaluate(()=>{for(const item of window.fixture.release.splice(0))item.resolve();});await rows.locator('img').first().waitFor();assert.equal(await editor.evaluate(element=>document.activeElement===element),true);assert.equal(await editor.inputValue(),'保留草稿');
  assert.equal(await page.evaluate(()=>window.fixture.reads.every(item=>item.body.resourceRevision==='asset-revision'&&item.body.revision.startsWith('observed-source:row:'))),true);pass('Late real image decoding preserves editor focus and uses the exact source/resource basis.');
  await page.getByLabel('节点内容缩略图',{exact:true}).uncheck();assert.equal(await rows.locator('img').count(),0);
  const reads=await page.evaluate(()=>window.fixture.reads.length),latePreviewSerial=await page.evaluate(()=>window.fixture.previewSerial);await page.getByLabel('节点内容缩略图',{exact:true}).check();await page.waitForFunction(serial=>window.fixture.release.some(item=>item.generation===window.fixture.generation&&item.serial>serial),latePreviewSerial);await page.getByLabel('节点内容缩略图',{exact:true}).uncheck();await page.evaluate(()=>{for(const item of window.fixture.release.splice(0))item.resolve();});await page.waitForTimeout(300);assert.equal(await rows.locator('img').count(),0);assert.equal(await page.evaluate(()=>window.fixture.reads.length),reads);pass('Turning previews off disposes pixels and suppresses late resource reads.');
  await page.evaluate(async()=>{
    const {HostedLibrary}=await import('/src/hosted-library.ts');window.fixture.controller.dispose();
    const host=document.createElement('section');host.id='hosted-order';document.body.append(host);const source='{"version":1}',hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(source)))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
    const fixture=window.fixture;fixture.hostedRequests=[];fixture.hostedPlans=new Map();
    fixture.library=new HostedLibrary(host,async(route,body)=>{
      fixture.hostedRequests.push({route,body:structuredClone(body)});
      if(route==='library')return{roles:[{role:'settings',title:{key:'error.api.hostedLibrary.settingsRoleTitle',params:{}},read:true,write:true}],styleUse:false,reorderNodes:true,orderParents:[{reference:fixture.parentRef,title:'Parent'}]};
      if(route==='library/read')return{role:'settings',basis:'settings-basis',source,installed:false};
      if(route==='library/order/read')return{parentRef:fixture.parentRef,settingsSourceHash:hash,members:[{targetRef:fixture.a,name:'A'},{targetRef:fixture.b,name:'B'}],orderedRefs:[fixture.a,fixture.b],complete:true};
      if(route==='library/prepare'){const plan={role:'settings',operationId:body.operationId,planId:'hosted-plan',status:'prepared',commitAllowed:true,preview:{beforeSource:source,afterSource:body.source,summary:{key:'error.api.hostedLibrary.configurationPlanSummary',params:{}}}};fixture.hostedPlans.set(body.operationId,plan);return plan;}
      if(route==='library/settle'){const previous=fixture.hostedPlans.get(body.operationId),plan={...previous,status:'resolved',planId:null,commitAllowed:false};fixture.hostedPlans.set(body.operationId,plan);return plan;}
      throw Error('Unexpected hosted component route '+route);
    },()=>{});await fixture.library.connect('owned-order-component','actor',1);
  });
  const hosted=page.locator('#hosted-order');await hosted.getByRole('button',{name:'读取托管库配置',exact:true}).click();await hosted.getByRole('button',{name:'读取完整作者子级',exact:true}).click();await hosted.getByRole('button',{name:'B 作者顺序上移',exact:true}).click();
  const proposed=JSON.parse(await hosted.getByLabel('托管库配置完整源码',{exact:true}).inputValue()),identity=await page.evaluate(()=>({parent:window.fixture.parentRef,a:window.fixture.a,b:window.fixture.b}));assert.deepEqual(proposed.navigationOrders[identity.parent],[identity.b,identity.a]);
  assert.equal(await page.evaluate(()=>window.fixture.hostedRequests.some(item=>item.route==='library/prepare')),false);
  await hosted.getByRole('button',{name:'准备库配置变更',exact:true}).click();assert.equal(await hosted.getByLabel('原计划提交前配置',{exact:true}).inputValue(),'{"version":1}');assert.deepEqual(JSON.parse(await hosted.getByLabel('原计划提交后配置',{exact:true}).inputValue()),proposed);
  await hosted.getByRole('button',{name:'结束原库配置操作并保留输入',exact:true}).click();assert.deepEqual(JSON.parse(await hosted.getByLabel('托管库配置完整源码',{exact:true}).inputValue()),proposed);assert.equal(await page.evaluate(()=>window.fixture.hostedRequests.some(item=>item.route==='library/commit')),false);pass('Hosted friendly controls retain exact parent/member references in the ordinary settings Draft; preparation previews and cancellation preserve input without body or hidden commit.');
  assert.deepEqual(result.pageErrors,[]);result.completed=true;await page.screenshot({path:path.join(out,'node-contents.png'),fullPage:true});
}catch(error){result.failure=String(error.stack||error);if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});throw error;}
finally{await browser?.close();await vite.close();await writeFile(path.join(out,'results.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({out,completed:result.completed,cases:result.cases.length,failure:result.failure}));}
