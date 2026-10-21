// Real owned browser tasks; no desktop input, real OS IME or reference-host claim.

import assert from 'node:assert/strict';

import { spawn, execFileSync } from 'node:child_process';

import { mkdir, readFile, writeFile, access } from 'node:fs/promises';

import { createHash, randomUUID, randomBytes } from 'node:crypto';

import { createRequire } from 'node:module';

import { pathToFileURL } from 'node:url';

import path from 'node:path';

const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');

const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]); await mkdir(out, { recursive: false });

const sha = bytes => createHash('sha256').update(bytes).digest('hex'), pause = ms => new Promise(resolve => setTimeout(resolve, ms));

const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), frontend: process.env.WEFTEXT_DEV_FRONTEND ? 'current Vite sources' : 'embedded candidate', cases: [], pageErrors: [], httpErrors: [], completed: false, limits: ['Owned headless Chrome. No OS IME, full AT, touch or external product task comparison.'] };

const data = path.join(out, 'state'), root = path.join(out, 'UIRoot'), tokens = Object.fromEntries(['alice','reader','admin'].map(actor => [actor, randomBytes(32).toString('hex')]));

let server, base, token, browser, page, vite, stderr = '', phase = 'local';

async function until(test) { let last; for (let i = 0; i < 450; i++) { try { if (await test()) return; } catch (error) { last = error; } await pause(100); } throw last ?? Error('Condition timed out'); }

async function start(args = [], hosted = false) { server = spawn(exe, ['--no-browser', '--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore','pipe','pipe'] }); let stdout = ''; server.stdout.on('data', bytes => stdout += bytes); server.stderr.on('data', bytes => stderr += bytes); await until(() => { const match = stdout.match(hosted ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/); if (match) { base = match[1]; token = match[2]; return true; } if (server.exitCode !== null) throw Error(stderr); return false; }); }

async function stop() { if (server && server.exitCode === null && server.signalCode === null) { const closed = new Promise(resolve => server.once('exit', resolve)); server.kill(); await closed; } server = undefined; }

async function api(route, body, actor, expected = 200) { const response = await fetch(base + route, { method: body === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + tokens[actor] } : { 'X-Weftext-Token': token }) }, body: body === undefined ? undefined : JSON.stringify(body) }); const value = await response.json(); assert.equal(response.status, expected, JSON.stringify(value)); return value; }

async function frontend(hosted = false) { if (!process.env.WEFTEXT_DEV_FRONTEND) return base; await vite?.close(); const app = path.resolve(import.meta.dirname, '../app'); const { createServer } = await import(pathToFileURL(path.join(app, 'node_modules/vite/dist/node/index.js')).href); const proxy = { target: base, changeOrigin: true, configure(instance) { instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', base); }); } }; vite = await createServer({ root: app, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] }, proxy: { '/api': proxy, '/fonts': proxy } } }); await vite.listen(); return 'http://127.0.0.1:' + vite.httpServer.address().port + (hosted ? '/hosted.html' : ''); }

const pendingObservations = new Set();

async function observe(target) {
  // Keep only references to the actual retained CodeMirror views. Reading an
  // off-screen view must not switch mode, edit source or create a new request.
  await target.addInitScript(() => {
    const views = new Map();
    const remember = () => {
      const sessionId = document.querySelector('.topbar')?.dataset.session;
      const content = document.querySelector('.source-host .cm-content');
      const view = content?.cmTile?.root?.view;
      if (sessionId && view?.contentDOM === content) views.set(sessionId, view);
    };
    new MutationObserver(remember).observe(document, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-session'] });
    window.weftextAcceptanceEditor = () => {
      remember();
      const sessionId = document.querySelector('.topbar')?.dataset.session ?? null;
      const view = views.get(sessionId);
      const active = document.activeElement;
      return { sessionId, source: view?.state.doc.toString() ?? null,
        viewMode: document.querySelector('.view-switch button[aria-pressed=true]')?.textContent ?? null,
        focusedInput: active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? { label: active.getAttribute('aria-label'), value: active.value, selectionStart: active.selectionStart, selectionEnd: active.selectionEnd } : null };
    };
  });
  const requests = new WeakMap(), basisRequests = [];
  let sequence = 0;
  target.on('request', request => {
    const item = { sequence: ++sequence, phase, route: new URL(request.url()).pathname };
    try {
      const body = request.postDataJSON();
      if (typeof body?.sessionId === 'string' && typeof body.source === 'string' && Number.isSafeInteger(body.serial)) {
        item.basis = { sessionId: body.sessionId, serial: body.serial, sourceHash: sha(body.source), sourceBytes: Buffer.byteLength(body.source) };
        basisRequests.push(item);
        if (item.route === '/api/data/project') item.requestBody = body;
      }
    } catch {}
    requests.set(request, item);
  });
  target.on('pageerror', error => result.pageErrors.push(String(error)));
  target.on('response', response => {
    if (!response.url().includes('/api/') || response.status() < 400) return;
    const captured = phase, responseSequence = ++sequence, original = requests.get(response.request());
    // A later basis must already have been sent when this response arrives.
    // An edit made afterwards cannot excuse a still-current refusal.
    const later = basisRequests.filter(item => item.sequence > (original?.sequence ?? Infinity) && item.sequence < responseSequence);
    const observation = (async () => {
      try {
        const route = new URL(response.url()).pathname;
        const error = { phase: captured, requestPhase: original?.phase, route, status: response.status(), body: await response.text() };
        if (route === '/api/data/project') {
          error.requestBody = original?.requestBody;
          error.requestBasis = original?.basis;
          error.requestSequence = original?.sequence;
          error.responseSequence = responseSequence;
          const editor = await target.evaluate(() => window.weftextAcceptanceEditor?.() ?? null);
          error.editorAtResponse = editor && { ...editor, source: undefined, sourceHash: editor.source === null ? null : sha(editor.source) };
          const newer = later.filter(item => item.basis?.sessionId === original?.basis?.sessionId && item.basis.serial > original.basis.serial && item.basis.sourceHash === error.editorAtResponse?.sourceHash).at(-1);
          error.supersededBasis = !!newer && editor?.sessionId === original?.basis?.sessionId ? newer : null;
        }
        result.httpErrors.push(error);
      } catch (error) { if (!target.isClosed()) (result.observerErrors ??= []).push(String(error)); }
    })();
    pendingObservations.add(observation);
    void observation.finally(() => pendingObservations.delete(observation));
  });
}

async function finishObservations() { await until(()=>pendingObservations.size===0); }

function supersededProjection(error, code) {
  const original = error.requestBasis, newer = error.supersededBasis, editor = error.editorAtResponse;
  return error.route === '/api/data/project' && error.status === 409 && code === 'stale_draft'
    && typeof original?.sessionId === 'string' && Number.isSafeInteger(original.serial)
    && newer?.sequence > error.requestSequence && newer.sequence < error.responseSequence
    && ['/api/render', '/api/editor-map', '/api/paged', '/api/knowledge/read', '/api/data/project', '/api/query/embedded/discover', '/api/query/embedded/run'].includes(newer.route)
    && newer.basis.sessionId === original.sessionId && newer.basis.serial > original.serial
    && editor?.sessionId === original.sessionId && typeof editor.sourceHash === 'string' && newer.basis.sourceHash === editor.sourceHash;
}

const click = (name, host = page) => host.getByRole('button', { name, exact: true }).click();

async function source(hosted = false) {

  const locator = page.locator(hosted ? '#source .cm-content' : '.source-host .cm-content'); let restore;

  if (!await locator.count()) { restore = hosted ? await page.getByRole('button',{name:/^V0[1-5] /}).evaluateAll(buttons=>buttons.find(button=>button.getAttribute('aria-pressed')==='true')?.textContent) : await page.locator('.view-switch button[aria-pressed=true]').textContent(); await click(hosted ? 'V01 源码' : '源码'); }

  const value = await locator.evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw Error('Actual CodeMirror editor unavailable'); return view.state.doc.toString(); });

  if (restore) { await click(restore); await until(async()=>await page.getByRole('button',{name:restore,exact:true}).getAttribute('aria-pressed')==='true'); } return value;

}

const expectSource = async (value, hosted = false) => until(async () => { assert.equal(await source(hosted), value); return true; });

const query = label => page.locator(`.embedded-query[data-query-label=${JSON.stringify(label)}]`);

async function more(label) { const details = query(label).locator('.embedded-query-more'); if (!await details.evaluate(element => element.open)) await details.locator('summary').click(); }

const pass = text => { result.cases.push(text); console.log('PASS', text); };

async function readOwned(file) { for (let i=0; i<20; i++) { try { return await readFile(file, 'utf8'); } catch (error) { if (!['EBUSY','EACCES'].includes(error.code) || i === 19) throw error; (result.fileReadRetries ??= []).push({ file, i, code:error.code }); await pause(100); } } }

async function saveAuthor(expected) {
  const responseWait=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/save'&&response.request().postDataJSON()?.source===expected);
  await page.keyboard.press('Control+s'); const response=await responseWait;
  assert.equal(response.status(),200,await response.text());
  // Observe disk only after the actual Save reply, so the test's read handle
  // cannot compete with the product's Windows commit protection.
  await until(async()=>await readOwned(rootPathForSave)===expected);
}
let rootPathForSave;
async function command(label) { await page.keyboard.press('Control+Shift+p'); const dialog = page.getByRole('dialog', { name: '命令', exact: true }); await dialog.getByRole('searchbox').fill(label); await dialog.getByRole('searchbox').press('Enter'); }

async function undo(expected, hosted = false) { await page.keyboard.press('Control+z'); await expectSource(expected, hosted); }

try {

  await start(); browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) }); result.browser = browser.version(); page = await browser.newPage({ viewport: { width:1440, height:1100 } }); await observe(page); await page.goto((await frontend()) + '/#token=' + token);

  await click('创建资料库'); let dialog = page.getByRole('dialog', { name:'创建资料库与文档排版语言', exact:true });

  await dialog.getByLabel('新资料库完整路径').fill(root); await dialog.getByLabel('根节点初始内容').selectOption('queries'); await dialog.getByLabel('根笔记标题（可留空）').fill('我的普通首页😀'); await dialog.getByLabel('初始查询：任务', {exact:true}).check(); await dialog.getByLabel('初始查询：事件', {exact:true}).check(); await click('预览创建', dialog);

  assert.match(await dialog.innerText(), /\.weftext-schema\.json/); await assert.rejects(access(root)); await click('关闭', dialog); await assert.rejects(access(root)); pass('Preset selection and actual scope/schema preview cancel without any author files or sample records.');

  const creations = []; let lose = true;

  await page.route('**/api/library/create', async route => { creations.push(route.request().postDataJSON()); if (lose) { lose=false; const reply=await route.fetch(); result.lostCreation=await reply.json(); await route.abort('failed'); } else await route.continue(); });

  await click('创建资料库'); dialog = page.getByRole('dialog', { name:'创建资料库与文档排版语言', exact:true }); assert.equal(await dialog.getByLabel('根笔记标题（可留空）').inputValue(),'我的普通首页😀'); await click('预览创建',dialog); await click('创建资料库',dialog); await until(()=>dialog.getByRole('button',{name:'确认同一创建操作',exact:true}).isEnabled()); assert.equal(await dialog.getByLabel('根节点初始内容').isDisabled(),true); await click('关闭',dialog);

  await click('创建资料库'); dialog=page.getByRole('dialog',{name:'创建资料库与文档排版语言',exact:true}); await click('确认同一创建操作',dialog); await until(()=>creations.length===2); await page.unroute('**/api/library/create'); assert.equal(creations.length,2); assert.deepEqual(creations[0],creations[1]);

  const rootPath=result.lostCreation.rootNotePath, rootDoc=await api('/api/open',{path:rootPath,root}), original=rootDoc.source; rootPathForSave=rootPath; await expectSource(original); await until(()=>page.locator('.embedded-query').count().then(count=>count===2)); pass('A lost creation reply reuses the exact original operation/options, opens the ordinary root note and creates each Query key once.');

  const taskDocs=[]; phase='fixture-seed';

  for (const [name,status,due] of [['任务甲','open','2026-10-19'],['任务乙','open','2026-10-20'],['任务丙','done',null]]) { const doc=await api('/api/note/create',{root,parent:root,name,operationId:randomUUID()}), date=due ? `, due: datetime(year: ${due.slice(0,4)}, month: ${Number(due.slice(5,7))}, day: ${Number(due.slice(8,10))})` : ''; const native=`#metadata((id: "${doc.nodeId}", types: ("task",), status: "${status}"${date})) <weftext>\n#set document(title: "${name}")\n#title()\n\n正文保留😀\n`; await api('/api/save',{sessionId:doc.sessionId,baseId:doc.baseId,source:native,operationId:randomUUID()}); taskDocs.push({...doc,source:native}); }

  const discovered=await api('/api/query/embedded/discover',{sessionId:rootDoc.sessionId,source:original,serial:0}), tasks=discovered.occurrences.find(item=>item.plan.types.includes('task')), events=discovered.occurrences.find(item=>item.plan.types.includes('event'));

  await click('刷新结果',query(tasks.label)); await until(()=>query(tasks.label).locator('tbody tr').count().then(count=>count===3)); await until(()=>query(events.label).innerText().then(text=>/完整空结果/.test(text)));

  phase='local'; const personal=query(tasks.label).locator('.embedded-query-view'); await personal.locator('summary').click(); await personal.getByLabel('此查询的个人视图').selectOption('board'); await until(()=>query(tasks.label).locator('.embedded-query-card').count().then(count=>count===3)); assert.equal(await query(tasks.label).locator('.embedded-query-lane').count(),2);

  await query(tasks.label).getByLabel('此查询的个人视图').selectOption('calendar'); await until(()=>query(tasks.label).locator('.embedded-query-lane').count().then(count=>count===3)); assert.equal(await query(tasks.label).locator('.embedded-query-card').count(),3); await expectSource(original); assert.equal(await readOwned(rootPath),original); pass('Independent Query results and personal board/calendar preserve all dated and undated rows, without changing source, data or the other Query.');

  await click('可视编辑'); await click('查询构建器',query(tasks.label)); let builder=query(tasks.label).locator('.embedded-query-builder'); await builder.getByLabel('筛选属性',{exact:true}).selectOption({label:'状态'}); await builder.getByLabel('筛选值',{exact:true}).selectOption('open'); await click('添加筛选条件',builder); await expectSource(original); await click('应用查询修改',builder);

  await until(async()=>{const value=await source(); return value!==original&&value.includes('open');}); const filtered=await source(); assert.ok(filtered.includes(events.range.raw)); assert.equal(await readOwned(rootPath),original); await until(()=>query(tasks.label).locator('.embedded-query-card').count().then(count=>count===2)); await undo(original); pass('Typed visible filtering is a single native Draft edit with exact one-step Undo and no implicit Save.');

  await click('查询构建器',query(tasks.label)); builder=query(tasks.label).locator('.embedded-query-builder'); await builder.getByText('分组与汇总（作者查询）',{exact:true}).click(); await builder.getByLabel('汇总分组属性').selectOption({label:'状态'}); await click('采用分组汇总',builder); await click('应用查询修改',builder); await until(async()=> (await source()).includes('aggregates')); await until(()=>query(tasks.label).innerText().then(text=>text.includes('count'))); await undo(original); pass('Author grouping/count uses the same Query executor and Draft Undo; aggregate presentation never becomes a field writer.');

  const first=discovered.occurrences[0], last=discovered.occurrences[1], separator=original.slice(first.range.utf16To,last.range.utf16From), swapped=original.slice(0,first.range.utf16From)+last.range.raw+separator+first.range.raw+original.slice(last.range.utf16To);

  await more(first.label); await click('下移此查询',query(first.label)); await until(()=>page.evaluate(label=>document.activeElement?.closest('.embedded-query')?.getAttribute('data-query-label')===label,first.label)); await expectSource(swapped); await undo(original);

  await more(first.label); await click('移除此处 Query',query(first.label)); dialog=page.getByRole('dialog',{name:'移除此处 Query',exact:true}); await click('取消',dialog); await expectSource(original); await more(first.label); await click('移除此处 Query',query(first.label)); dialog=page.getByRole('dialog',{name:'移除此处 Query',exact:true}); await click('确认移除此处',dialog); const removed=original.slice(0,first.range.utf16From)+original.slice(first.range.utf16To); await expectSource(removed); await undo(original); pass('Keyboard-equivalent reorder and confirmed removal preserve exact neighbouring source, cancel safely and each use one Undo.');

  await until(()=>query(tasks.label).getByRole('button',{name:'任务甲',exact:true}).count().then(count=>count===1)); await click('任务甲',query(tasks.label)); await expectSource(taskDocs[0].source); await page.waitForFunction(()=>document.querySelector('.status-right')?.textContent.includes('草稿预览已更新')); await click('可视编辑'); await until(async()=>await page.getByRole('button',{name:'可视编辑',exact:true}).getAttribute('aria-pressed')==='true'); await page.waitForFunction(()=>document.querySelector('.status-right')?.textContent.includes('草稿预览已更新')); await click('属性'); const properties=page.getByRole('complementary',{name:'笔记属性',exact:true}), due=properties.locator('.data-property').filter({has:page.getByRole('heading',{name:'截止日期',exact:true})});

  await due.getByLabel('截止日期',{exact:true}).fill('2026-'); phase='property-refusal'; await click('应用到草稿',due); await expectSource(taskDocs[0].source); await click('刷新',properties); await until(()=>due.getByLabel('截止日期',{exact:true}).inputValue().then(value=>value==='2026-')); assert.equal(await readOwned(taskDocs[0].path),taskDocs[0].source);

  phase='local';
  let releaseOldProjection, oldBasis;
  const projectionResume = new Promise(resolve => releaseOldProjection = resolve);
  const holdProjection = async route => {
    const body = route.request().postDataJSON();
    if (!oldBasis && body.source === taskDocs[0].source) { oldBasis = body; await projectionResume; }
    await route.continue();
  };
  await page.route('**/api/data/project', holdProjection);
  try {
    await click('刷新',properties); await until(()=>!!oldBasis);
    assert.equal(await due.getByLabel('截止日期',{exact:true}).inputValue(),'2026-');
    const newerProjection = page.waitForResponse(response => {
      if (new URL(response.url()).pathname !== '/api/render' || response.status() !== 200) return false;
      const body = response.request().postDataJSON();
      return body?.sessionId === oldBasis.sessionId && body.serial > oldBasis.serial && body.source !== oldBasis.source;
    }, { timeout:45000 }).then(response=>({response}),error=>({error}));
    await due.getByLabel('截止日期',{exact:true}).fill('2026-10-21'); await click('应用到草稿',due); await until(async()=> (await source())!==taskDocs[0].source);
    const corrected = await source(), newer = await newerProjection; if(newer.error) throw newer.error;
    const oldReply = page.waitForResponse(response => new URL(response.url()).pathname === '/api/data/project' && response.request().postDataJSON()?.sessionId === oldBasis.sessionId && response.request().postDataJSON()?.serial === oldBasis.serial, { timeout:45000 });
    releaseOldProjection(); const refusal = await oldReply; assert.equal(refusal.status(),409); assert.equal((await refusal.json()).error.code,'stale_draft');
    await finishObservations(); await expectSource(corrected);
    await until(async()=>await due.getByLabel('截止日期',{exact:true}).inputValue()==='2026-10-21');
    const superseded = result.httpErrors.find(error => error.requestBasis?.sessionId === oldBasis.sessionId && error.requestBasis.serial === oldBasis.serial && supersededProjection(error,'stale_draft'));
    assert.ok(superseded);
    assert.equal(supersededProjection({...superseded,supersededBasis:null},'stale_draft'),false);
    assert.equal(supersededProjection({...superseded,editorAtResponse:{...superseded.editorAtResponse,sourceHash:superseded.requestBasis.sourceHash}},'stale_draft'),false);
    assert.equal(supersededProjection({...superseded,supersededBasis:{...superseded.supersededBasis,basis:{...superseded.supersededBasis.basis,serial:oldBasis.serial}}},'stale_draft'),false);
  } finally { releaseOldProjection(); await page.unroute('**/api/data/project', holdProjection); }
  assert.equal(await readOwned(taskDocs[0].path),taskDocs[0].source); await undo(taskDocs[0].source); await click('返回',page.getByRole('navigation',{name:'日常导航与命令'})); await until(()=>page.locator('.embedded-query').count().then(count=>count===2)); await expectSource(original); pass('Typed property rejection and explicit refresh retain unfinished input; a corrected property shares the actual note Draft/Undo, survives an exact superseded projection refusal and leaves disk and other notes unchanged.');

  await click('查询与视图'); const advanced=page.locator('.query-panel'), card=advanced.locator('.query-card').filter({has:page.getByRole('button',{name:'任务甲',exact:true})}); await until(()=>card.count().then(n=>n===1)); await card.getByText('就地编辑真实属性',{exact:true}).click(); const statusField=card.locator('.query-inline-fields section').filter({has:page.getByLabel('结果属性：状态',{exact:true})}); await statusField.getByLabel('结果属性：状态',{exact:true}).selectOption('done'); await click('应用此字段到笔记草稿',statusField); dialog=page.locator('dialog.data-confirm-dialog'); await click('取消',dialog); await expectSource(taskDocs[0].source); assert.equal(await readOwned(taskDocs[0].path),taskDocs[0].source);

  await click('应用此字段到笔记草稿',statusField); dialog=page.locator('dialog.data-confirm-dialog'); await click('应用到草稿',dialog); await until(async()=> (await source()).includes('status: "done"')); assert.equal(await readOwned(taskDocs[0].path),taskDocs[0].source); await undo(taskDocs[0].source); await click('返回',page.getByRole('navigation',{name:'日常导航与命令'})); await until(()=>page.locator('.embedded-query').count().then(n=>n===2)); await expectSource(original); await click('查询与视图'); pass('A typed direct result-field edit reopens its exact real owner, confirms independently, retains input on cancel and changes only that owner Draft with one Undo.');

  phase='call-edits'; await command('插入内嵌 Query'); dialog=page.getByRole('dialog',{name:'插入内嵌 Query',exact:true}); await dialog.getByLabel('查询名称',{exact:true}).fill('共享UI'); await dialog.getByLabel('查询来源').selectOption('call'); await dialog.getByLabel('定义归属笔记').selectOption(`weftext-node://${rootDoc.workspaceId}/${rootDoc.nodeId}`); await click('读取所选笔记的保存定义',dialog); await until(()=>dialog.getByLabel('复用的保存定义').locator('option').count().then(n=>n===2)); await dialog.getByLabel('复用的保存定义').selectOption(tasks.key); await dialog.getByLabel('复用定义版本方式').selectOption('pinned'); const pinnedRevision=await dialog.getByLabel('复用定义准确固定版本').inputValue(); assert.ok(pinnedRevision); await click('插入查询',dialog); await until(()=>query('weftext-query-共享UI').count().then(n=>n===1)); const withCall=await source(); assert.ok(withCall.includes(tasks.range.raw)&&withCall.includes(events.range.raw)); assert.equal(await readOwned(rootPath),original); await saveAuthor(withCall); await click('可视编辑'); const callQuery=query('weftext-query-共享UI'); await click('刷新结果',callQuery); await until(()=>callQuery.locator('tbody tr').count().then(n=>n===3)); await click('查询构建器',callQuery); builder=callQuery.locator('.embedded-query-builder'); await builder.getByLabel('调用定义版本方式').selectOption('current'); await click('应用查询修改',builder); await until(async()=> (await source())!==withCall); assert.equal(await readOwned(rootPath),withCall); await undo(withCall); await undo(original); await saveAuthor(original); pass('Actual shared-definition insertion selects the exact owner/key and pinned revision, normal Save admits the call, current/pinned editing affects only that call, and Undo restores both exact author definitions.');

  await click('阅读'); assert.equal(await page.locator('.embedded-query-more').count(),0); assert.equal(await page.locator('.embedded-query-builder').count(),0); await expectSource(original); await page.screenshot({path:path.join(out,'local-queries.png'),fullPage:true});

  await page.close(); await vite?.close(); vite=undefined; await stop();

  const files={Home:{path:path.basename(rootPath),title:'普通首页',editable:true},...Object.fromEntries(taskDocs.map((doc,i)=>['Task'+i,{path:path.relative(root,doc.path).replaceAll('\\','/'),title:'任务'+i,editable:true}]))};

  const grants=Object.fromEntries(Object.keys(files).map(resource=>[resource,{read:true,write:true,history:true,fields:{'tasks/status':{read:true,write:true},'tasks/due':{read:true,write:true}},query:{definitions:true,definitionHistory:true,products:{title:true,types:true}}}]));

  const readers=Object.fromEntries(Object.entries(grants).map(([resource,rights])=>[resource,{...rights,write:false,fields:{'tasks/status':{read:true,write:false},'tasks/due':{read:true,write:false}}}]));

  const config={deployment:'query-organization-owned',bind:'127.0.0.1:0',root,adminTokenSha256:sha(Buffer.from(tokens.admin)),files,users:{alice:{enabled:true,tokenSha256:sha(Buffer.from(tokens.alice)),library:{styleUse:true},grants},reader:{enabled:true,tokenSha256:sha(Buffer.from(tokens.reader)),library:{styleUse:true},grants:readers}}}; const configPath=path.join(out,'server.json'); await writeFile(configPath,JSON.stringify(config,null,2)); phase='hosted'; await start(['--server-config',configPath],true); page=await browser.newPage({viewport:{width:1440,height:1100}}); await observe(page); await page.goto(await frontend(true)); await page.getByLabel('访问凭据',{exact:true}).fill(tokens.alice); await click('连接'); await click('普通首页'); await until(()=>page.locator('.embedded-query').count().then(count=>count===2)); await click('V04 可视写作');

  await more(first.label); await click('下移此查询',query(first.label)); await expectSource(swapped,true); assert.equal(await readOwned(rootPath),original); await undo(original,true); pass('Hosted organization uses the same retained editor, exact author Draft and Undo under current independent body authorization.');

  const policy=await api('/api/hosted/admin/policy',undefined,'admin'), users=structuredClone(policy.users); users.alice.grants={}; phase='revoked'; await api('/api/hosted/admin/policy',{expectedGeneration:policy.generation,users},'admin');
  // Revocation polling may already have retired the authenticated workspace.
  // Wait for the original clearing facts instead of clicking its hidden toolbar.
  await until(()=>page.locator('#source .cm-content').count().then(count=>count===0)); assert.equal(await page.locator('.embedded-query-more').count(),0);
  if (await page.getByLabel('访问凭据',{exact:true}).isVisible()) { assert.equal(await page.locator('#workspace').isHidden(),true); result.revocationExitObserved=true; }
  else await click('退出并清除本页缓存');

  await page.getByLabel('访问凭据',{exact:true}).fill(tokens.reader); await click('连接'); await click('普通首页'); await until(()=>page.locator('.embedded-query').count().then(count=>count===2)); await click('V05 阅读'); await page.keyboard.press('Control+z'); await expectSource(original,true); assert.equal(await page.locator('.embedded-query-more').count(),0); assert.equal(await page.locator('.embedded-query-builder').count(),0); assert.equal(await page.getByRole('button',{name:/独立字段权限/}).count(),0); await page.screenshot({path:path.join(out,'hosted-reader.png'),fullPage:true}); pass('Revocation clears retained query editors/data, and a reader gains neither source arrangement nor independent result-field writers.');

  await finishObservations(); result.expectedHttpErrors=[]; result.unexpectedHttpErrors=[];

  for(const error of result.httpErrors){let code,message,detailsCode;try{const value=JSON.parse(error.body).error;code=value?.code;message=value?.message;detailsCode=value?.details?.code;}catch{}const expected=supersededProjection(error,code)||(error.route==='/api/query/embedded/run'&&error.status===409&&code==='busy'&&detailsCode==='busy'&&message?.endsWith('(os error 32)'))||(error.phase==='revoked'&&error.status===401&&code==='authentication'&&['/api/hosted/query/embedded/run','/api/hosted/editor/render'].includes(error.route))||(['hosted','revoked'].includes(error.phase)&&error.status===429&&code==='busy'&&['/api/hosted/query/embedded/run','/api/hosted/editor/render'].includes(error.route))||(error.route==='/api/knowledge/read'&&error.status===409&&code==='staleBase')||(error.phase==='fixture-seed'&&error.route==='/api/knowledge/read'&&error.status===409&&code==='citationCutChanged')||(error.status===409&&code==='stale_draft'&&['/api/render','/api/knowledge/read','/api/query/embedded/run','/api/hosted/editor/render','/api/hosted/query/embedded/run'].includes(error.route))||(error.status===404&&code==='unavailable'&&error.route==='/api/hosted/office/legacy-capabilities')||(error.phase==='revoked'&&[404,410].includes(error.status)&&['sessionExpired','unavailable'].includes(code))||((error.requestPhase==='property-refusal'||error.phase==='property-refusal')&&error.route==='/api/data/edit'&&error.status===400&&code==='invalid_data');(expected?result.expectedHttpErrors:result.unexpectedHttpErrors).push({...error,code});}

  assert.deepEqual(result.pageErrors,[]); assert.deepEqual(result.observerErrors??[],[]); assert.deepEqual(result.unexpectedHttpErrors,[]); result.completed=true;

} catch(error) { result.failure=String(error.stack??error); if(page&&!page.isClosed()) result.failureToasts=await page.locator('.toast-host').innerText().catch(()=>''); if(page&&!page.isClosed()){await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});await writeFile(path.join(out,'failure.html'),await page.content().catch(()=>''));} throw error; }

finally { await browser?.close(); await vite?.close(); await stop(); await writeFile(path.join(out,'results.json'),JSON.stringify(result,null,2)); await writeFile(path.join(out,'server.stderr.log'),stderr); console.log(JSON.stringify({out,completed:result.completed,cases:result.cases.length,failure:result.failure})); }
