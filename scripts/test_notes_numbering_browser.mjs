// Real local/hosted authoring UI over owned fixtures. No desktop input or Save.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const python = process.env.WEFTEXT_PYTHON; assert.ok(python && path.isAbsolute(python));
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]), repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const dev = process.argv.includes('--dev'), viteServers = [];
await mkdir(out, { recursive: false });
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], httpFailures: [], completed: false };
result.frontend = dev ? 'current Vite sources; component integration only' : 'embedded candidate';
result.resourceDiagnostics = process.env.WEFTEXT_TEST_RESOURCE_DIAGNOSTICS === '1';
const helper = path.join(out, 'owned-fixture.py');
await writeFile(helper, String.raw`import json,sys,uuid
from pathlib import Path
sys.path.insert(0,sys.argv[1]+"/scripts")
import test_hosted
from _creation_result import created_document
test_hosted.EXE=Path(sys.argv[2]);out=Path(sys.argv[3]);c=test_hosted.Hosted("runTest");c.evidence=out
body='\n= 一、重复标题 <alpha>\n\n正文#footnote[*中文脚注*] <memo>。再次#footnote(<memo>)。\n\n#figure(table(columns:2,[甲],[乙]),caption:[原始表题]) <tab>\n\n$ x+y $\n\n= 一、重复标题 <beta>\n'
c.setUp()
try:
 c.server.close();local=test_hosted.Server(['--no-browser','--data-dir',str(c.data)])
 try:
  code,doc,_=local.call('/api/open',{'path':str(c.root/'Shared/Shared.typ'),'root':str(c.root)});assert code==200,doc
  source=doc['source']+body;code,receipt,_=local.call('/api/save',{'sessionId':doc['sessionId'],'baseId':doc['baseId'],'source':source,'operationId':str(uuid.uuid4())});assert code==200,receipt
  owner={'workspaceId':doc['workspaceId'],'nodeId':doc['nodeId']}
 finally:local.close()
 c.config['files']['Shared']['inputs']=['notes-assets/*']
 c.config['resourceCollections']={'notes-assets':{'owner':'Shared','expectedOwner':owner,'title':'Notes assets'}}
 for actor in ['alice','bob','reader']:c.config['users'][actor]['resourceCollections']={'notes-assets':{'read':True,'download':True,'import':actor=='alice'}}
 c.config_path.write_text(json.dumps(c.config),'utf-8')
 c.start()
 def grant_collections(users):
  for actor in ['alice','bob','reader']:users[actor]['resourceCollections']={'notes-assets':{'read':True,'download':True,'import':actor=='alice'}}
 c.policy(grant_collections)
 c.server.close()
 localroot=out/'local-library';localdata=out/'local-state';local=test_hosted.Server(['--no-browser','--data-dir',str(localdata)])
 try:
  code,value,_=local.call('/api/library/create',{'path':str(localroot),'operationId':str(uuid.uuid4())});assert code==200,value
  code,ldoc,_=local.call('/api/note/create',{'root':str(localroot),'parent':str(localroot),'name':'Notes','operationId':str(uuid.uuid4())});assert code==200,ldoc
  ldoc=created_document(ldoc)
  local_source=ldoc['source']+body;code,value,_=local.call('/api/save',{'sessionId':ldoc['sessionId'],'baseId':ldoc['baseId'],'source':local_source,'operationId':str(uuid.uuid4())});assert code==200,value
 finally:local.close()
 (out/'fixture.json').write_text(json.dumps({'root':str(c.root),'data':str(c.data),'configPath':str(c.config_path),'tokens':c.tokens,'source':source,'localRoot':str(localroot),'localData':str(localdata),'localPath':ldoc['path'],'localSource':local_source},ensure_ascii=False),'utf-8')
finally:
 if c.server.process.poll() is None:c.server.close()
`, 'utf8');
let fixture, browser, local, hosted, stderr = '';
const stderrByScope = { local: '', hosted: '' }, stderrEvents = [], apiEvents = [], contexts = new Set();
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function savedSource(file) {
  for (let attempt = 0; ; attempt++) {
    try { return await readFile(file, 'utf8'); } catch (error) { if (error.code !== 'EBUSY' || attempt >= 39) throw error; await pause(50); }
  }
}
async function start(args, isHosted) {
  const child = spawn(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }), scope = isHosted ? 'hosted' : 'local'; let stdout = ''; child.stdout.on('data', value => stdout += value); child.stderr.on('data', value => { const text = String(value); stderr += text; stderrByScope[scope] += text; stderrEvents.push({ at: new Date().toISOString(), scope, pid: child.pid, text }); });
  for (let count = 0; count < 300; count++) { const match = stdout.match(isHosted ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/); if (match) return { child, base: match[1], token: match[2] }; if (child.exitCode !== null) throw new Error(stderr); await pause(100); }
  child.kill(); throw new Error('startup timeout');
}
async function stop(server) { if (server?.child.exitCode === null && server.child.signalCode === null) { const done = new Promise(resolve => server.child.once('exit', resolve)); server.child.kill(); await done; } }
async function observedPage(scope) {
  const context = await browser.newContext({ viewport: { width: 1480, height: 1080 }, recordHar: { path: path.join(out, `${scope}.har`), content: 'embed', mode: 'full', urlFilter: '**/api/**' } }); contexts.add(context); context.on('close', () => contexts.delete(context));
  const page = await context.newPage(); page.on('pageerror', error => result.pageErrors.push(error.message)); page.on('dialog', dialog => dialog.dismiss());
  page.on('response', async response => {
    if (!response.url().includes('/api/')) return;
    const request = response.request(), event = { at: new Date().toISOString(), scope, url: response.url(), method: request.method(), status: response.status(), timing: request.timing(), request: request.postDataJSON() }; apiEvents.push(event);
    if (response.status() >= 400) result.httpFailures.push({ ...event, body: await response.text().catch(() => null) });
  });
  return page;
}
async function frontendFor(backend, isHosted) {
  if (!dev) return backend.base;
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  const proxy = {target:backend.base, changeOrigin:true, configure(instance) { instance.on('proxyReq',(request,incoming)=>{ if(incoming.headers.origin) request.setHeader('Origin',backend.base); }); }};
  const server = await createServer({root:path.join(repo,'app'),configFile:false,server:{host:'127.0.0.1',port:0,hmr:false,watch:{ignored:['**/*']},proxy:{'/api':proxy,'/fonts':proxy}}});
  await server.listen(); viteServers.push(server);
  return `http://127.0.0.1:${server.httpServer.address().port}${isHosted ? '/hosted.html' : ''}`;
}
const click = (page, name) => page.getByRole('button', { name, exact: true }).click();
async function connectActor(page, actor) {
  await page.getByLabel('访问凭据', { exact: true }).fill(fixture.tokens[actor]); await click(page, '连接');
  await page.waitForFunction(() => !!document.querySelector('#documents button:not(:disabled)') && !document.querySelector('#workspace')?.hidden);
}
async function logoutDuringConnect(page) {
  await click(page, '退出并清除本页缓存');
  let release, arrived; const held = new Promise(resolve => { release = resolve; }), pending = new Promise(resolve => { arrived = resolve; }); let once = true;
  const pattern = '**/api/hosted/documents';
  const handler = async route => { if (once && route.request().headers().authorization === 'Bearer ' + fixture.tokens.bob) { once = false; arrived(); await held; } await route.continue(); };
  await page.route(pattern, handler);
  try {
    await page.getByLabel('访问凭据', { exact: true }).fill(fixture.tokens.bob); await click(page, '连接'); await pending;
    await click(page, '退出并清除本页缓存'); await connectActor(page, 'alice');
    const late = page.waitForResponse(response => response.url().endsWith('/api/hosted/documents') && response.request().headers().authorization === 'Bearer ' + fixture.tokens.bob); release(); await (await late).finished();
    assert.equal(await page.evaluate(() => JSON.parse(sessionStorage.getItem('weftext-hosted-identity')).actor), 'alice');
    assert.equal(await page.locator('#login').isVisible(), false); assert.equal(await page.locator('#status').innerText().then(value => value.includes('Cannot read')), false);
    await click(page, '取回未应用的注释输入'); assert.equal(await page.getByLabel('保留的本次新输入', { exact: true }).inputValue(), '用户新输入甲\n用户新输入乙');
  } finally { release(); await page.unroute(pattern, handler); }
}
async function waitNoteApplied(page) {
  await page.waitForFunction(() => { const dialog = document.querySelector('dialog.document-notes-dialog'); return !dialog || !!dialog.querySelector('[role="alert"]:not([hidden])'); });
  const error = page.locator('dialog.document-notes-dialog [role="alert"]:not([hidden])'); if (await error.count()) throw new Error(`Native note proposal rejected: ${await error.innerText()}`);
}
const rawSource = async (page, hosted = false) => (await page.locator(hosted ? '#source .cm-content .cm-line' : '.source-host .cm-content .cm-line').allTextContents()).join('\n');
async function waitNotes(page) { await page.getByRole('button', { name: '脚注', exact: true }).waitFor(); await page.waitForFunction(() => Array.from(document.querySelectorAll('button')).some(button => button.textContent === '脚注' && !button.disabled)); }
async function notes(page, type = '脚注') { await waitNotes(page); await click(page, type); await page.getByRole('dialog', { name: '脚注、尾注与题注' }).waitFor(); }
async function firstNote(page) { await page.getByRole('button', { name: /中文脚注.*2 处标记/ }).click(); }
async function sourceChanged(page, match, hosted) {
  await page.waitForFunction(({ match, selector }) => {
    const value = Array.from(document.querySelectorAll(selector)).map(node => node.textContent).join('\n');
    return (match.equals === undefined || value === match.equals) && (match.notEquals === undefined || value !== match.notEquals) && (match.includes ?? []).every(part => value.includes(part)) && (match.excludes ?? []).every(part => !value.includes(part));
  }, { match, selector: hosted ? '#source .cm-content .cm-line' : '.source-host .cm-content .cm-line' });
}
async function applyNoteAndUndo(page, original, isHosted) {
  await notes(page); await firstNote(page); assert.equal(await page.getByLabel('注释或题注内容', { exact: true }).inputValue(), '*中文脚注*');
  await page.getByLabel('注释或题注内容', { exact: true }).fill('*修改的脚注*\n\n第二段'); await click(page, '应用此注释内容'); await page.getByRole('dialog', { name: '脚注、尾注与题注' }).waitFor({ state: 'detached' });
  await sourceChanged(page, { includes: ['修改的脚注'] }, isHosted); const changed = await rawSource(page, isHosted); assert.notEqual(changed, original);
  const content = page.locator(isHosted ? '#source .cm-content' : '.source-host .cm-content'); await content.click(); await content.press('ControlOrMeta+z'); await sourceChanged(page, { excludes: ['修改的脚注'] }, isHosted); assert.equal(await rawSource(page, isHosted), original);
  await content.press('ControlOrMeta+Shift+Z'); await sourceChanged(page, { includes: ['修改的脚注'] }, isHosted); assert.equal(await rawSource(page, isHosted), changed);
  await notes(page); await page.getByRole('button', { name: /修改的脚注.*2 处标记/ }).click(); await page.getByLabel('注释或题注内容', { exact: true }).fill('取消的未应用注释'); await page.keyboard.press('Escape'); assert.equal(await rawSource(page, isHosted), changed);
  return changed;
}
async function numbering(page, isHosted) {
  await click(page, '编号控制'); await page.getByLabel('文档编号方案', { exact: true }).selectOption('chinese');
  const endpoint = isHosted ? '/api/hosted/editor/preview' : '/api/editor/preview', response = page.waitForResponse(reply => reply.url().endsWith(endpoint) && reply.status() === 200);
  await click(page, '预览文档编号'); const preview = await (await response).json(); assert.ok(preview.ok && preview.tree); assert.equal(await page.getByRole('dialog').getByText(/一、重复标题：无编号 →/).count(), 2);
  await click(page, '应用到文档（一次草稿修改）'); await sourceChanged(page, { includes: ['weftext:heading-numbering'] }, isHosted);
  const beforePersonal = await rawSource(page, isHosted); await page.getByLabel('此篇个人辅助序号显示', { exact: true }).selectOption('show'); await click(page, '关闭');
  assert.equal(await rawSource(page, isHosted), beforePersonal);
  return beforePersonal;
}
async function captionsAndReading(page, isHosted) {
  const original = await rawSource(page, isHosted), content = page.locator(isHosted ? '#source .cm-content' : '.source-host .cm-content');
  await notes(page, '题注与编号'); await page.getByRole('button', { name: /^表 ·.*原始表题/ }).click(); await page.getByLabel('注释或题注内容', { exact: true }).fill('*修改的表题*'); await click(page, '应用所选对象的题注与编号'); await waitNoteApplied(page); await sourceChanged(page, { includes: ['修改的表题'] }, isHosted);
  await content.click(); await content.press('ControlOrMeta+z'); await sourceChanged(page, { equals: original }, isHosted); assert.equal(await rawSource(page, isHosted), original);
  await notes(page, '题注与编号'); await page.getByRole('button', { name: /^公式本体编号 ·/ }).click(); await page.getByLabel('题注编号方式', { exact: true }).selectOption('pattern'); await page.getByLabel('原生编号方案', { exact: true }).fill('(1)'); await click(page, '应用公式本体编号'); await waitNoteApplied(page); await sourceChanged(page, { includes: ['#math.equation'] }, isHosted); const numbered = await rawSource(page, isHosted);
  await notes(page, '题注与编号'); await page.getByRole('button', { name: /^公式本体编号 ·/ }).click(); await page.getByLabel('题注编号方式', { exact: true }).selectOption('none'); await click(page, '应用公式本体编号'); await waitNoteApplied(page); await sourceChanged(page, { notEquals: numbered }, isHosted);
  await content.click(); await content.press('ControlOrMeta+z'); await sourceChanged(page, { equals: numbered }, isHosted); await content.press('ControlOrMeta+z'); await sourceChanged(page, { equals: original }, isHosted);
  await click(page, isHosted ? 'V05 阅读' : '阅读'); const markers = page.getByRole('button', { name: '查看脚注正文与全部标记', exact: true }); await markers.first().waitFor(); const second = await markers.nth(1).getAttribute('data-document-node-key'); await markers.first().click();
  const dialog = page.getByRole('dialog', { name: '脚注、尾注与题注' }); assert.equal(await dialog.locator('textarea,input,select').count(), 0, 'read view must not expose author controls or native source'); assert.equal((await dialog.locator('.document-notes-reading').innerText()).includes('*'), false); await click(page, '跳到正文第 2 处标记'); await dialog.waitFor({ state: 'detached' }); assert.equal(await page.evaluate(() => document.activeElement.getAttribute('data-document-node-key')), second);
  assert.equal(await page.locator('.personal-heading-number').count(), 2); await click(page, isHosted ? 'V01 源码' : '源码'); await sourceChanged(page, { equals: original }, isHosted); assert.equal(await rawSource(page, isHosted), original);
}
async function installAndInsertEndnote(page, isHosted) {
  await page.evaluate(() => {
    window.__typingTrace = [];
    const snapshot = event => { const s=window.getSelection(), content=document.querySelector('.cm-content'); window.__typingTrace.push({type:event.type,data:event.data,inputType:event.inputType,key:event.key,code:event.code,ctrl:event.ctrlKey,shift:event.shiftKey,prevented:event.defaultPrevented,anchor:s?.anchorNode?.textContent,anchorOffset:s?.anchorOffset,focus:s?.focusNode?.textContent,focusOffset:s?.focusOffset,text:content?.textContent}); };
    for(const type of ['beforeinput','input','selectionchange','keydown','keyup']) document.addEventListener(type,snapshot);
  });
  const content = page.locator(isHosted ? '#source .cm-content' : '.source-host .cm-content'); await content.click(); await content.press('ControlOrMeta+End');
  const beforeTyping = await rawSource(page, isHosted);
  await page.keyboard.type('\n\n未保存的尾注测试正文。'); await sourceChanged(page, { includes: ['未保存的尾注测试正文'] }, isHosted); const before = await rawSource(page, isHosted); assert.equal(before, beforeTyping + '\n\n未保存的尾注测试正文。', 'each real keyboard insertion must keep Chinese order and the original selection');
  await notes(page, '尾注'); await click(page, '准备本篇尾注模块资源'); await page.getByRole('dialog', { name: '脚注、尾注与题注' }).waitFor({ state: 'detached' });
  assert.equal(await rawSource(page, isHosted), before, 'resource preparation must not save or replace this unsaved draft');
  await notes(page, '尾注'); await page.getByLabel('注释或题注内容', { exact: true }).fill('可编辑尾注正文'); await click(page, '插入尾注'); await waitNoteApplied(page);
  await sourceChanged(page, { includes: ['#endnote', '可编辑尾注正文'] }, isHosted);
  const inserted = await rawSource(page, isHosted); await notes(page, '尾注'); await page.getByRole('button', { name: /可编辑尾注正文.*1 处标记/ }).click(); await page.getByLabel('注释或题注内容', { exact: true }).fill('同一未保存草稿的修改尾注'); await click(page, '应用此注释内容'); await page.getByRole('dialog').waitFor({ state: 'detached' }); await sourceChanged(page, { includes: ['同一未保存草稿的修改尾注'] }, isHosted);
  const edited = await rawSource(page, isHosted); await content.click(); await content.press('ControlOrMeta+z'); await sourceChanged(page, { includes: ['可编辑尾注正文'] }, isHosted); assert.equal(await rawSource(page, isHosted), inserted); await content.press('ControlOrMeta+z'); await sourceChanged(page, { excludes: ['#endnote'] }, isHosted); assert.equal(await rawSource(page, isHosted), before); await content.press('ControlOrMeta+Shift+Z'); await sourceChanged(page, { includes: ['可编辑尾注正文'] }, isHosted); await content.press('ControlOrMeta+Shift+Z'); await sourceChanged(page, { includes: ['同一未保存草稿的修改尾注'] }, isHosted); assert.equal(await rawSource(page, isHosted), edited);
}
async function numberingWithEndnote(page, isHosted) {
  await waitNotes(page);
  const before = await rawSource(page, isHosted), end = '// weftext:end-heading-numbering\n', at = before.indexOf(end), body = '同一未保存草稿的修改尾注';
  assert.ok(before.startsWith('// weftext:heading-numbering\n') && at >= 0);
  const note = /#box\[#endnote\(id:\s*"([^"]+)"\)\[同一未保存草稿的修改尾注\]\]/.exec(before); assert.ok(note, 'the same unsaved draft must contain its exact native endnote declaration');
  assert.ok(before.includes('#import "weftext-endnotes.typ": endnote, endnote-ref, endnotes') && before.includes('#endnotes()'));
  const canonical = '// weftext:heading-numbering\n#set heading(numbering: (..n) => if n.pos().len() <= 6 { numbering("1.1", ..n) })\n#counter(heading).update(0)\n// weftext:end-heading-numbering\n';
  const expected = canonical + before.slice(at + end.length);
  await click(page, '编号控制'); await page.getByLabel('文档编号方案', { exact: true }).selectOption('decimal'); await page.getByLabel('文档编号起始值', { exact: true }).fill('1'); await page.getByLabel('文档编号最大层级', { exact: true }).fill('6'); await page.getByLabel('文档编号排除标题标记', { exact: true }).fill('');
  const endpoint = isHosted ? '/api/hosted/editor/preview' : '/api/editor/preview', response = page.waitForResponse(reply => reply.url().endsWith(endpoint));
  await click(page, '预览文档编号'); const reply = await response, preview = await reply.json(); assert.equal(reply.status(), 200, JSON.stringify(preview)); assert.ok(preview.ok && preview.tree); assert.equal(preview.source, expected, 'the frozen preview must change only the canonical heading header'); assert.equal(await rawSource(page, isHosted), before, 'a preview must not apply or save the unsaved endnote draft');
  const all = item => [item, ...(item.children ?? []).flatMap(all)], nodes = all(preview.tree);
  assert.ok(nodes.some(item => item.attrs?.['data-weftext-role'] === 'endnote-reference' && item.attrs?.['data-weftext-note-id'] === note[1]), 'the real preview must retain this endnote identity');
  const nativeBody = nodes.find(item => item.attrs?.['data-weftext-role'] === 'endnote-body' && item.attrs?.['data-weftext-note-id'] === note[1]); assert.ok(nativeBody); assert.ok(all(nativeBody).map(item => item.text ?? '').join('').includes(body));
  await click(page, '应用到文档（一次草稿修改）'); await sourceChanged(page, { equals: expected }, isHosted); const after = await rawSource(page, isHosted); assert.equal(after, expected); assert.equal(after.split('// weftext:heading-numbering\n').length - 1, 1); assert.ok(after.includes(note[0])); await click(page, '关闭');
  const content = page.locator(isHosted ? '#source .cm-content' : '.source-host .cm-content'); await content.click(); await content.press('ControlOrMeta+z'); await sourceChanged(page, { equals: before }, isHosted); assert.equal(await rawSource(page, isHosted), before, 'one Undo must restore the entire original endnote draft exactly');
  await content.press('ControlOrMeta+Shift+Z'); await sourceChanged(page, { equals: after }, isHosted); await content.press('ControlOrMeta+z'); await sourceChanged(page, { equals: before }, isHosted); assert.equal(await rawSource(page, isHosted), before);
}
async function hostedAPI(route, body, actor = 'alice') { const response = await fetch(hosted.base + '/api/hosted/' + route, { method: body === undefined ? 'GET' : 'POST', headers: { Authorization: 'Bearer ' + fixture.tokens[actor], 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) }); const value = await response.json(); assert.equal(response.status, 200, JSON.stringify(value)); return value; }
try {
  execFileSync(python, [helper, repo, exe, out], { windowsHide: true }); fixture = JSON.parse(await readFile(path.join(out, 'fixture.json'), 'utf8'));
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_CHROMIUM ? { executablePath: process.env.WEFTEXT_CHROMIUM } : { channel: 'chrome' }) });
  local = await start(['--no-browser', '--data-dir', fixture.localData, '--open', fixture.localPath, '--root', fixture.localRoot], false);
  const page = await observedPage('local');
  await page.goto(await frontendFor(local,false) + '/#token=' + local.token); await waitNotes(page); const localOriginal = await rawSource(page); assert.equal(localOriginal, fixture.localSource);
  await applyNoteAndUndo(page, localOriginal, false); assert.equal(await savedSource(fixture.localPath), fixture.localSource); result.cases.push('local native note source editing, shared Undo/Redo and cancellation leave the saved original unchanged');
  await numbering(page, false); result.cases.push('local numbering preview compiles exact proposed source, preserves literal title prefixes and applies one author transaction; personal display does not edit source');
  await captionsAndReading(page, false); result.cases.push('local owned caption and consecutive equation numbering changes use shared Undo; V05 hides source controls, displays personal numbers and moves focus to the exact second native marker');
  await installAndInsertEndnote(page, false); assert.equal(await savedSource(fixture.localPath), fixture.localSource); await page.screenshot({ path: path.join(out, 'local-notes.png'), fullPage: true }); result.cases.push('local explicit fixed module resource installation and editable native endnote insertion do not save the author draft');
  await numberingWithEndnote(page, false); assert.equal(await savedSource(fixture.localPath), fixture.localSource); result.cases.push('local heading numbering over the same unsaved native endnote draft uses a real frozen preview, retains module/body/identity and restores the exact original draft with one Undo');
  await page.context().close(); await stop(local); local = undefined;
  hosted = await start(['--no-browser', '--data-dir', fixture.data, '--server-config', fixture.configPath], true);
  const hp = await observedPage('hosted');
  await hp.goto(await frontendFor(hosted,true)); await connectActor(hp, 'alice'); await hp.locator('#documents').getByRole('button', { name: 'Shared', exact: true }).click(); await waitNotes(hp);
  await applyNoteAndUndo(hp, fixture.source, true); assert.equal(await savedSource(path.join(fixture.root, 'Shared', 'Shared.typ')), fixture.source); await numbering(hp, true); await captionsAndReading(hp, true); await installAndInsertEndnote(hp, true); result.cases.push('hosted native notes, captions, equation numbering, read navigation, heading numbering preview and owned module resource import use actor-bound routes while retaining personal Draft/Undo and original saved bytes');
  await numberingWithEndnote(hp, true); assert.equal(await savedSource(path.join(fixture.root, 'Shared', 'Shared.typ')), fixture.source); result.cases.push('hosted heading numbering over the same unsaved native endnote draft proves the admitted module, compiles exact canonical output with stable endnote identity and restores all bytes with one Undo');
  await hp.screenshot({ path: path.join(out, 'hosted-notes.png'), fullPage: true });
  await notes(hp); await hp.getByRole('button', { name: /修改的脚注.*2 处标记/ }).click(); const pendingInput = hp.getByLabel('注释或题注内容', { exact: true }); await pendingInput.press('ControlOrMeta+Home'); await hp.keyboard.type('用户新输入甲'); await pendingInput.press('ControlOrMeta+End'); await hp.keyboard.type('用户新输入乙');
  const policy = await hostedAPI('admin/policy', undefined, 'admin'); policy.users.alice.grants.Shared = {}; await hostedAPI('admin/policy', { expectedGeneration: policy.generation, users: policy.users }, 'admin');
  await click(hp, '应用此注释内容'); await hp.waitForFunction(() => !document.querySelector('.document-notes-dialog') && !document.querySelector('#source .cm-content'));
  await click(hp, '取回未应用的注释输入'); assert.equal(await hp.getByLabel('保留的本次新输入', { exact: true }).inputValue(), '用户新输入甲\n用户新输入乙'); await click(hp, '恢复这份输入继续编辑'); assert.equal(await hp.getByLabel('保留的本次新输入', { exact: true }).isEditable(), true);
  await click(hp, '退出并清除本页缓存'); await connectActor(hp, 'bob'); assert.equal(await hp.locator('#note-input-recovery').isVisible(), false); assert.equal(await hp.locator('#note-inputs').textContent(), '');
  await click(hp, '退出并清除本页缓存'); await connectActor(hp, 'alice'); await click(hp, '取回未应用的注释输入'); assert.equal(await hp.getByLabel('保留的本次新输入', { exact: true }).inputValue(), '用户新输入甲\n用户新输入乙'); result.cases.push('revoked proposal recovery preserves only separated new author input, remains independently editable/copyable, and stays bound to its original actor across logout');
  await logoutDuringConnect(hp); result.cases.push('logout during a deliberately delayed account connection permits a new login and rejects the former account response without restoring its view or changing recovery ownership');
  assert.equal(await savedSource(path.join(fixture.root, 'Shared', 'Shared.typ')), fixture.source); result.cases.push('actual hosted read revocation rejects old note proposal, clears private dialog/source and never writes the saved original');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
  result.failure = error.stack || String(error);
  result.failurePages = [];
  for (const context of browser?.contexts() ?? []) for (const page of context.pages()) {
    const index = result.failurePages.length;
    result.failurePages.push({ url: page.url(), body: await page.locator('body').innerText().catch(() => 'unavailable') });
    await writeFile(path.join(out, `typing-${index}.json`), JSON.stringify(await page.evaluate(()=>window.__typingTrace ?? []),null,2));
    await page.screenshot({path:path.join(out, `failure-${index}.png`),fullPage:true}).catch(()=>{});
  }
  throw error;
}
finally {
  const cleanupErrors = [], failed = !!result.failure, finish = async work => { try { await work(); } catch (error) { cleanupErrors.push(String(error)); } };
  for (const context of contexts) await finish(() => context.close());
  await finish(() => browser?.close()); for(const server of viteServers) await finish(() => server.close()); await finish(() => stop(local)); await finish(() => stop(hosted));
  if (cleanupErrors.length) { result.cleanupErrors = cleanupErrors; result.completed = false; result.failure ??= 'Owned browser/backend cleanup failed'; }
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  for (const [scope, text] of Object.entries(stderrByScope)) await writeFile(path.join(out, `${scope}.stderr.log`), text);
  await writeFile(path.join(out, 'stderr-events.json'), JSON.stringify(stderrEvents, null, 2)); await writeFile(path.join(out, 'api-events.json'), JSON.stringify(apiEvents, null, 2));
  if (cleanupErrors.length && !failed) throw new Error(result.failure);
}
process.stdout.write(JSON.stringify(result, null, 2));
