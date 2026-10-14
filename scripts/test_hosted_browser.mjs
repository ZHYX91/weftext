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
let local,server,browser,page,base,localToken,config,stderr='';
const token={alice:randomBytes(32).toString('hex'),bob:randomBytes(32).toString('hex'),admin:randomBytes(32).toString('hex')};
async function start(args,hosted=false){
 const p=spawn(exe,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});let text='';p.stdout.on('data',b=>text+=b);p.stderr.on('data',b=>stderr+=b);
 for(let i=0;i<300;i++){const match=text.match(hosted?/(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/:/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);if(match){base=match[1];localToken=match[2];return p;}if(p.exitCode!==null)throw new Error(stderr);await pause(100);}p.kill();throw new Error('startup timeout');
}
async function stop(p){if(p && p.exitCode===null && p.signalCode===null){const done=new Promise(resolve=>p.once('exit',resolve));p.kill();await done;}}
async function api(route,data,actor){
 const r=await fetch(base+route,{method:data===undefined?'GET':'POST',headers:{'Content-Type':'application/json',...(actor?{Authorization:'Bearer '+token[actor]}:{'X-Weftext-Token':localToken})},body:data===undefined?undefined:JSON.stringify(data)});const body=await r.json();assert.ok(r.ok,JSON.stringify(body));return body;
}
async function login(p,actor){await p.goto(base);await p.getByLabel('访问凭据',{exact:true}).fill(token[actor]);await p.getByRole('button',{name:'连接',exact:true}).click();await p.getByRole('button',{name:'共同文档',exact:true}).waitFor();}
async function open(p){await p.getByRole('button',{name:'共同文档',exact:true}).click();await p.locator('#source .cm-content[contenteditable=true]').waitFor();await p.getByRole('button',{name:'预览',exact:true}).waitFor();}
async function fillSource(p, text){const e=p.locator('#source .cm-content');await e.click();await e.press('ControlOrMeta+a');await p.keyboard.insertText(text);}
async function sourceValue(p){return (await p.locator('#source .cm-line').allTextContents()).join('\n');}
async function save(p){const response=p.waitForResponse(r=>r.url().endsWith('/api/hosted/save'));await p.getByRole('button',{name:'保存',exact:true}).click();return (await response).status();}
try{
 const data=path.join(out,'state'), root=path.join(out,'library');
 local=await start(['--no-browser','--data-dir',data]);
 await api('/api/library/create',{path:root,operationId:randomUUID()});
 const note=await api('/api/note/create',{root,parent:root,name:'Shared',operationId:randomUUID()});
 const initial=note.source+'\n中文远端测试😀\n';
 await api('/api/save',{sessionId:note.sessionId,baseId:note.baseId,source:initial,operationId:randomUUID()});
 await stop(local);local=null;
 const rights={read:true,write:true,history:true,download:true};
 config={deployment:'browser-test',bind:'127.0.0.1:0',root,adminTokenSha256:sha(token.admin),files:{shared:{path:'Shared/Shared.typ',title:'共同文档',editable:true}},users:Object.fromEntries(['alice','bob'].map(id=>[id,{enabled:true,tokenSha256:sha(token[id]),grants:{shared:rights}}]))};
 const configPath=path.join(out,'server.json');await writeFile(configPath,JSON.stringify(config,null,2));
 server=await start(['--no-browser','--data-dir',data,'--server-config',configPath],true);
 browser=await chromium.launch({channel:'chrome',headless:true});result.browser=browser.version();
 const contextA=await browser.newContext({viewport:{width:1440,height:1000},acceptDownloads:true});
 const contextB=await browser.newContext({viewport:{width:1280,height:900}});
 page=await contextA.newPage();const bob=await contextB.newPage();
 for(const p of [page,bob]){p.on('pageerror',e=>result.pageErrors.push(String(e)));p.on('dialog',d=>d.accept());p.on('console',m=>{if(m.type()==='error')result.consoleErrors.push(m.text());});}
 await login(page,'alice');await login(bob,'bob');await open(page);await open(bob);
 await page.locator('#source [role=status]').filter({hasText:'预览对应当前草稿'}).waitFor({timeout:30000});
 await page.getByRole('button',{name:'V03 按需语法',exact:true}).click();
 await page.getByRole('button',{name:/展开源码：中文远端/}).first().click();
 await page.getByLabel('当前构造的 Typst 源码',{exact:true}).waitFor();
 await page.getByRole('button',{name:'收起语法',exact:true}).click();
 await page.getByRole('button',{name:'V04 可视写作',exact:true}).click();
 await page.screenshot({path:path.join(out,'visual-writing.png'),fullPage:true});
 await page.getByRole('textbox',{name:'编辑段落文字',exact:true}).last().fill('中文远端测试😀富编辑');
 await page.getByRole('button',{name:'V05 阅读',exact:true}).click();
 await page.locator('.hosted-flow').filter({hasText:'富编辑'}).waitFor();
 assert.equal(await page.locator('.literal-input').count(),0);
 await page.getByRole('button',{name:'V01 源码',exact:true}).click();
 assert.match(await sourceValue(page),/富编辑/);
 await page.getByRole('button',{name:'撤销',exact:true}).click();assert.equal(await sourceValue(page),initial);
 await page.getByRole('button',{name:'重做',exact:true}).click();assert.match(await sourceValue(page),/富编辑/);
 await page.getByRole('button',{name:'撤销',exact:true}).click();assert.equal(await sourceValue(page),initial);
 await page.getByRole('button',{name:'更新写作预览',exact:true}).click();
 await page.locator('#source [role=status]').filter({hasText:'预览对应当前草稿'}).waitFor({timeout:30000});
 await page.getByRole('button',{name:'V04 可视写作',exact:true}).click();
 await page.getByRole('button',{name:'链接',exact:true}).click();
 const linkDialog=page.getByRole('dialog',{name:'添加链接',exact:true});
 await linkDialog.getByLabel('目标地址',{exact:true}).fill('https://example.test/notes');
 await linkDialog.getByLabel('显示文字',{exact:true}).fill('测试链接');await linkDialog.getByRole('button',{name:'添加',exact:true}).click();await linkDialog.waitFor({state:'hidden'});
 await page.getByRole('button',{name:'V05 阅读',exact:true}).click();
 await page.getByRole('link',{name:'测试链接',exact:true}).waitFor();assert.equal(await page.getByRole('link',{name:'测试链接',exact:true}).getAttribute('href'),'https://example.test/notes');
 await page.getByRole('button',{name:'V01 源码',exact:true}).click();
 await page.getByRole('button',{name:'撤销',exact:true}).click();assert.equal(await sourceValue(page),initial);
 assert.equal((await readFile(path.join(root,'Shared','Shared.typ'))).toString(),initial);
 await page.screenshot({path:path.join(out,'five-view-undo.png'),fullPage:true});
 result.cases.push('five hosted views share native Chinese/non-BMP rich edits and source Undo/Redo; view changes do not save');

 await fillSource(page,initial+'甲保存。');assert.equal(await save(page),200);
 await fillSource(bob,initial+'乙冲突草稿。');assert.equal(await save(bob),409);
 await bob.getByRole('button',{name:'重新读取与核对',exact:true}).click();await bob.locator('#comparison:not([hidden])').waitFor();
 assert.match(await bob.locator('#current').innerText(),/甲保存/);assert.match(await sourceValue(bob),/乙冲突/);
 await bob.screenshot({path:path.join(out,'conflict.png'),fullPage:true});
 await fillSource(bob,initial+'甲保存。乙合并。');await bob.getByRole('button',{name:'已核对，以当前稿作为保存依据',exact:true}).click();
 await bob.getByRole('button',{name:'保存',exact:true}).waitFor({state:'visible'});assert.equal(await save(bob),200);
 result.cases.push('two browser users preserve both drafts and explicitly merge');

 await page.getByRole('button',{name:'重新读取与核对',exact:true}).click();await page.locator('#status').filter({hasText:'已读取当前稿'}).waitFor();
 await contextA.setOffline(true);await fillSource(page,initial+'甲离线输入。');
 assert.match(await sourceValue(page),/离线输入/);
 await contextA.setOffline(false);await page.getByRole('button',{name:'重新读取与核对',exact:true}).click();
 await page.locator('#status').filter({hasText:'已恢复个人草稿'}).waitFor();assert.match(await sourceValue(page),/离线输入/);
 assert.equal(await save(page),200);result.cases.push('disconnected typing retained and revalidated after reconnect');

 let finish;const received=new Promise(resolve=>finish=resolve);
 await page.route('**/api/hosted/save',async route=>{const response=await route.fetch();assert.equal(response.status(),200);finish();await route.abort('failed');});
 await fillSource(page,initial+'丢响应已提交。');await page.getByRole('button',{name:'保存',exact:true}).click();await received;
 await page.locator('#status').filter({hasText:'fetch'}).waitFor();
 await page.unroute('**/api/hosted/save');
 await page.reload();await page.getByRole('button',{name:'共同文档',exact:true}).waitFor();await open(page);
 await page.locator('#status').filter({hasText:'已读取当前稿'}).waitFor();assert.match(await sourceValue(page),/丢响应已提交/);
 assert.equal(await page.locator('#save').isEnabled(),true);result.cases.push('lost save response reconciled by original operation after page reload');

 const preview=page.waitForResponse(r=>r.url().endsWith('/api/hosted/preview') && r.status()!==429);await page.getByRole('button',{name:'预览',exact:true}).click();const previewResponse=await preview;assert.equal(previewResponse.status(),200);await writeFile(path.join(out,'preview.json'),await previewResponse.text());await page.locator('#render:not([hidden])').waitFor();
 await page.locator('#render').scrollIntoViewIfNeeded();
 await page.frameLocator('#render').locator('body').filter({hasText:'丢响应已提交'}).waitFor({timeout:5000});
 await writeFile(path.join(out,'preview-dom.json'),JSON.stringify(await page.frameLocator('#render').locator('body').evaluate(body=>({display:getComputedStyle(body).display,visibility:getComputedStyle(body).visibility,rect:body.getBoundingClientRect().toJSON()})),null,2));
 await fillSource(page,initial+'未保存预览。');assert.equal(await page.locator('#render').isHidden(),true);
 await page.getByRole('button',{name:'预览',exact:true}).click();
 await page.frameLocator('#render').locator('body').filter({hasText:'未保存预览'}).waitFor({timeout:5000});
 await page.getByRole('button',{name:'保存历史',exact:true}).click();await page.locator('#history-list [data-version-id]').first().waitFor();assert.ok(await page.locator('#history-list [data-version-id]').count()>1);
 await page.locator('#history-list [data-version-id]').first().click();await page.getByLabel('所选准确旧版 · 只读源码',{exact:true}).filter({hasText:'#metadata'}).waitFor();
 const downloading=page.waitForEvent('download');await page.getByRole('button',{name:'下载已保存源码',exact:true}).click();const download=await downloading;await download.saveAs(path.join(out,'download.typ'));
 assert.equal((await readFile(path.join(out,'download.typ'))).toString(),initial+'丢响应已提交。');
 await page.locator('#render').scrollIntoViewIfNeeded();
 await page.locator('#render').screenshot({path:path.join(out,'visible-preview.png')});
 await page.screenshot({path:path.join(out,'hosted-editor.png'),fullPage:true});result.cases.push('actual preview, exact saved-source download and authorized history');

 let releaseOpen, observedOpen;
 const openHeld=new Promise(resolve=>observedOpen=resolve);
 const resumeOpen=new Promise(resolve=>releaseOpen=resolve);
 await bob.route('**/api/hosted/open',async route=>{observedOpen();await resumeOpen;await route.continue();});
 await bob.getByRole('button',{name:'重新读取与核对',exact:true}).click();await openHeld;
 assert.equal(await bob.locator('#source .cm-content').getAttribute('contenteditable'),'false');
 releaseOpen();await bob.locator('#source .cm-content[contenteditable=true]').waitFor();await bob.unroute('**/api/hosted/open');
 await fillSource(bob,initial+'乙未提交草稿仍保留');
 result.cases.push('reopening fences input until the new source and retained draft are reconciled');
 await page.getByRole('button',{name:'更新写作预览',exact:true}).click();
 await page.locator('#source [role=status]').filter({hasText:'预览对应当前草稿'}).waitFor({timeout:30000});
 await page.getByRole('button',{name:'V04 可视写作',exact:true}).click();
 let releaseEdit, observedEdit;
 const editHeld=new Promise(resolve=>observedEdit=resolve), resumeEdit=new Promise(resolve=>releaseEdit=resolve);
 await page.route('**/api/hosted/editor/edit',async route=>{const response=await route.fetch();assert.equal(response.status(),200);observedEdit();await resumeEdit;await route.fulfill({response});});
 await page.getByRole('textbox',{name:'编辑段落文字',exact:true}).last().fill('迟到富编辑不能恢复撤销的授权');await editHeld;
 const policy=await api('/api/hosted/admin/policy',undefined,'admin');policy.users.alice.grants={};
 await api('/api/hosted/admin/policy',{expectedGeneration:policy.generation,users:policy.users},'admin');
 await page.locator('#status').filter({hasText:'缓存已清除'}).waitFor({timeout:12000});assert.equal(await sourceValue(page),'');releaseEdit();await pause(500);assert.equal(await sourceValue(page),'');await page.unroute('**/api/hosted/editor/edit');
 assert.equal(await page.locator('#render').isHidden(),true);assert.equal(await page.locator('#history-list').isHidden(),true);assert.equal(await page.locator('#stable-link').isHidden(),true);assert.match(await sourceValue(bob),/仍保留/);
 const cached=await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-draft:')));assert.deepEqual(cached,[]);
 result.cases.push('revoked actor display/preview/cache cleared, late rich edit cannot restore the old draft, and other actor draft survives');
 assert.deepEqual(result.pageErrors,[]);result.completed=true;
}catch(e){result.failure=String(e.stack??e);if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});throw e;}
finally{await browser?.close();await stop(local);await stop(server);await writeFile(path.join(out,'results.json'),JSON.stringify(result,null,2));await writeFile(path.join(out,'server.stderr.log'),stderr);console.log(JSON.stringify({out,cases:result.cases,completed:result.completed,pageErrors:result.pageErrors}));}
