import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Optional real-browser acceptance. Supply a qualified Playwright installation
// through WEFTEXT_PLAYWRIGHT_MODULE, plus EXE and a new owned evidence directory.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
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
const token={alice:randomBytes(32).toString('hex'),bob:randomBytes(32).toString('hex'),writer:randomBytes(32).toString('hex'),admin:randomBytes(32).toString('hex')};
async function start(args,hosted=false){
 const p=spawnServe(exe,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});let text='';p.stdout.on('data',b=>text+=b);p.stderr.on('data',b=>stderr+=b);
 for(let i=0;i<300;i++){const match=serveReady(p);if(match){base=match.base;localToken=match.token;return p;}if(p.exitCode!==null)throw new Error(stderr);await pause(100);}await stopServe(p);throw new Error('startup timeout');
}
async function stop(p){ await stopServe(p); }
async function api(route,data,actor){
 const r=await fetch(base+route,{method:data===undefined?'GET':'POST',headers:{'Content-Type':'application/json',...(actor?{Authorization:'Bearer '+token[actor]}:{'X-Weftext-Token':localToken})},body:data===undefined?undefined:JSON.stringify(data)});const body=await r.json();assert.ok(r.ok,JSON.stringify(body));return body;
}
async function click(name){await page.getByRole('button',{name,exact:true}).click();}

async function changePolicy(edit){const p=await api('/api/hosted/admin/policy',undefined,'admin');edit(p.users);return api('/api/hosted/admin/policy',{expectedGeneration:p.generation,users:p.users},'admin');}
async function readReview(p=page, label='重新读取批注'){const r=p.waitForResponse(r=>r.url().endsWith('/api/hosted/review/read'));await p.getByRole('button',{name:label,exact:true}).click();assert.equal((await r).status(),200);await p.getByLabel('批注内容',{exact:true}).waitFor();}
async function commit(p=page){const r=p.waitForResponse(r=>r.url().endsWith('/api/hosted/review/commit'));await p.getByRole('button',{name:'确认提交批注操作',exact:true}).click();assert.equal((await r).status(),200);await p.locator('#review').getByText('原批注操作已提交。请重新读取批注；正文变更仍须与个人草稿核对。',{exact:true}).waitFor();}
try{
 const data=path.join(out,'state'),root=path.join(out,'library');local=await start(['--data-dir',data]);
 await api('/api/library/create',{path:root,operationId:randomUUID()});
 const note=createdDocument(await api('/api/note/create',{root,parent:root,name:'Shared',operationId:randomUUID()}));
 const initial=note.source+'\n这里有错别字，配对（未结束。\n批注范围😀';
 await api('/api/save',{sessionId:note.sessionId,baseId:note.baseId,source:initial,operationId:randomUUID()});await stop(local);local=null;
 config={deployment:'review-browser',bind:'127.0.0.1:0',root,adminTokenSha256:sha(token.admin),files:{shared:{path:'Shared/Shared.typ',title:'共同文档',editable:true}},users:{
  alice:{enabled:true,tokenSha256:sha(token.alice),grants:{shared:{read:true,write:true,annotations:{read:true,write:true,target:true,accept:true,proofread:true}}}},
  bob:{enabled:true,tokenSha256:sha(token.bob),grants:{shared:{annotations:{read:true,write:true}}}}}};
 const configPath=path.join(out,'server.json');await writeFile(configPath,JSON.stringify(config,null,2));server=await start(['--data-dir',data,'--server-config',configPath],true);
 browser=await chromium.launch({channel:'chrome',headless:true});result.browser=browser.version();
 const ctx=await browser.newContext({viewport:{width:1440,height:1100}}), ctxB=await browser.newContext({viewport:{width:1280,height:1000}});page=await ctx.newPage();const bob=await ctxB.newPage();
 for(const p of [page,bob]){p.on('pageerror',e=>result.pageErrors.push(String(e)));p.on('console',m=>{if(m.type()==='error')result.consoleErrors.push(m.text());});}
 async function login(p,actor){await p.goto(base);await p.getByLabel('访问凭据',{exact:true}).fill(token[actor]);await p.getByRole('button',{name:'连接',exact:true}).click();await p.getByLabel('选择批注对象').waitFor();}
 await login(page,'alice');await click('共同文档');await page.locator('#source .cm-content[contenteditable=true]').waitFor();await click('V01 源码');
 await readReview(page,'读取批注');
 const source=page.locator('#source .cm-content');await source.click();await source.press('ControlOrMeta+End');await source.press('Shift+Home');
 await page.getByLabel('批注类型',{exact:true}).selectOption('suggestion');await page.getByLabel('批注内容',{exact:true}).fill('请核对这一处😀');await page.getByLabel('建议替换文字',{exact:true}).fill('准确修改😀');
 await click('准备新批注');await page.getByRole('button',{name:'确认提交批注操作',exact:true}).waitFor();assert.equal(await readFile(path.join(root,'Shared/Shared.typ'),'utf8'),initial);
 await commit();await readReview();const first=page.locator('#review [data-annotation-id]').first();await first.getByText('请核对这一处😀',{exact:true}).waitFor();
 assert.match(await first.innerText(),/批注范围😀/);result.cases.push('exact Chinese/non-BMP source selection prepares and commits a real annotation without changing author source');
 await login(bob,'bob');await readReview(bob,'读取批注');assert.equal(await bob.locator('#documents button').count(),0);assert.ok(!(await bob.locator('#review').innerText()).includes('批注范围😀'));
 const reply=bob.locator('#review [data-annotation-id]').first();await reply.getByRole('textbox').fill('仅批注权限也能回复');await reply.getByRole('button',{name:'准备回复',exact:true}).click();await commit(bob);await readReview();assert.match(await first.innerText(),/仅批注权限也能回复/);
 result.cases.push('annotation-only user replies without source access or target excerpts; author sees actual reply');
 await first.getByRole('button',{name:'准备应用建议',exact:true}).click();await page.getByRole('button',{name:'确认提交批注操作',exact:true}).waitFor();assert.equal(await readFile(path.join(root,'Shared/Shared.typ'),'utf8'),initial);
 await commit();assert.equal(await readFile(path.join(root,'Shared/Shared.typ'),'utf8'),initial.replace('批注范围😀','准确修改😀'));
 assert.ok(await page.getByRole('button',{name:'保存',exact:true}).isDisabled());await click('重新读取与核对');await page.locator('#status').filter({hasText:'已读取当前稿'}).waitFor();await readReview();assert.match(await first.innerText(),/已接受/);
 await page.locator('#source [role=status]').filter({hasText:'预览对应当前草稿'}).waitFor({timeout:30000});
 await page.getByText(/词典与白名单/).click();await page.getByLabel('查找词语／符号',{exact:true}).fill('错别字');await page.getByLabel('建议替换为',{exact:true}).fill('正确字');await click('添加规则');await click('开始校对');
 await page.locator('#review [role=status]').filter({hasText:'发现'}).waitFor();await click('将所选结果加入批注');await commit();await readReview();assert.ok(await page.locator('#review [data-annotation-id]').count()>=3);
 result.cases.push('accepted suggestion commits source/state together; visible-text proofreading explicitly creates a batch of annotations');
 await page.screenshot({path:path.join(out,'review-proofread.png'),fullPage:true});
 await ctx.setOffline(true);await page.getByLabel('批注内容',{exact:true}).fill('离线讨论草稿😀');await ctx.setOffline(false);await page.reload();await page.getByLabel('选择批注对象').waitFor();await readReview(page,'读取批注');assert.equal(await page.getByLabel('批注内容',{exact:true}).inputValue(),'离线讨论草稿😀');
 result.cases.push('offline annotation input survives reload in actor-scoped tab recovery storage');
 const card=page.locator('#review [data-annotation-id]').first();await card.getByRole('textbox').fill('丢响应回复');await card.getByRole('button',{name:'准备回复',exact:true}).click();
 let pending;await page.route('**/api/hosted/review/commit',async route=>{pending=route.request().postDataJSON();const response=await route.fetch();assert.equal(response.status(),200);await route.abort('failed');});
 await click('确认提交批注操作');await page.waitForFunction(()=>Object.keys(sessionStorage).some(k=>k.startsWith('weftext-hosted-review:')&&JSON.parse(sessionStorage[k]).pending));await page.waitForTimeout(500);await page.unroute('**/api/hosted/review/commit');
 await page.reload();await page.getByLabel('选择批注对象').waitFor();await readReview(page,'读取批注');await click('查询原批注操作');await page.locator('#review').getByText('原批注操作已提交。请重新读取批注；正文变更仍须与个人草稿核对。',{exact:true}).waitFor();
 assert.equal((await api('/api/hosted/review/operation',{resource:'shared',operationId:pending.operationId},'alice')).status,'committed');await readReview();assert.equal((await page.locator('#review').innerText()).split('丢响应回复').length-1,1);
 result.cases.push('lost commit response and reload recover the original operation once, with no duplicate reply');
 let release,observed;const held=new Promise(r=>observed=r),resume=new Promise(r=>release=r);await page.route('**/api/hosted/review/read',async route=>{const response=await route.fetch();assert.equal(response.status(),200);observed();await resume;await route.fulfill({response});});
 await click('重新读取批注');await held;await changePolicy(users=>users.alice.grants.shared.annotations={});await page.locator('#status').filter({hasText:'缓存已清除'}).waitFor({timeout:12000});release();await pause(400);
 assert.equal(await page.locator('#review').isHidden(),true);assert.deepEqual(await page.evaluate(()=>Object.keys(sessionStorage).filter(k=>k.startsWith('weftext-hosted-review:'))),[]);
 assert.ok((await api('/api/hosted/review/read',{resource:'shared'},'bob')).annotations.length>=3);result.cases.push('revocation clears review drafts/results and rejects held late delivery while the other actor remains authorized');
 await page.unroute('**/api/hosted/review/read');
 let releasePoll, observedPoll, holdPoll=true;const heldPoll=new Promise(r=>observedPoll=r),resumePoll=new Promise(r=>releasePoll=r);
 await page.route('**/api/hosted/me',async route=>{if(!holdPoll){await route.continue();return;}holdPoll=false;const response=await route.fetch();assert.equal(response.status(),200);observedPoll();await resumePoll;await route.fulfill({response});});
 await changePolicy(users=>users.alice.grants.shared.history=true);await Promise.race([heldPoll,pause(12000).then(()=>{throw new Error('authorization poll was not observed');})]);
 await click('退出并清除本页缓存');await page.getByLabel('访问凭据',{exact:true}).fill(token.bob);await click('连接');await page.getByLabel('选择批注对象').waitFor();await readReview(page,'读取批注');
 const currentReply=page.locator('#review [data-annotation-id]').first().getByRole('textbox');await currentReply.fill('新用户当前未提交回复');releasePoll();await pause(500);
 assert.equal(await currentReply.inputValue(),'新用户当前未提交回复');assert.match(await page.locator('#status').innerText(),/bob/);
 assert.equal(await page.evaluate(()=>JSON.parse(sessionStorage.getItem('weftext-hosted-identity')).actor),'bob');
 result.cases.push('late authorization polling from a logged-out actor cannot clear the new actor draft or replace its identity');
 assert.deepEqual(result.pageErrors,[]);result.completed=true;
}catch(e){result.failure=String(e.stack??e);if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});throw e;}
finally{await browser?.close();await stop(local);await stop(server);await writeFile(path.join(out,'results.json'),stringifyServeJson(result, 2));await writeFile(path.join(out,'server.stderr.log'),stderr);console.log(JSON.stringify({out,cases:result.cases,completed:result.completed,pageErrors:result.pageErrors}));}
